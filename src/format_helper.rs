use crate::{CcaEntry, CcaType, NodeType, OperationType, TTGraph};
use std::collections::BTreeSet;

/// Prints anomaly entries in a uniform way.
pub fn print_entries(entries: &BTreeSet<(CcaType, CcaEntry)>) {
    for (cca_type, entry) in entries {
        println!(
            "  {cca_type:?}: ({}, {}, {})",
            entry.variable, entry.first_node, entry.second_node
        );
    }
}

/// Prints Table 2 rows.
pub fn print_paper_table_2(graph: &TTGraph) {
    print_d_opn_rows(
        graph,
        &[
            ("B1", "v", OperationType::Read),
            ("B1", "v", OperationType::Write),
            ("B1", "i", OperationType::Read),
            ("B1", "i", OperationType::Write),
            ("B2", "v", OperationType::Read),
            ("B2", "v", OperationType::Write),
            ("B2", "v", OperationType::Kill),
            ("B2", "i", OperationType::Write),
            ("B3", "v", OperationType::Read),
            ("B3", "i", OperationType::Read),
            ("B3", "i", OperationType::Write),
            ("B4", "v", OperationType::Read),
            ("B4", "i", OperationType::Write),
            ("B4", "v", OperationType::Kill),
            ("B5", "v", OperationType::Read),
            ("B5", "v", OperationType::Kill),
        ],
    );
}

/// Prints Table 4 rows.
pub fn print_paper_table_4(graph: &TTGraph) {
    print_d_opn_rows(
        graph,
        &[
            ("B1", "v", OperationType::Read),
            ("B1", "v", OperationType::Write),
            ("B1", "i", OperationType::Read),
            ("B1", "i", OperationType::Write),
        ],
    );
}

/// Helper to print selected d_OPN_set rows.
pub fn print_d_opn_rows(graph: &TTGraph, rows: &[(&str, &str, OperationType)]) {
    let mut current_block = "";
    for (block_id, variable, op) in rows {
        if *block_id != current_block {
            current_block = block_id;
            println!("{block_id}:");
        }

        let key = (variable.to_string(), *op);
        let mut node_ids: Vec<String> = graph.nodes[*block_id]
            .d_opn_set
            .get(&key)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();
        node_ids.sort();
        println!("  d_OPN_set({variable}, {op:?}, {block_id}) = {node_ids:?}");
    }
}

/// Prints all d_OPN_set rows.
pub fn print_all_d_opn_rows(graph: &TTGraph) {
    let mut block_ids: Vec<String> = graph
        .nodes
        .values()
        .filter(|node| node.node_type == NodeType::Block)
        .map(|node| node.node_id.clone())
        .collect();
    block_ids.sort();

    for block_id in block_ids {
        println!("{block_id}:");
        for (variable, op, node_ids) in graph.sorted_d_opn_rows(&block_id) {
            println!("  d_OPN_set({variable}, {op:?}, {block_id}) = {node_ids:?}");
        }
    }
}

/// Identifies if a graph is the paper's Program 1 example graph.
pub fn is_paper_program_1_graph(graph: &TTGraph) -> bool {
    [
        "And1", "B1", "B2", "B3", "B4", "B5", "Act1", "Act2", "Act3", "Act4", "Act5", "Loop1",
        "Xor1",
    ]
    .iter()
    .all(|node_id| graph.nodes.contains_key(*node_id))
}

/// Counts anomalies per CcaType on an AND node.
pub fn anomaly_counts(graph: &TTGraph, and_id: &str) -> Vec<(CcaType, usize)> {
    let mut rows: Vec<(CcaType, usize)> = graph.nodes[and_id]
        .cca_sets
        .iter()
        .map(|(cca_type, entries)| (*cca_type, entries.len()))
        .collect();
    rows.sort_by_key(|(cca_type, _)| *cca_type);
    rows
}

/// Prints CCA counts.
pub fn print_counts(counts: &[(CcaType, usize)]) {
    for (cca_type, count) in counts {
        println!("  {cca_type:?}: {count}");
    }
}

/// Prints full CCA entries on an AND node.
pub fn print_cca_sets(graph: &TTGraph, and_id: &str) {
    let mut rows: Vec<(CcaType, Vec<String>)> = graph.nodes[and_id]
        .cca_sets
        .iter()
        .map(|(cca_type, entries)| {
            (
                *cca_type,
                entries
                    .iter()
                    .map(|entry| {
                        format!(
                            "({}, {}, {})",
                            entry.variable, entry.first_node, entry.second_node
                        )
                    })
                    .collect(),
            )
        })
        .collect();
    rows.sort_by_key(|(cca_type, _)| *cca_type);
    for (cca_type, entries) in rows {
        println!("  {cca_type:?}: {entries:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_paper_example_graph, build_synthetic_full_and_graph};

    #[test]
    fn classifies_only_the_paper_program_1_graph_as_paper_output() {
        let paper_graph = build_paper_example_graph();
        let synthetic_graph = build_synthetic_full_and_graph(1, 1).graph;

        assert!(is_paper_program_1_graph(&paper_graph));
        assert!(!is_paper_program_1_graph(&synthetic_graph));
    }
}
