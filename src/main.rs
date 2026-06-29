use std::env;
use std::fs;
use std::io::{self, Write};
use tt_graph_cdfa_rust::{
    bench, format_helper, build_paper_example_graph, export::graph_to_json,
    ControlType, NodeType, OperationType,
};

const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const RESET: &str = "\x1b[0m";

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("analyze-c") => run_analyze_c(&args[2..]),
        Some("analyze-cpp") => run_analyze_cpp(&args[2..]),
        Some("analyze-pseudo") => run_analyze_pseudo(&args[2..]),
        Some("bench") => bench::run_benchmark(),
        Some("bench-csv") => bench::run_benchmark_csv(),
        Some("bench-corpus") => run_bench_corpus(&args[2..]),
        Some("bench-paper-table5") => run_bench_paper_table5(),
        Some("figure1") => tt_graph_cdfa_rust::figures::print_figure1(),
        Some("figure3") => tt_graph_cdfa_rust::figures::print_figure3(),
        Some("figure4") => tt_graph_cdfa_rust::figures::print_figure4(),
        Some("figure5") => tt_graph_cdfa_rust::figures::print_figure5(),
        Some("figure6") => run_figure6(args.get(2)),
        Some("table1") => tt_graph_cdfa_rust::figures::print_table1(),
        Some("table5") => tt_graph_cdfa_rust::figures::print_table5(),
        Some("table6") => tt_graph_cdfa_rust::figures::print_table6(),
        Some("delete-demo") => run_delete_demo(),
        Some("dot") => run_dot(),
        Some("export-json") => run_export_json(args.get(2).map(String::as_str)),
        Some("paper") => run_paper_reproduction(),
        Some("c") => run_c(args.get(2).map(String::as_str)),
        Some("cpp") => run_cpp(args.get(2).map(String::as_str)),
        Some("cpp-implicit") => run_cpp_implicit(args.get(2).map(String::as_str)),
        Some("analyze-cpp-implicit") => run_analyze_cpp_implicit(&args[2..]),
        Some("parse") => run_parse(args.get(2).map(String::as_str)),
        Some("pseudo") => run_pseudo(args.get(2).map(String::as_str)),
        Some("interactive") | Some("sde-sim") => run_interactive(),
        Some("demo") | None => run_demo(),
        Some(command) => {
            eprintln!("unknown command: {command}");
            eprintln!("usage: cargo run -- paper");
            eprintln!("       cargo run -- interactive");
            eprintln!(
                "       cargo run -- analyze-cpp <path> [insert <node> <variable> <Read|Write|Kill>]"
            );
            eprintln!(
                "       cargo run -- analyze-c <path> [insert <node> <variable> <Read|Write|Kill>]"
            );
            eprintln!(
                "       cargo run -- analyze-pseudo <path> [insert <node> <variable> <Read|Write|Kill>]"
            );
            eprintln!("       cargo run -- cpp examples/program1.cpp");
            eprintln!("       cargo run -- cpp-implicit examples/program1_plain.cpp");
            eprintln!(
                "       cargo run -- analyze-cpp-implicit <path> [insert <node> <variable> <Read|Write|Kill>]"
            );
            eprintln!("       cargo run -- c examples/program1.c");
            eprintln!("       cargo run -- parse examples/program1.tt");
            eprintln!("       cargo run -- pseudo examples/program1.pseudo");
            eprintln!("       cargo run -- export-json examples/program1.cpp");
            eprintln!("usage: cargo run -- demo");
            eprintln!("       cargo run -- delete-demo");
            eprintln!("       cargo run -- dot");
            eprintln!("       cargo run --release -- bench");
            eprintln!("       cargo run --release -- bench-csv");
            eprintln!("       cargo run --release -- bench-corpus [--csv]");
            eprintln!("       cargo run --release -- bench-paper-table5");
            eprintln!(
                "       cargo run -- figure1 | figure3 | figure4 | figure5 | figure6 [depth]"
            );
            eprintln!("       cargo run -- table1 | table5 | table6");
            std::process::exit(2);
        }
    }
}

fn run_analyze_c(args: &[String]) {
    run_analyze_source(args, "analyze-c", "C subset", parse_c_subset_graph);
}

fn run_analyze_cpp(args: &[String]) {
    run_analyze_source(args, "analyze-cpp", "C++", parse_cpp_graph);
}

fn run_analyze_cpp_implicit(args: &[String]) {
    run_analyze_source(
        args,
        "analyze-cpp-implicit",
        "C++ (implicit)",
        parse_cpp_implicit_graph,
    );
}

fn run_cpp_implicit(path: Option<&str>) {
    run_source_reproduction(
        path.unwrap_or("examples/program1_plain.cpp"),
        "C++ (implicit)",
        parse_cpp_implicit_graph,
    );
}

fn run_analyze_pseudo(args: &[String]) {
    run_analyze_source(args, "analyze-pseudo", "pseudo-code", parse_pseudo_graph);
}

fn run_analyze_source(
    args: &[String],
    command: &str,
    label: &str,
    parse: fn(&str) -> tt_graph_cdfa_rust::TTGraph,
) {
    if args.is_empty() {
        eprintln!(
            "usage: cargo run -- {command} <path> [insert <node> <variable> <Read|Write|Kill>]"
        );
        std::process::exit(2);
    }

    let path = &args[0];
    let mut graph = parse(path);
    println!("Parsed TT Graph from {label} {path}");
    println!("Initial d_OPN_set rows:");
    format_helper::print_all_d_opn_rows(&graph);

    if args.len() == 1 {
        return;
    }

    if args.len() != 5 || args[1] != "insert" {
        eprintln!(
            "usage: cargo run -- {command} <path> [insert <node> <variable> <Read|Write|Kill>]"
        );
        std::process::exit(2);
    }

    let node_id = &args[2];
    if !graph.nodes.contains_key(node_id) {
        eprintln!("cannot insert into missing node `{node_id}`");
        std::process::exit(1);
    }

    let variable = &args[3];
    let operation = parse_operation_type(&args[4]).unwrap_or_else(|| {
        eprintln!(
            "unknown operation `{}`; expected Read, Write, or Kill",
            args[4]
        );
        std::process::exit(2);
    });

    let result = graph.insert_operation(node_id, variable, operation);
    println!();
    println!("After insertion: {operation:?}({variable}) into {node_id}");
    println!("Matches direct scan: {}", result.matches_direct_scan());
    println!("Touched AND nodes: {:?}", result.touched_and_nodes);
    println!(
        "Updated BLOCK summaries: {:?}",
        result.summary_blocks_updated
    );
    println!("New CCA entries:");
    format_helper::print_entries(&result.summary_entries);
    println!();
    println!("Updated d_OPN_set rows:");
    format_helper::print_all_d_opn_rows(&graph);
}

fn run_export_json(path: Option<&str>) {
    let path = path.unwrap_or("examples/program1.cpp");
    let mut graph = parse_source_graph(path);
    graph.insert_operation("Act2", "v", OperationType::Write);
    print!("{}", graph_to_json(&graph));
}

fn run_cpp(path: Option<&str>) {
    run_source_reproduction(
        path.unwrap_or("examples/program1.cpp"),
        "C++",
        parse_cpp_graph,
    );
}

fn run_c(path: Option<&str>) {
    run_source_reproduction(
        path.unwrap_or("examples/program1.c"),
        "C subset",
        parse_c_subset_graph,
    );
}

fn run_pseudo(path: Option<&str>) {
    run_source_reproduction(
        path.unwrap_or("examples/program1.pseudo"),
        "pseudo-code",
        parse_pseudo_graph,
    );
}

fn run_source_reproduction(
    path: &str,
    label: &str,
    parse: fn(&str) -> tt_graph_cdfa_rust::TTGraph,
) {
    let mut graph = parse(path);

    println!("Parsed TT Graph from {label} {path}");
    if format_helper::is_paper_program_1_graph(&graph) {
        println!("Table 2 d_OPN_set reproduction from parsed {label}");
        format_helper::print_paper_table_2(&graph);

        let result = graph.insert_operation("Act2", "v", OperationType::Write);
        println!();
        println!("After Program 2 insertion: Write(v) into Act2");
        println!("Matches direct scan: {}", result.matches_direct_scan());
        println!("New CCA entries:");
        format_helper::print_entries(&result.summary_entries);
    } else {
        println!("d_OPN_set rows from parsed {label}");
        format_helper::print_all_d_opn_rows(&graph);
    }
}

fn parse_source_graph(path: &str) -> tt_graph_cdfa_rust::TTGraph {
    if path.ends_with(".cpp") || path.ends_with(".cc") || path.ends_with(".cxx") {
        parse_cpp_graph(path)
    } else if path.ends_with(".c") {
        parse_c_subset_graph(path)
    } else if path.ends_with(".pseudo") {
        parse_pseudo_graph(path)
    } else {
        eprintln!("unsupported source path `{path}`; expected .cpp, .c, or .pseudo");
        std::process::exit(2);
    }
}

fn parse_cpp_graph(path: &str) -> tt_graph_cdfa_rust::TTGraph {
    #[cfg(feature = "clang")]
    {
        tt_graph_cdfa_rust::clang_frontend::parse_cpp_file(path).unwrap_or_else(|error| {
            eprintln!("failed to parse {path}: {error}");
            std::process::exit(1);
        })
    }
    #[cfg(not(feature = "clang"))]
    {
        eprintln!("C++ Clang frontend is disabled; rebuild with `--features clang`");
        std::process::exit(1);
    }
}

fn parse_cpp_implicit_graph(path: &str) -> tt_graph_cdfa_rust::TTGraph {
    #[cfg(feature = "clang")]
    {
        tt_graph_cdfa_rust::clang_frontend::parse_cpp_implicit_file(path).unwrap_or_else(|error| {
            eprintln!("failed to parse {path}: {error}");
            std::process::exit(1);
        })
    }
    #[cfg(not(feature = "clang"))]
    {
        eprintln!("C++ Clang frontend is disabled; rebuild with `--features clang`");
        std::process::exit(1);
    }
}

fn parse_c_subset_graph(path: &str) -> tt_graph_cdfa_rust::TTGraph {
    let source = fs::read_to_string(path).unwrap_or_else(|error| {
        eprintln!("failed to read {path}: {error}");
        std::process::exit(1);
    });
    tt_graph_cdfa_rust::c_frontend::parse_c_program(&source).unwrap_or_else(|error| {
        eprintln!("failed to parse {path}: {error}");
        std::process::exit(1);
    })
}

fn parse_pseudo_graph(path: &str) -> tt_graph_cdfa_rust::TTGraph {
    let source = fs::read_to_string(path).unwrap_or_else(|error| {
        eprintln!("failed to read {path}: {error}");
        std::process::exit(1);
    });
    tt_graph_cdfa_rust::pseudo::parse_pseudo_program(&source).unwrap_or_else(|error| {
        eprintln!("failed to parse {path}: {error}");
        std::process::exit(1);
    })
}

fn parse_operation_type(value: &str) -> Option<OperationType> {
    match value.to_ascii_lowercase().as_str() {
        "read" => Some(OperationType::Read),
        "write" => Some(OperationType::Write),
        "kill" => Some(OperationType::Kill),
        _ => None,
    }
}

fn run_parse(path: Option<&str>) {
    let path = path.unwrap_or("examples/program1.tt");
    let source = fs::read_to_string(path).unwrap_or_else(|error| {
        eprintln!("failed to read {path}: {error}");
        std::process::exit(1);
    });
    let mut graph = tt_graph_cdfa_rust::toy::parse_toy_program(&source).unwrap_or_else(|error| {
        eprintln!("failed to parse {path}: {error}");
        std::process::exit(1);
    });

    println!("Parsed TT Graph from {path}");
    println!("Table 2 d_OPN_set reproduction from parsed input");
    format_helper::print_paper_table_2(&graph);

    let result = graph.insert_operation("Act2", "v", OperationType::Write);
    println!();
    println!("After parsed Program 2 insertion: Write(v) into Act2");
    println!("Matches direct scan: {}", result.matches_direct_scan());
    println!("New CCA entries:");
    format_helper::print_entries(&result.summary_entries);
}

fn run_demo() {
    let mut graph = build_paper_example_graph();
    let result = graph.insert_operation("Act2", "v", OperationType::Write);

    println!("TT Graph CDFA Rust demo");
    println!("Insertion: Write(v) into Act2");
    println!(
        "Summary matches direct scan: {}",
        result.matches_direct_scan()
    );
    println!("Touched AND nodes: {:?}", result.touched_and_nodes);
    println!(
        "Updated BLOCK summaries: {:?}",
        result.summary_blocks_updated
    );
    println!();
    println!("New anomalies from d_OPN_set:");
    format_helper::print_entries(&result.summary_entries);
    println!();
    println!("B1 d_OPN_set rows after insertion:");
    for (variable, op, node_ids) in graph.sorted_d_opn_rows("B1") {
        println!("  d_OPN_set({variable}, {op:?}, B1) = {node_ids:?}");
    }
}

fn run_paper_reproduction() {
    let mut graph = build_paper_example_graph();

    println!("TT Graph CDFA paper reproduction");
    println!("Paper: Program 1 / Figure 2 -> Program 2 insertion");
    println!();
    println!("Table 2 reproduction: d_OPN_set for BLOCK nodes before insertion");
    format_helper::print_paper_table_2(&graph);

    let initial_counts = format_helper::anomaly_counts(&graph, "And1");
    println!();
    println!("Initial CCA counts on And1, computed from Program 1");
    format_helper::print_counts(&initial_counts);

    let result = graph.insert_operation("Act2", "v", OperationType::Write);
    println!();
    println!("Program 2 insertion: add Write(v) into Act2");
    println!(
        "Algorithm 1 d_OPN_set result matches Algorithm 2 direct scan: {}",
        result.matches_direct_scan()
    );
    println!();
    println!("Table 3 reproduction: newly constructed anomalies from insertion");
    format_helper::print_entries(&result.summary_entries);
    println!();
    println!("Algorithm 2 direct-scan baseline for the same insertion");
    format_helper::print_entries(&result.direct_entries);
    println!();
    println!("Table 4 reproduction: updated d_OPN_set of B1");
    format_helper::print_paper_table_4(&graph);

    let after_insert_counts = format_helper::anomaly_counts(&graph, "And1");
    println!();
    println!("Table 3 full CCA sets on And1 after insertion");
    format_helper::print_cca_sets(&graph, "And1");
    println!();
    println!("CCA counts on And1 after insertion");
    format_helper::print_counts(&after_insert_counts);

    let delete_result = graph.delete_operation("Act2", "v", OperationType::Write);
    let after_delete_counts = format_helper::anomaly_counts(&graph, "And1");
    println!();
    println!("Future-work extension check: delete the inserted Write(v) from Act2");
    println!("Removed operation: {}", delete_result.removed_operation);
    println!(
        "After deletion returns to initial CCA counts: {}",
        after_delete_counts == initial_counts
    );
}

fn run_delete_demo() {
    let mut graph = build_paper_example_graph();
    let insert_result = graph.insert_operation("Act2", "v", OperationType::Write);
    let delete_result = graph.delete_operation("Act2", "v", OperationType::Write);

    println!("TT Graph CDFA Rust deletion demo");
    println!(
        "Setup insertion produced {} CCA entries",
        insert_result.summary_entries.len()
    );
    println!("Deletion: remove Write(v) from Act2");
    println!("Removed operation: {}", delete_result.removed_operation);
    println!("Touched AND nodes: {:?}", delete_result.touched_and_nodes);
    println!(
        "Updated BLOCK summaries: {:?}",
        delete_result.summary_blocks_updated
    );
    println!();
    println!("And1 anomalies after deletion:");
    for (cca_type, entries) in &graph.nodes["And1"].cca_sets {
        println!("  {cca_type:?}: {}", entries.len());
    }
    println!();
    println!("B1 d_OPN_set rows after deletion:");
    for (variable, op, node_ids) in graph.sorted_d_opn_rows("B1") {
        println!("  d_OPN_set({variable}, {op:?}, B1) = {node_ids:?}");
    }
}

fn run_dot() {
    let mut graph = build_paper_example_graph();
    graph.insert_operation("Act2", "v", OperationType::Write);
    print!("{}", graph.to_dot());
}

fn run_figure6(depth: Option<&String>) {
    let depth = depth
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(4);
    tt_graph_cdfa_rust::figures::print_figure6(depth);
}

fn run_bench_corpus(args: &[String]) {
    let csv = args.first().is_some_and(|arg| arg == "--csv");
    let rows = tt_graph_cdfa_rust::bench_corpus::bench_corpus_rows(10);
    if csv {
        println!("{}", tt_graph_cdfa_rust::bench_corpus::corpus_csv_header());
        for row in &rows {
            println!(
                "{}",
                tt_graph_cdfa_rust::bench_corpus::corpus_row_to_csv(row)
            );
        }
        return;
    }

    println!("TT Graph CDFA benchmark corpus");
    println!("cases={}", rows.len());
    for row in &rows {
        println!(
            "{} nodes={} leaves={} summary_us={:.1} direct_us={:.1} speedup={:.1}x match={}",
            row.case_id,
            row.node_count,
            row.leaf_count,
            row.summary_median_us,
            row.direct_median_us,
            row.speedup,
            row.matches_direct_scan
        );
    }
}

fn run_bench_paper_table5() {
    let rows = tt_graph_cdfa_rust::bench_corpus::bench_corpus_rows(10);
    tt_graph_cdfa_rust::bench_corpus::print_table5_comparison(&rows);
}

fn run_interactive() {
    let mut graph = build_paper_example_graph();
    let mut last_action_info = None;

    loop {
        // Clear screen
        print!("\x1b[2J\x1b[H");
        io::stdout().flush().unwrap();

        // Print header
        println!("{BOLD}{CYAN}=== TT Graph CDFA - SDE Simulator ==={RESET}");
        println!("{DIM}Simulate incremental variable operations edits and trace concurrency anomalies live.{RESET}\n");

        // Render Graph ASCII Tree
        println!("{BOLD}1. Current TT Graph Structure:{RESET}");
        print!("{}", render_graph_ascii(&graph));
        println!();

        // Render d_OPN_set summaries
        println!("{BOLD}2. d_OPN_set Summaries (per BLOCK):{RESET}");
        for block_id in &["B1", "B2", "B3", "B4", "B5"] {
            let rows = graph.sorted_d_opn_rows(block_id);
            if rows.is_empty() {
                println!("  {CYAN}{block_id}{RESET}: {DIM}<empty>{RESET}");
            } else {
                println!("  {CYAN}{block_id}{RESET}:");
                for (var, op, nodes) in rows {
                    let op_color = match op {
                        OperationType::Read => GREEN,
                        OperationType::Write => YELLOW,
                        OperationType::Kill => RED,
                    };
                    println!("    d_OPN_set({var}, {op_color}{op:?}{RESET}) = {GREEN}{nodes:?}{RESET}");
                }
            }
        }
        println!();

        // Render anomalies on And1
        println!("{BOLD}3. Stored Anomalies (on And1):{RESET}");
        if let Some(and_node) = graph.nodes.get("And1") {
            let mut cca_types: Vec<tt_graph_cdfa_rust::CcaType> = and_node.cca_sets.keys().copied().collect();
            cca_types.sort();
            for cca_type in cca_types {
                let entries = &and_node.cca_sets[&cca_type];
                let mut rendered = Vec::new();
                for entry in entries {
                    rendered.push(format!("({}, {}, {})", entry.variable, entry.first_node, entry.second_node));
                }
                rendered.sort();
                println!("  {RED}{BOLD}{cca_type:?}{RESET}: {RED}{:?}{RESET}", rendered);
            }
        } else {
            println!("  {DIM}<No And1 control node in graph>{RESET}");
        }
        println!();

        // Render last action result
        println!("{BOLD}4. Last Action Result:{RESET}");
        if let Some(ref info) = last_action_info {
            println!("{info}");
        } else {
            println!("  {DIM}No action performed yet. Choose an option below to modify the graph.{RESET}");
        }
        println!();

        // Render shortcuts
        println!("{BOLD}Shortcuts & Commands:{RESET}");
        println!("  {GREEN}[1]{RESET} Insert {YELLOW}Write(v){RESET} into {GREEN}Act2{RESET}  (Paper Program 2 example)");
        println!("  {GREEN}[2]{RESET} Insert {GREEN}Read(i){RESET} into {GREEN}Act4{RESET}   (Extra edge case)");
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
            last_action_info = Some(format!("  {GREEN}Graph reset to initial Program 1 state.{RESET}"));
            continue;
        }

        if choice == "1" {
            let res = graph.insert_operation("Act2", "v", OperationType::Write);
            last_action_info = Some(format_insert_result("Act2", "v", OperationType::Write, &res));
            continue;
        }

        if choice == "2" {
            let res = graph.insert_operation("Act4", "i", OperationType::Read);
            last_action_info = Some(format_insert_result("Act4", "i", OperationType::Read, &res));
            continue;
        }

        if choice == "3" {
            let res = graph.delete_operation("Act2", "v", OperationType::Write);
            last_action_info = Some(format_delete_result("Act2", "v", OperationType::Write, &res));
            continue;
        }

        // Custom command parsing
        let tokens: Vec<&str> = choice.split_whitespace().collect();
        if tokens.len() == 4 && (tokens[0] == "i" || tokens[0] == "insert") {
            let node_id = tokens[1];
            let variable = tokens[2];
            if !graph.nodes.contains_key(node_id) {
                last_action_info = Some(format!("  {RED}Error: Node `{node_id}` not found in graph.{RESET}"));
                continue;
            }
            if let Some(op) = parse_operation_type(tokens[3]) {
                let res = graph.insert_operation(node_id, variable, op);
                last_action_info = Some(format_insert_result(node_id, variable, op, &res));
            } else {
                last_action_info = Some(format!("  {RED}Error: Unknown operation type `{}`. Use read, write, or kill.{RESET}", tokens[3]));
            }
            continue;
        }

        if tokens.len() == 4 && (tokens[0] == "d" || tokens[0] == "delete") {
            let node_id = tokens[1];
            let variable = tokens[2];
            if !graph.nodes.contains_key(node_id) {
                last_action_info = Some(format!("  {RED}Error: Node `{node_id}` not found in graph.{RESET}"));
                continue;
            }
            if let Some(op) = parse_operation_type(tokens[3]) {
                let res = graph.delete_operation(node_id, variable, op);
                last_action_info = Some(format_delete_result(node_id, variable, op, &res));
            } else {
                last_action_info = Some(format!("  {RED}Error: Unknown operation type `{}`. Use read, write, or kill.{RESET}", tokens[3]));
            }
            continue;
        }

        last_action_info = Some(format!("  {RED}Error: Unknown command `{choice}`. Check help/shortcuts.{RESET}"));
    }
}

fn format_ops(operations: &std::collections::HashSet<tt_graph_cdfa_rust::Operation>) -> String {
    if operations.is_empty() {
        return String::new();
    }
    let mut ops: Vec<String> = operations.iter()
        .map(|op| {
            let color = match op.op {
                OperationType::Read => GREEN,
                OperationType::Write => YELLOW,
                OperationType::Kill => RED,
            };
            format!("{color}{:?}({}){RESET}", op.op, op.variable)
        })
        .collect();
    ops.sort();
    format!("{DIM}[{RESET}{}{DIM}]{RESET}", ops.join(&format!("{DIM}, {RESET}")))
}

fn render_graph_ascii(graph: &tt_graph_cdfa_rust::TTGraph) -> String {
    let mut output = String::new();
    let mut roots: Vec<String> = graph.nodes.values()
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

fn render_node(graph: &tt_graph_cdfa_rust::TTGraph, node_id: &str, prefix: &str, is_last: bool, output: &mut String) {
    let node = &graph.nodes[node_id];
    let marker = format!("{DIM}{}{RESET}", if is_last { "└── " } else { "├── " });
    let child_prefix = if is_last { "    " } else { "│   " };

    match node.node_type {
        NodeType::Activity => {
            let ops_str = format_ops(&node.operations);
            output.push_str(&format!("{prefix}{marker}{GREEN}{node_id}{RESET} {ops_str}\n"));
        }
        NodeType::Control => {
            let kind_str = match node.control_type {
                Some(ControlType::And) => format!("{YELLOW}[AND]{RESET}"),
                Some(ControlType::Xor) => format!("{YELLOW}[XOR]{RESET}"),
                Some(ControlType::Loop) => format!("{YELLOW}[LOOP]{RESET}"),
                None => String::new(),
            };
            let ops_str = format_ops(&node.operations);
            output.push_str(&format!("{prefix}{marker}{YELLOW}{node_id}{RESET} {kind_str} {ops_str}\n"));
            
            let num_branches = node.branch_arc.len();
            for (idx, branch_id) in node.branch_arc.iter().enumerate() {
                let branch_is_last = idx == num_branches - 1;
                let next_prefix = format!("{prefix}{DIM}{child_prefix}{RESET}");
                render_node(graph, branch_id, &next_prefix, branch_is_last, output);
            }
        }
        NodeType::Block => {
            output.push_str(&format!("{prefix}{marker}{CYAN}{node_id}{RESET}\n"));
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

fn format_insert_result(node_id: &str, variable: &str, op: OperationType, res: &tt_graph_cdfa_rust::DetectionResult) -> String {
    let op_color = match op {
        OperationType::Read => GREEN,
        OperationType::Write => YELLOW,
        OperationType::Kill => RED,
    };
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
            summary_lines.push(format!("    {RED}{cca_type:?}{RESET}: ({}, {}, {})", entry.variable, entry.first_node, entry.second_node));
        }
    }

    format!(
        "  Inserted {op_color}{op:?}({variable}){RESET} into {GREEN}{node_id}{RESET}\n  \
           Matches direct scan: {match_str}\n  \
           Touched AND nodes: {YELLOW}{:?}{RESET}\n  \
           Updated BLOCK summaries: {CYAN}{:?}{RESET}\n  \
           New anomalies created:\n{}",
        res.touched_and_nodes, res.summary_blocks_updated, summary_lines.join("\n")
    )
}

fn format_delete_result(node_id: &str, variable: &str, op: OperationType, res: &tt_graph_cdfa_rust::DeletionResult) -> String {
    let op_color = match op {
        OperationType::Read => GREEN,
        OperationType::Write => YELLOW,
        OperationType::Kill => RED,
    };
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
