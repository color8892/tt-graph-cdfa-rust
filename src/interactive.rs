use std::io::{self, Write};

use crate::{
    ControlType, DeletionResult, DetectionResult, NodeType, Operation, OperationType, TTGraph,
    build_paper_example_graph, parse_operation_type,
};

const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const RESET: &str = "\x1b[0m";

pub fn run_interactive() {
    let mut graph = build_paper_example_graph();
    let mut last_action_info = None;

    loop {
        print!("\x1b[2J\x1b[H");
        io::stdout().flush().unwrap();

        println!("{BOLD}{CYAN}=== TT Graph CDFA - SDE Simulator ==={RESET}");
        println!(
            "{DIM}Simulate incremental variable operations edits and trace concurrency anomalies live.{RESET}\n"
        );

        println!("{BOLD}1. Current TT Graph Structure:{RESET}");
        print!("{}", render_graph_ascii(&graph));
        println!();

        println!("{BOLD}2. d_OPN_set Summaries (per BLOCK):{RESET}");
        for block_id in &["B1", "B2", "B3", "B4", "B5"] {
            let rows = graph.sorted_d_opn_rows(block_id);
            if rows.is_empty() {
                println!("  {CYAN}{block_id}{RESET}: {DIM}<empty>{RESET}");
            } else {
                println!("  {CYAN}{block_id}{RESET}:");
                for (var, op, nodes) in rows {
                    let op_color = operation_color(op);
                    println!(
                        "    d_OPN_set({var}, {op_color}{op:?}{RESET}) = {GREEN}{nodes:?}{RESET}"
                    );
                }
            }
        }
        println!();

        println!("{BOLD}3. Stored Anomalies (on And1):{RESET}");
        if let Some(and_node) = graph.nodes.get("And1") {
            let mut cca_types: Vec<crate::CcaType> = and_node.cca_sets.keys().copied().collect();
            cca_types.sort();
            for cca_type in cca_types {
                let entries = &and_node.cca_sets[&cca_type];
                let mut rendered = Vec::new();
                for entry in entries {
                    rendered.push(format!(
                        "({}, {}, {})",
                        entry.variable, entry.first_node, entry.second_node
                    ));
                }
                rendered.sort();
                println!(
                    "  {RED}{BOLD}{cca_type:?}{RESET}: {RED}{:?}{RESET}",
                    rendered
                );
            }
        } else {
            println!("  {DIM}<No And1 control node in graph>{RESET}");
        }
        println!();

        println!("{BOLD}4. Last Action Result:{RESET}");
        if let Some(ref info) = last_action_info {
            println!("{info}");
        } else {
            println!(
                "  {DIM}No action performed yet. Choose an option below to modify the graph.{RESET}"
            );
        }
        println!();

        println!("{BOLD}Shortcuts & Commands:{RESET}");
        println!(
            "  {GREEN}[1]{RESET} Insert {YELLOW}Write(v){RESET} into {GREEN}Act2{RESET}  (Paper Program 2 example)"
        );
        println!(
            "  {GREEN}[2]{RESET} Insert {GREEN}Read(i){RESET} into {GREEN}Act4{RESET}   (Extra edge case)"
        );
        println!("  {GREEN}[3]{RESET} Delete {YELLOW}Write(v){RESET} from {GREEN}Act2{RESET}");
        println!("  {GREEN}[r]{RESET} Reset graph to initial state");
        println!("  {GREEN}[q]{RESET} Quit simulator");
        println!("  {BOLD}Custom command syntax:{RESET}");
        println!("    {GREEN}i <node_id> <var> <read|write|kill>{RESET} - Insert custom operation");
        println!("    {GREEN}d <node_id> <var> <read|write|kill>{RESET} - Delete custom operation");
        println!();
        print!("{BOLD}sde-sim > {RESET}");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        if io::stdin().read_line(&mut choice).is_err() {
            break;
        }
        let choice = choice.trim();
        if choice.is_empty() {
            continue;
        }

        if choice == "q" || choice == "quit" {
            break;
        }

        if choice == "r" || choice == "reset" {
            graph = build_paper_example_graph();
            last_action_info = Some(format!(
                "  {GREEN}Graph reset to initial Program 1 state.{RESET}"
            ));
            continue;
        }

        if choice == "1" {
            let res = graph.insert_operation("Act2", "v", OperationType::Write);
            last_action_info = Some(format_insert_result(
                "Act2",
                "v",
                OperationType::Write,
                &res,
            ));
            continue;
        }

        if choice == "2" {
            let res = graph.insert_operation("Act4", "i", OperationType::Read);
            last_action_info = Some(format_insert_result("Act4", "i", OperationType::Read, &res));
            continue;
        }

        if choice == "3" {
            let res = graph.delete_operation("Act2", "v", OperationType::Write);
            last_action_info = Some(format_delete_result(
                "Act2",
                "v",
                OperationType::Write,
                &res,
            ));
            continue;
        }

        let tokens: Vec<&str> = choice.split_whitespace().collect();
        if tokens.len() == 4 && (tokens[0] == "i" || tokens[0] == "insert") {
            last_action_info = Some(run_custom_insert(&mut graph, &tokens));
            continue;
        }

        if tokens.len() == 4 && (tokens[0] == "d" || tokens[0] == "delete") {
            last_action_info = Some(run_custom_delete(&mut graph, &tokens));
            continue;
        }

        last_action_info = Some(format!(
            "  {RED}Error: Unknown command `{choice}`. Check help/shortcuts.{RESET}"
        ));
    }
}

fn run_custom_insert(graph: &mut TTGraph, tokens: &[&str]) -> String {
    let node_id = tokens[1];
    let variable = tokens[2];
    if !graph.nodes.contains_key(node_id) {
        return format!("  {RED}Error: Node `{node_id}` not found in graph.{RESET}");
    }
    if let Some(op) = parse_operation_type(tokens[3]) {
        let res = graph.insert_operation(node_id, variable, op);
        format_insert_result(node_id, variable, op, &res)
    } else {
        format!(
            "  {RED}Error: Unknown operation type `{}`. Use read, write, or kill.{RESET}",
            tokens[3]
        )
    }
}

fn run_custom_delete(graph: &mut TTGraph, tokens: &[&str]) -> String {
    let node_id = tokens[1];
    let variable = tokens[2];
    if !graph.nodes.contains_key(node_id) {
        return format!("  {RED}Error: Node `{node_id}` not found in graph.{RESET}");
    }
    if let Some(op) = parse_operation_type(tokens[3]) {
        let res = graph.delete_operation(node_id, variable, op);
        format_delete_result(node_id, variable, op, &res)
    } else {
        format!(
            "  {RED}Error: Unknown operation type `{}`. Use read, write, or kill.{RESET}",
            tokens[3]
        )
    }
}

fn operation_color(op: OperationType) -> &'static str {
    match op {
        OperationType::Read => GREEN,
        OperationType::Write => YELLOW,
        OperationType::Kill => RED,
    }
}

fn format_ops(operations: &std::collections::HashSet<Operation>) -> String {
    if operations.is_empty() {
        return String::new();
    }
    let mut ops: Vec<String> = operations
        .iter()
        .map(|op| {
            let color = operation_color(op.op);
            format!("{color}{:?}({}){RESET}", op.op, op.variable)
        })
        .collect();
    ops.sort();
    format!(
        "{DIM}[{RESET}{}{DIM}]{RESET}",
        ops.join(&format!("{DIM}, {RESET}"))
    )
}

fn render_graph_ascii(graph: &TTGraph) -> String {
    let mut output = String::new();
    let mut roots: Vec<String> = graph
        .nodes
        .values()
        .filter(|node| node.scope_arc.is_none())
        .map(|node| node.node_id.clone())
        .collect();
    roots.sort();

    for (i, root_id) in roots.iter().enumerate() {
        let is_last = i == roots.len() - 1;
        render_node(graph, root_id, "", is_last, &mut output);
    }
    output
}

fn render_node(graph: &TTGraph, node_id: &str, prefix: &str, is_last: bool, output: &mut String) {
    let node = &graph.nodes[node_id];
    let marker = if is_last { "`-- " } else { "|-- " };
    let child_prefix = if is_last { "    " } else { "|   " };

    match node.node_type {
        NodeType::Activity => {
            let ops_str = format_ops(&node.operations);
            output.push_str(&format!(
                "{prefix}{DIM}{marker}{RESET}{GREEN}{node_id}{RESET} {ops_str}\n"
            ));
        }
        NodeType::Control => {
            let kind_str = match node.control_type {
                Some(ControlType::And) => format!("{YELLOW}[AND]{RESET}"),
                Some(ControlType::Xor) => format!("{YELLOW}[XOR]{RESET}"),
                Some(ControlType::Loop) => format!("{YELLOW}[LOOP]{RESET}"),
                None => String::new(),
            };
            let ops_str = format_ops(&node.operations);
            output.push_str(&format!(
                "{prefix}{DIM}{marker}{RESET}{YELLOW}{node_id}{RESET} {kind_str} {ops_str}\n"
            ));

            let num_branches = node.branch_arc.len();
            for (idx, branch_id) in node.branch_arc.iter().enumerate() {
                let branch_is_last = idx == num_branches - 1;
                let next_prefix = format!("{prefix}{DIM}{child_prefix}{RESET}");
                render_node(graph, branch_id, &next_prefix, branch_is_last, output);
            }
        }
        NodeType::Block => {
            output.push_str(&format!(
                "{prefix}{DIM}{marker}{RESET}{CYAN}{node_id}{RESET}\n"
            ));
            let mut seq = node.sequence_arc.clone();
            let mut seq_nodes = Vec::new();
            while let Some(seq_id) = seq {
                seq_nodes.push(seq_id.clone());
                seq = graph.nodes[&seq_id].sequence_arc.clone();
            }

            let num_seq = seq_nodes.len();
            for (idx, seq_id) in seq_nodes.iter().enumerate() {
                let seq_is_last = idx == num_seq - 1;
                let next_prefix = format!("{prefix}{DIM}{child_prefix}{RESET}");
                render_node(graph, seq_id, &next_prefix, seq_is_last, output);
            }
        }
    }
}

fn format_insert_result(
    node_id: &str,
    variable: &str,
    op: OperationType,
    res: &DetectionResult,
) -> String {
    let op_color = operation_color(op);
    let match_str = if res.matches_direct_scan() {
        format!("{GREEN}{BOLD}true{RESET}")
    } else {
        format!("{RED}{BOLD}false{RESET}")
    };

    let mut summary_lines = Vec::new();
    if res.summary_entries.is_empty() {
        summary_lines.push(format!("    {DIM}<none>{RESET}"));
    } else {
        for (cca_type, entry) in &res.summary_entries {
            summary_lines.push(format!(
                "    {RED}{cca_type:?}{RESET}: ({}, {}, {})",
                entry.variable, entry.first_node, entry.second_node
            ));
        }
    }

    format!(
        "  Inserted {op_color}{op:?}({variable}){RESET} into {GREEN}{node_id}{RESET}\n  \
           Matches direct scan: {match_str}\n  \
           Touched AND nodes: {YELLOW}{:?}{RESET}\n  \
           Updated BLOCK summaries: {CYAN}{:?}{RESET}\n  \
           New anomalies created:\n{}",
        res.touched_and_nodes,
        res.summary_blocks_updated,
        summary_lines.join("\n")
    )
}

fn format_delete_result(
    node_id: &str,
    variable: &str,
    op: OperationType,
    res: &DeletionResult,
) -> String {
    let op_color = operation_color(op);
    let removed_str = if res.removed_operation {
        format!("{GREEN}{BOLD}true{RESET}")
    } else {
        format!("{RED}{BOLD}false (operation did not exist){RESET}")
    };

    format!(
        "  Deleted {op_color}{op:?}({variable}){RESET} from {GREEN}{node_id}{RESET}\n  \
           Operation actually removed: {removed_str}\n  \
           Touched AND nodes: {YELLOW}{:?}{RESET}\n  \
           Updated BLOCK summaries: {CYAN}{:?}{RESET}",
        res.touched_and_nodes, res.summary_blocks_updated
    )
}
