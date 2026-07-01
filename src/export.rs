use crate::{CcaType, ControlType, NodeType, Operation, OperationType, TTGraph};
use serde::Serialize;

pub fn graph_to_json(graph: &TTGraph) -> String {
    let mut output = serde_json::to_string_pretty(&GraphJson::from_graph(graph))
        .expect("graph export serialization should not fail");
    output.push('\n');
    output
}

#[derive(Serialize)]
struct GraphJson {
    schema_version: u8,
    nodes: Vec<NodeJson>,
    d_opn_set: Vec<DOpnRowJson>,
    cca_sets: Vec<CcaSetJson>,
}

impl GraphJson {
    fn from_graph(graph: &TTGraph) -> Self {
        Self {
            schema_version: 1,
            nodes: export_nodes(graph),
            d_opn_set: export_d_opn_rows(graph),
            cca_sets: export_cca_sets(graph),
        }
    }
}

#[derive(Serialize)]
struct NodeJson {
    id: String,
    node_type: &'static str,
    control_type: Option<&'static str>,
    operation_sequence: Vec<OperationJson>,
    sequence_arc: Option<String>,
    branch_arc: Vec<String>,
    scope_arc: Option<String>,
}

#[derive(Serialize)]
struct OperationJson {
    variable: String,
    operation: &'static str,
}

#[derive(Serialize)]
struct DOpnRowJson {
    block: String,
    variable: String,
    operation: &'static str,
    nodes: Vec<String>,
}

#[derive(Serialize)]
struct CcaSetJson {
    and_node: String,
    cca_type: &'static str,
    entries: Vec<CcaEntryJson>,
}

#[derive(Serialize)]
struct CcaEntryJson {
    variable: String,
    first_node: String,
    second_node: String,
}

fn export_nodes(graph: &TTGraph) -> Vec<NodeJson> {
    let mut node_ids: Vec<&String> = graph.nodes.keys().collect();
    node_ids.sort();

    node_ids
        .into_iter()
        .map(|node_id| {
            let node = &graph.nodes[node_id];
            NodeJson {
                id: node.node_id.clone(),
                node_type: node_type_name(node.node_type),
                control_type: node.control_type.map(control_type_name),
                operation_sequence: sorted_operations(&node.operations)
                    .into_iter()
                    .map(|operation| OperationJson {
                        variable: operation.variable,
                        operation: operation_type_name(operation.op),
                    })
                    .collect(),
                sequence_arc: node.sequence_arc.clone(),
                branch_arc: node.branch_arc.clone(),
                scope_arc: node.scope_arc.clone(),
            }
        })
        .collect()
}

fn export_d_opn_rows(graph: &TTGraph) -> Vec<DOpnRowJson> {
    let mut block_ids: Vec<&String> = graph
        .nodes
        .iter()
        .filter_map(|(node_id, node)| (node.node_type == NodeType::Block).then_some(node_id))
        .collect();
    block_ids.sort();

    let mut rows = Vec::new();
    for block_id in block_ids {
        for (variable, op, node_ids) in graph.sorted_d_opn_rows(block_id) {
            rows.push(DOpnRowJson {
                block: block_id.clone(),
                variable,
                operation: operation_type_name(op),
                nodes: node_ids,
            });
        }
    }
    rows
}

fn export_cca_sets(graph: &TTGraph) -> Vec<CcaSetJson> {
    let mut node_ids: Vec<&String> = graph.nodes.keys().collect();
    node_ids.sort();

    let mut rows = Vec::new();
    for node_id in node_ids {
        let node = &graph.nodes[node_id];
        let mut cca_types: Vec<CcaType> = node.cca_sets.keys().copied().collect();
        cca_types.sort();
        for cca_type in cca_types {
            let entries: Vec<CcaEntryJson> = node.cca_sets[&cca_type]
                .iter()
                .map(|entry| CcaEntryJson {
                    variable: entry.variable.clone(),
                    first_node: entry.first_node.clone(),
                    second_node: entry.second_node.clone(),
                })
                .collect();
            if !entries.is_empty() {
                rows.push(CcaSetJson {
                    and_node: node_id.clone(),
                    cca_type: cca_type_name(cca_type),
                    entries,
                });
            }
        }
    }
    rows
}

fn sorted_operations(operations: &std::collections::HashSet<Operation>) -> Vec<Operation> {
    let mut operations: Vec<Operation> = operations.iter().cloned().collect();
    operations.sort();
    operations
}

pub(crate) fn node_type_name(node_type: NodeType) -> &'static str {
    match node_type {
        NodeType::Activity => "Activity",
        NodeType::Control => "Control",
        NodeType::Block => "Block",
    }
}

pub(crate) fn control_type_name(control_type: ControlType) -> &'static str {
    match control_type {
        ControlType::And => "And",
        ControlType::Xor => "Xor",
        ControlType::Loop => "Loop",
    }
}

pub(crate) fn operation_type_name(operation_type: OperationType) -> &'static str {
    match operation_type {
        OperationType::Write => "Write",
        OperationType::Read => "Read",
        OperationType::Kill => "Kill",
    }
}

pub(crate) fn cca_type_name(cca_type: CcaType) -> &'static str {
    match cca_type {
        CcaType::WriteWrite => "WriteWrite",
        CcaType::WriteRead => "WriteRead",
        CcaType::WriteKill => "WriteKill",
        CcaType::ReadKill => "ReadKill",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        ControlType, Operation, OperationType, TTGraph, TTNode, build_paper_example_graph,
    };

    use super::graph_to_json;

    #[test]
    fn exports_nodes_d_opn_sets_and_cca_sets() {
        let mut graph = build_paper_example_graph();
        let result = graph.insert_operation("Act2", "v", OperationType::Write);
        assert!(result.matches_direct_scan());

        let json = graph_to_json(&graph);

        assert!(json.contains("\"nodes\""));
        assert!(json.contains("\"d_opn_set\""));
        assert!(json.contains("\"cca_sets\""));
        assert!(json.contains("\"id\": \"Act2\""));
        assert!(json.contains("\"block\": \"B1\""));
        assert!(json.contains("\"operation\": \"Write\""));
        assert!(json.contains("\"cca_type\": \"WriteRead\""));
        assert!(json.contains("\"first_node\": \"Act2\""));
    }

    #[test]
    fn tiny_graph_after_insertion_matches_golden_json_fixture() {
        let mut graph = tiny_graph();
        let result = graph.insert_operation("Act1", "x", OperationType::Write);
        assert!(result.matches_direct_scan());

        let expected: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/tiny_after_insertion.json")).unwrap();
        let actual: serde_json::Value = serde_json::from_str(&graph_to_json(&graph)).unwrap();
        assert_eq!(actual, expected);
    }

    fn tiny_graph() -> TTGraph {
        let mut nodes = HashMap::new();
        nodes.insert(
            "And1".to_string(),
            TTNode::control("And1", ControlType::And, None)
                .with_branch_arc(vec!["B1".to_string(), "B2".to_string()]),
        );
        nodes.insert(
            "B1".to_string(),
            TTNode::block("B1", "And1").with_sequence_arc("Act1"),
        );
        nodes.insert("Act1".to_string(), TTNode::activity("Act1", "B1"));
        nodes.insert(
            "B2".to_string(),
            TTNode::block("B2", "And1").with_sequence_arc("Act2"),
        );
        nodes.insert(
            "Act2".to_string(),
            TTNode::activity("Act2", "B2")
                .with_operations(vec![Operation::new("x", OperationType::Read)]),
        );

        TTGraph::new(nodes)
    }
}
