use std::collections::BTreeSet;

use crate::{
    CcaEntry, CcaType, ControlType, NodeType, OperationType, TTGraph, build_paper_example_graph,
    build_synthetic_full_and_graph,
};

pub fn print_figure1() {
    println!("Figure 1 — TT Graph notation");
    println!("Node classes:");
    println!("  ACTIVITY — stores operations (variable, Read|Write|Kill)");
    println!("  CONTROL  — kind ∈ {{AND, XOR, LOOP}}");
    println!("  BLOCK    — body scope for a control construct");
    println!("Arc classes:");
    println!("  sequenceArc — sequential order inside a BLOCK");
    println!("  branchArc   — CONTROL → child BLOCK bodies");
    println!("  scopeArc    — ACTIVITY/CONTROL → enclosing BLOCK; BLOCK → parent CONTROL");
    println!();
    let graph = build_paper_example_graph();
    println!("Program 1 mini-example (Figure 2 subset):");
    for node_id in ["And1", "B1", "Act1", "Loop1", "B3", "Act2"] {
        print_node_summary(&graph, node_id);
    }
}

pub fn print_figure3() {
    println!("Figure 3 — SDE incremental detection flow");
    println!("Initiating package: {{ edit_activity, TNode, variable_name, operation_type }}");
    println!("Steps:");
    println!("  1. Append operation to TNode.operations");
    println!("  2. Walk upward via scopeArc until NULL");
    println!("  3. At each ancestor BLOCK, update d_OPN_set");
    println!("  4. When ancestor CONTROL is AND, run Algorithm 1 (MaintainAndAnalyzeCDF)");
    println!();
    println!("Program 2 walkthrough package:");
    println!("  {{ edit_activity=Act2, variable=v, operation=Write }}");
    let mut graph = build_paper_example_graph();
    let result = graph.insert_operation("Act2", "v", OperationType::Write);
    println!("  touched AND nodes: {:?}", result.touched_and_nodes);
    println!(
        "  updated BLOCK summaries: {:?}",
        result.summary_blocks_updated
    );
}

pub fn print_figure4() {
    println!("Figure 4 — upward scope traversal after Write(v) into Act2");
    let graph = build_paper_example_graph();
    let steps = trace_scope_ascent(&graph, "Act2");
    for (round, step) in steps.iter().enumerate() {
        println!(
            "Round {}: {} --scopeArc--> {} (type {:?})",
            round + 1,
            step.from,
            step.to,
            step.to_type
        );
        if step.to == "Loop1" {
            println!("  (first round stops at LOOP control — no AND analysis yet)");
        }
        if step.to == "And1" {
            println!("  (second round reaches AND — triggers CCA analysis on And1)");
        }
    }
}

pub fn print_figure5() {
    println!("Figure 5 — d_OPN_set queries on B2 drive CCA detection");
    let graph = build_paper_example_graph();
    println!("Insert Write(v) at Act2; sibling branch under And1 is B2.");
    println!("B2 d_OPN_set before insertion:");
    print_block_d_opn(&graph, "B2");
    println!();
    println!("Algorithm 1 queries sibling BLOCK B2 while updating B3/B1 chain:");
    for (variable, op) in [("v", OperationType::Write), ("v", OperationType::Read)] {
        if let Some(nodes) = graph.nodes["B2"].d_opn_set.get(&(variable.to_string(), op)) {
            println!("  d_OPN_set({variable}, {op:?}, B2) = {nodes:?}");
        }
    }
    println!("Matching Act3/Xor1/Act4/Act5 Read/Write/Kill tuples become Table 3 CCA entries.");
}

pub fn print_figure6(depth: usize) {
    let depth = depth.max(1);
    let case = build_synthetic_full_and_graph(depth, 16);
    println!("Figure 6 — full binary AND tree (depth={depth})");
    println!("  nodes={}", case.node_count);
    println!("  leaves={}", case.leaf_count);
    println!("  target activity={}", case.target_node_id);
    println!(
        "  matching Read(target) leaves={}",
        case.matching_leaf_count
    );
    println!("Tree spine (AND nodes):");
    for node in case.graph.nodes.values() {
        if node.node_type == NodeType::Control && node.control_type == Some(ControlType::And) {
            println!("  {} branches={:?}", node.node_id, node.branch_arc);
        }
    }
}

pub fn print_table1() {
    println!("Table 1 — CCA set definitions on AND node x");
    println!("  CCA_W&W(x) = {{ (v, m, n) | Write(v) in branch m, Write(v) in branch n, m ≠ n }}");
    println!("  CCA_W&R(x) = {{ (v, m, n) | Write(v) in branch m, Read(v) in branch n, m ≠ n }}");
    println!("  CCA_W&K(x) = {{ (v, m, n) | Write(v) in branch m, Kill(v) in branch n, m ≠ n }}");
    println!("  CCA_R&K(x) = {{ (v, m, n) | Read(v) in branch m, Kill(v) in branch n, m ≠ n }}");
}

pub fn print_table5() {
    println!("Table 5 — complexity comparison (paper binary-tree model)");
    println!("  With d_OPN_set:    maintenance O(log n), detection O(log n)  → total O(log n)");
    println!("  Without d_OPN_set: detection O(n) per AND level");
    println!("  Repo precise form: O(h * b + k)");
    println!("    h = ancestor scopes visited");
    println!("    b = sibling BLOCK branches at each AND");
    println!("    k = emitted CCA entries");
}

pub fn print_table6() {
    println!("Table 6 — incremental / concurrent detection vs prior work (paper summary)");
    println!("  Refs [1]–[8]:  incremental sequential dataflow — not concurrent CCA");
    println!("  Refs [9],[12]: concurrent detection — batch-oriented");
    println!("  Refs [10],[11]: related static analyses — not incremental TT Graph CDFA");
    println!("  Ours: incremental insertion + concurrent CCA via d_OPN_set summaries");
}

struct ScopeStep {
    from: String,
    to: String,
    to_type: NodeType,
}

fn trace_scope_ascent(graph: &TTGraph, start: &str) -> Vec<ScopeStep> {
    let mut steps = Vec::new();
    let mut current = start.to_string();
    while let Some(scope) = graph.nodes[&current].scope_arc.clone() {
        let to_type = graph.nodes[&scope].node_type;
        steps.push(ScopeStep {
            from: current,
            to: scope.clone(),
            to_type,
        });
        current = scope;
    }
    steps
}

fn print_node_summary(graph: &TTGraph, node_id: &str) {
    let node = &graph.nodes[node_id];
    println!(
        "  {node_id}: type={:?} control={:?} seq={:?} branches={:?} scope={:?}",
        node.node_type, node.control_type, node.sequence_arc, node.branch_arc, node.scope_arc
    );
}

fn print_block_d_opn(graph: &TTGraph, block_id: &str) {
    for (variable, op, nodes) in graph.sorted_d_opn_rows(block_id) {
        println!("  d_OPN_set({variable}, {op:?}, {block_id}) = {nodes:?}");
    }
}

pub fn figure5_cca_preview() -> BTreeSet<(CcaType, CcaEntry)> {
    let mut graph = build_paper_example_graph();
    let result = graph.insert_operation("Act2", "v", OperationType::Write);
    result.summary_entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn figure5_preview_has_seven_entries() {
        assert_eq!(figure5_cca_preview().len(), 7);
    }

    #[test]
    fn figure4_trace_reaches_and1() {
        let graph = build_paper_example_graph();
        let steps = trace_scope_ascent(&graph, "Act2");
        assert!(steps.iter().any(|step| step.to == "Loop1"));
        assert_eq!(steps.last().map(|step| step.to.as_str()), Some("And1"));
    }
}
