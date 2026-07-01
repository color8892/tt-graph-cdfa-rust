use std::collections::HashMap;

use serde::Serialize;

use crate::export::{cca_type_name, control_type_name, node_type_name};
use crate::{CcaEntry, CcaType, NodeType, TTGraph};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl SourceLocation {
    pub fn new(file: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            file: file.into(),
            line: line.max(1),
            column: column.max(1),
        }
    }
}

pub fn graph_diagnostics_to_json(
    graph: &TTGraph,
    source_path: &str,
    language: &str,
    source_locations: &HashMap<String, SourceLocation>,
) -> String {
    serialize_pretty(&DiagnosticsReportJson::from_graph(
        graph,
        source_path,
        language,
        source_locations,
    ))
}

pub fn error_diagnostics_to_json(source_path: &str, language: &str, message: &str) -> String {
    serialize_pretty(&DiagnosticsReportJson::error(
        source_path,
        language,
        message,
    ))
}

fn serialize_pretty<T: Serialize>(value: &T) -> String {
    let mut output =
        serde_json::to_string_pretty(value).expect("diagnostics serialization should not fail");
    output.push('\n');
    output
}

#[derive(Serialize)]
struct DiagnosticsReportJson {
    schema_version: u8,
    source: SourceJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    graph: GraphJson,
    diagnostics: Vec<DiagnosticJson>,
}

impl DiagnosticsReportJson {
    fn from_graph(
        graph: &TTGraph,
        source_path: &str,
        language: &str,
        source_locations: &HashMap<String, SourceLocation>,
    ) -> Self {
        Self {
            schema_version: 1,
            source: SourceJson::new(source_path, language),
            error: None,
            graph: GraphJson {
                nodes: graph_nodes(graph, source_locations),
                edges: graph_edges(graph),
            },
            diagnostics: diagnostics(graph, source_path, source_locations),
        }
    }

    fn error(source_path: &str, language: &str, message: &str) -> Self {
        Self {
            schema_version: 1,
            source: SourceJson::new(source_path, language),
            error: Some(message.to_string()),
            graph: GraphJson {
                nodes: Vec::new(),
                edges: Vec::new(),
            },
            diagnostics: Vec::new(),
        }
    }
}

#[derive(Serialize)]
struct SourceJson {
    path: String,
    language: String,
}

impl SourceJson {
    fn new(path: &str, language: &str) -> Self {
        Self {
            path: path.to_string(),
            language: language.to_string(),
        }
    }
}

#[derive(Serialize)]
struct GraphJson {
    nodes: Vec<GraphNodeJson>,
    edges: Vec<GraphEdgeJson>,
}

#[derive(Serialize)]
struct GraphNodeJson {
    id: String,
    node_type: &'static str,
    control_type: Option<&'static str>,
    label: String,
    source: Option<SourceLocation>,
}

#[derive(Serialize)]
struct GraphEdgeJson {
    from: String,
    to: String,
    #[serde(rename = "type")]
    edge_type: String,
}

#[derive(Serialize)]
struct DiagnosticJson {
    severity: &'static str,
    cca_type: &'static str,
    variable: String,
    message: String,
    first: DiagnosticEndpointJson,
    second: DiagnosticEndpointJson,
}

#[derive(Serialize)]
struct DiagnosticEndpointJson {
    node: String,
    #[serde(flatten)]
    location: SourceLocation,
}

fn graph_nodes(
    graph: &TTGraph,
    source_locations: &HashMap<String, SourceLocation>,
) -> Vec<GraphNodeJson> {
    let mut node_ids: Vec<&String> = graph.nodes.keys().collect();
    node_ids.sort();

    node_ids
        .into_iter()
        .map(|node_id| {
            let node = &graph.nodes[node_id];
            GraphNodeJson {
                id: node.node_id.clone(),
                node_type: node_type_name(node.node_type),
                control_type: node.control_type.map(control_type_name),
                label: node.node_id.clone(),
                source: source_locations.get(&node.node_id).cloned(),
            }
        })
        .collect()
}

fn graph_edges(graph: &TTGraph) -> Vec<GraphEdgeJson> {
    let mut edges = Vec::new();
    let mut node_ids: Vec<&String> = graph.nodes.keys().collect();
    node_ids.sort();

    for node_id in node_ids {
        let node = &graph.nodes[node_id];
        if let Some(sequence_arc) = &node.sequence_arc {
            edges.push(GraphEdgeJson {
                from: node.node_id.clone(),
                to: sequence_arc.clone(),
                edge_type: "sequence".to_string(),
            });
        }
        for branch_arc in &node.branch_arc {
            edges.push(GraphEdgeJson {
                from: node.node_id.clone(),
                to: branch_arc.clone(),
                edge_type: "branch".to_string(),
            });
        }
        if let Some(scope_arc) = &node.scope_arc {
            edges.push(GraphEdgeJson {
                from: node.node_id.clone(),
                to: scope_arc.clone(),
                edge_type: "scope".to_string(),
            });
        }
        let mut cca_types: Vec<CcaType> = node.cca_sets.keys().copied().collect();
        cca_types.sort();
        for cca_type in cca_types {
            for entry in &node.cca_sets[&cca_type] {
                edges.push(GraphEdgeJson {
                    from: entry.first_node.clone(),
                    to: entry.second_node.clone(),
                    edge_type: format!("cca:{}", cca_type_name(cca_type)),
                });
            }
        }
    }

    edges
}

fn diagnostics(
    graph: &TTGraph,
    source_path: &str,
    source_locations: &HashMap<String, SourceLocation>,
) -> Vec<DiagnosticJson> {
    let mut rows: Vec<(CcaType, CcaEntry)> = Vec::new();
    let mut node_ids: Vec<&String> = graph.nodes.keys().collect();
    node_ids.sort();
    for node_id in node_ids {
        let node = &graph.nodes[node_id];
        if node.node_type != NodeType::Control {
            continue;
        }
        let mut cca_types: Vec<CcaType> = node.cca_sets.keys().copied().collect();
        cca_types.sort();
        for cca_type in cca_types {
            for entry in &node.cca_sets[&cca_type] {
                rows.push((cca_type, entry.clone()));
            }
        }
    }

    rows.into_iter()
        .map(|(cca_type, entry)| {
            let cca_name = cca_type_name(cca_type);
            DiagnosticJson {
                severity: "warning",
                cca_type: cca_name,
                variable: entry.variable.clone(),
                message: format!(
                    "Concurrent dataflow anomaly {cca_name} on variable {}",
                    entry.variable
                ),
                first: diagnostic_endpoint(&entry.first_node, source_path, source_locations),
                second: diagnostic_endpoint(&entry.second_node, source_path, source_locations),
            }
        })
        .collect()
}

fn diagnostic_endpoint(
    node_id: &str,
    source_path: &str,
    source_locations: &HashMap<String, SourceLocation>,
) -> DiagnosticEndpointJson {
    DiagnosticEndpointJson {
        node: node_id.to_string(),
        location: source_locations
            .get(node_id)
            .cloned()
            .unwrap_or_else(|| SourceLocation::new(source_path, 1, 1)),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::diagnostics::{
        SourceLocation, error_diagnostics_to_json, graph_diagnostics_to_json,
    };
    use crate::{OperationType, build_paper_example_graph};

    #[test]
    fn diagnostics_json_contains_schema_graph_and_conflicts() {
        let mut graph = build_paper_example_graph();
        graph.insert_operation("Act2", "v", OperationType::Write);
        let mut locations = HashMap::new();
        locations.insert(
            "Act2".to_string(),
            SourceLocation::new("examples/program1.cpp", 20, 3),
        );

        let json = graph_diagnostics_to_json(&graph, "examples/program1.cpp", "cpp", &locations);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["source"]["path"], "examples/program1.cpp");
        assert_eq!(parsed["source"]["language"], "cpp");
        assert!(parsed["graph"]["nodes"].as_array().unwrap().len() >= 10);
        assert!(parsed["graph"]["edges"].as_array().unwrap().len() >= 10);

        let diagnostics = parsed["diagnostics"].as_array().unwrap();
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic["cca_type"] == "WriteRead"
                && diagnostic["variable"] == "v"
                && diagnostic["first"]["node"] == "Act2"
                && diagnostic["first"]["line"] == 20
                && diagnostic["first"]["column"] == 3
        }));
    }

    #[test]
    fn diagnostics_json_uses_location_fallback() {
        let mut graph = build_paper_example_graph();
        graph.insert_operation("Act2", "v", OperationType::Write);

        let json =
            graph_diagnostics_to_json(&graph, "examples/program1.cpp", "cpp", &HashMap::new());
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let diagnostics = parsed["diagnostics"].as_array().unwrap();

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic["first"]["file"] == "examples/program1.cpp"
                && diagnostic["first"]["line"] == 1
                && diagnostic["first"]["column"] == 1
        }));
    }

    #[test]
    fn error_diagnostics_json_contains_source_and_message() {
        let json = error_diagnostics_to_json(
            "examples/missing.cpp",
            "cpp",
            "failed to parse examples/missing.cpp",
        );
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["source"]["path"], "examples/missing.cpp");
        assert_eq!(parsed["source"]["language"], "cpp");
        assert_eq!(parsed["error"], "failed to parse examples/missing.cpp");
        assert!(parsed["graph"]["nodes"].as_array().unwrap().is_empty());
        assert!(parsed["graph"]["edges"].as_array().unwrap().is_empty());
        assert!(parsed["diagnostics"].as_array().unwrap().is_empty());
    }
}
