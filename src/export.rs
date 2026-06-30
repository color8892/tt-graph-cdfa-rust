use crate::{CcaEntry, CcaType, ControlType, NodeType, Operation, OperationType, TTGraph};

pub fn graph_to_json(graph: &TTGraph) -> String {
    let mut output = String::new();
    output.push_str("{\n");
    output.push_str("  \"schema_version\": 1,\n");
    output.push_str("  \"nodes\": ");
    push_nodes_json(&mut output, graph);
    output.push_str(",\n");
    output.push_str("  \"d_opn_set\": ");
    push_d_opn_json(&mut output, graph);
    output.push_str(",\n");
    output.push_str("  \"cca_sets\": ");
    push_cca_json(&mut output, graph);
    output.push_str("\n}\n");
    output
}

fn push_nodes_json(output: &mut String, graph: &TTGraph) {
    output.push_str("[\n");
    let mut node_ids: Vec<&String> = graph.nodes.keys().collect();
    node_ids.sort();

    for (index, node_id) in node_ids.iter().enumerate() {
        let node = &graph.nodes[*node_id];
        output.push_str("    {\n");
        push_json_field(output, 6, "id", &json_string(&node.node_id), true);
        push_json_field(
            output,
            6,
            "node_type",
            &json_string(node_type_name(node.node_type)),
            true,
        );
        push_json_field(
            output,
            6,
            "control_type",
            &json_optional_string(node.control_type.map(control_type_name)),
            true,
        );
        output.push_str("      \"operation_sequence\": ");
        push_operations_json(output, &sorted_operations(&node.operations));
        output.push_str(",\n");
        push_json_field(
            output,
            6,
            "sequence_arc",
            &json_optional_string(node.sequence_arc.as_deref()),
            true,
        );
        output.push_str("      \"branch_arc\": ");
        push_string_array(output, &node.branch_arc);
        output.push_str(",\n");
        push_json_field(
            output,
            6,
            "scope_arc",
            &json_optional_string(node.scope_arc.as_deref()),
            false,
        );
        output.push_str("    }");
        if index + 1 != node_ids.len() {
            output.push(',');
        }
        output.push('\n');
    }

    output.push_str("  ]");
}

fn push_d_opn_json(output: &mut String, graph: &TTGraph) {
    output.push_str("[\n");
    let mut rows = Vec::new();
    let mut block_ids: Vec<&String> = graph
        .nodes
        .iter()
        .filter_map(|(node_id, node)| (node.node_type == NodeType::Block).then_some(node_id))
        .collect();
    block_ids.sort();

    for block_id in block_ids {
        for (variable, op, node_ids) in graph.sorted_d_opn_rows(block_id) {
            rows.push((block_id.clone(), variable, op, node_ids));
        }
    }

    for (index, (block_id, variable, op, node_ids)) in rows.iter().enumerate() {
        output.push_str("    {\n");
        push_json_field(output, 6, "block", &json_string(block_id), true);
        push_json_field(output, 6, "variable", &json_string(variable), true);
        push_json_field(
            output,
            6,
            "operation",
            &json_string(operation_type_name(*op)),
            true,
        );
        output.push_str("      \"nodes\": ");
        push_string_array(output, node_ids);
        output.push('\n');
        output.push_str("    }");
        if index + 1 != rows.len() {
            output.push(',');
        }
        output.push('\n');
    }

    output.push_str("  ]");
}

fn push_cca_json(output: &mut String, graph: &TTGraph) {
    output.push_str("[\n");
    let mut rows: Vec<(&String, CcaType, Vec<CcaEntry>)> = Vec::new();
    let mut node_ids: Vec<&String> = graph.nodes.keys().collect();
    node_ids.sort();

    for node_id in node_ids {
        let node = &graph.nodes[node_id];
        let mut cca_types: Vec<CcaType> = node.cca_sets.keys().copied().collect();
        cca_types.sort();
        for cca_type in cca_types {
            let entries: Vec<CcaEntry> = node.cca_sets[&cca_type].iter().cloned().collect();
            if !entries.is_empty() {
                rows.push((node_id, cca_type, entries));
            }
        }
    }

    for (index, (and_node, cca_type, entries)) in rows.iter().enumerate() {
        output.push_str("    {\n");
        push_json_field(output, 6, "and_node", &json_string(and_node), true);
        push_json_field(
            output,
            6,
            "cca_type",
            &json_string(cca_type_name(*cca_type)),
            true,
        );
        output.push_str("      \"entries\": ");
        push_cca_entries_json(output, entries);
        output.push('\n');
        output.push_str("    }");
        if index + 1 != rows.len() {
            output.push(',');
        }
        output.push('\n');
    }

    output.push_str("  ]");
}

fn push_operations_json(output: &mut String, operations: &[Operation]) {
    output.push('[');
    for (index, operation) in operations.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        output.push_str("{\"variable\": ");
        output.push_str(&json_string(&operation.variable));
        output.push_str(", \"operation\": ");
        output.push_str(&json_string(operation_type_name(operation.op)));
        output.push('}');
    }
    output.push(']');
}

fn push_cca_entries_json(output: &mut String, entries: &[CcaEntry]) {
    output.push('[');
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        output.push_str("{\"variable\": ");
        output.push_str(&json_string(&entry.variable));
        output.push_str(", \"first_node\": ");
        output.push_str(&json_string(&entry.first_node));
        output.push_str(", \"second_node\": ");
        output.push_str(&json_string(&entry.second_node));
        output.push('}');
    }
    output.push(']');
}

fn push_string_array(output: &mut String, values: &[String]) {
    output.push('[');
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        output.push_str(&json_string(value));
    }
    output.push(']');
}

fn push_json_field(output: &mut String, indent: usize, name: &str, value: &str, comma: bool) {
    output.push_str(&" ".repeat(indent));
    output.push('"');
    output.push_str(name);
    output.push_str("\": ");
    output.push_str(value);
    if comma {
        output.push(',');
    }
    output.push('\n');
}

fn sorted_operations(operations: &std::collections::HashSet<Operation>) -> Vec<Operation> {
    let mut operations: Vec<Operation> = operations.iter().cloned().collect();
    operations.sort();
    operations
}

fn json_optional_string(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_string())
}

fn json_string(value: &str) -> String {
    let mut escaped = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            value if value.is_control() => escaped.push_str(&format!("\\u{:04x}", value as u32)),
            value => escaped.push(value),
        }
    }
    escaped.push('"');
    escaped
}

fn node_type_name(node_type: NodeType) -> &'static str {
    match node_type {
        NodeType::Activity => "Activity",
        NodeType::Control => "Control",
        NodeType::Block => "Block",
    }
}

fn control_type_name(control_type: ControlType) -> &'static str {
    match control_type {
        ControlType::And => "And",
        ControlType::Xor => "Xor",
        ControlType::Loop => "Loop",
    }
}

fn operation_type_name(operation_type: OperationType) -> &'static str {
    match operation_type {
        OperationType::Write => "Write",
        OperationType::Read => "Read",
        OperationType::Kill => "Kill",
    }
}

fn cca_type_name(cca_type: CcaType) -> &'static str {
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
    fn escapes_json_strings() {
        let json = super::json_string("quote\" slash\\ newline\n");

        assert_eq!(json, "\"quote\\\" slash\\\\ newline\\n\"");
    }

    #[test]
    fn tiny_graph_after_insertion_matches_golden_json_fixture() {
        let mut graph = tiny_graph();
        let result = graph.insert_operation("Act1", "x", OperationType::Write);
        assert!(result.matches_direct_scan());

        let expected = include_str!("../fixtures/tiny_after_insertion.json").replace("\r\n", "\n");
        assert_eq!(graph_to_json(&graph), expected);
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
