use clap::{Parser, Subcommand};
use tt_graph_cdfa_rust::{
    OperationType, bench, build_paper_example_graph, cli_support, format_helper, interactive,
    parse_operation_type,
};

#[derive(Parser)]
#[command(
    name = "tt-graph-cdfa-rust",
    version,
    about = "TT-Graph based Concurrent Data Flow Anomaly detection library and CLI tool"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    #[command(name = "analyze-cpp")]
    AnalyzeCpp {
        path: String,
        #[arg(value_name = "INSERT_ARGS", trailing_var_arg = true)]
        insert_args: Vec<String>,
    },
    #[command(name = "diagnostics-cpp")]
    DiagnosticsCpp { path: Option<String> },
    #[command(name = "diagnostics-cpp-implicit")]
    DiagnosticsCppImplicit { path: Option<String> },
    #[command(name = "bench")]
    Bench,
    #[command(name = "bench-csv")]
    BenchCsv,
    #[command(name = "bench-corpus")]
    BenchCorpus {
        #[arg(long)]
        csv: bool,
    },
    #[command(name = "bench-paper-table5")]
    BenchPaperTable5,
    #[command(name = "figure1")]
    Figure1,
    #[command(name = "figure3")]
    Figure3,
    #[command(name = "figure4")]
    Figure4,
    #[command(name = "figure5")]
    Figure5,
    #[command(name = "figure6")]
    Figure6 { depth: Option<usize> },
    #[command(name = "table1")]
    Table1,
    #[command(name = "table5")]
    Table5,
    #[command(name = "table6")]
    Table6,
    #[command(name = "delete-demo")]
    DeleteDemo,
    #[command(name = "dot")]
    Dot,
    #[command(name = "export-json")]
    ExportJson { path: Option<String> },
    #[command(name = "export-paper-json")]
    ExportPaperJson { path: Option<String> },
    #[command(name = "paper")]
    Paper,
    #[command(name = "cpp")]
    Cpp { path: Option<String> },
    #[command(name = "cpp-implicit")]
    CppImplicit { path: Option<String> },
    #[command(name = "analyze-cpp-implicit")]
    AnalyzeCppImplicit {
        path: String,
        #[arg(value_name = "INSERT_ARGS", trailing_var_arg = true)]
        insert_args: Vec<String>,
    },
    #[command(name = "interactive", alias = "sde-sim")]
    Interactive,
    #[command(name = "demo")]
    Demo,
}

fn main() {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Demo) {
        Command::AnalyzeCpp { path, insert_args } => {
            let args = analyze_args(path, insert_args);
            run_analyze_cpp(&args);
        }
        Command::DiagnosticsCpp { path } => run_diagnostics_cpp(path.as_deref()),
        Command::DiagnosticsCppImplicit { path } => {
            run_diagnostics_cpp_implicit(path.as_deref());
        }
        Command::Bench => bench::run_benchmark(),
        Command::BenchCsv => bench::run_benchmark_csv(),
        Command::BenchCorpus { csv } => print!("{}", cli_support::bench_corpus_output(csv)),
        Command::BenchPaperTable5 => run_bench_paper_table5(),
        Command::Figure1 => tt_graph_cdfa_rust::figures::print_figure1(),
        Command::Figure3 => tt_graph_cdfa_rust::figures::print_figure3(),
        Command::Figure4 => tt_graph_cdfa_rust::figures::print_figure4(),
        Command::Figure5 => tt_graph_cdfa_rust::figures::print_figure5(),
        Command::Figure6 { depth } => run_figure6(depth),
        Command::Table1 => tt_graph_cdfa_rust::figures::print_table1(),
        Command::Table5 => tt_graph_cdfa_rust::figures::print_table5(),
        Command::Table6 => tt_graph_cdfa_rust::figures::print_table6(),
        Command::DeleteDemo => print!("{}", cli_support::delete_demo_output()),
        Command::Dot => print!("{}", cli_support::dot_output()),
        Command::ExportJson { path } => run_export_json(path.as_deref()),
        Command::ExportPaperJson { path } => run_export_paper_json(path.as_deref()),
        Command::Paper => run_paper_reproduction(),
        Command::Cpp { path } => run_cpp(path.as_deref()),
        Command::CppImplicit { path } => run_cpp_implicit(path.as_deref()),
        Command::AnalyzeCppImplicit { path, insert_args } => {
            let args = analyze_args(path, insert_args);
            run_analyze_cpp_implicit(&args);
        }
        Command::Interactive => interactive::run_interactive(),
        Command::Demo => print!("{}", cli_support::demo_output()),
    }
}

fn analyze_args(path: String, insert_args: Vec<String>) -> Vec<String> {
    let mut args = Vec::with_capacity(insert_args.len() + 1);
    args.push(path);
    args.extend(insert_args);
    args
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

fn run_diagnostics_cpp(path: Option<&str>) {
    run_diagnostics_source(
        path.unwrap_or("examples/paper_program1/program1.cpp"),
        "cpp",
        false,
    );
}

fn run_diagnostics_cpp_implicit(path: Option<&str>) {
    run_diagnostics_source(
        path.unwrap_or("examples/paper_program1/program1_plain.cpp"),
        "cpp",
        true,
    );
}

fn run_diagnostics_source(path: &str, language: &str, implicit: bool) {
    let output = cli_support::diagnostics_json_for_cpp(path, language, implicit);
    print!("{}", output.stdout);
    if output.exit_code != 0 {
        std::process::exit(output.exit_code);
    }
}

fn run_cpp_implicit(path: Option<&str>) {
    run_source_reproduction(
        path.unwrap_or("examples/paper_program1/program1_plain.cpp"),
        "C++ (implicit)",
        parse_cpp_implicit_graph,
    );
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
    let path = path.unwrap_or("examples/paper_program1/program1.cpp");
    match cli_support::export_json_for_source(path) {
        Ok(json) => print!("{json}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn run_export_paper_json(path: Option<&str>) {
    let path = path.unwrap_or("examples/paper_program1/program1.cpp");
    match cli_support::export_paper_json_for_source(path) {
        Ok(json) => print!("{json}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn run_cpp(path: Option<&str>) {
    run_source_reproduction(
        path.unwrap_or("examples/paper_program1/program1.cpp"),
        "C++",
        parse_cpp_graph,
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

fn parse_cpp_graph(path: &str) -> tt_graph_cdfa_rust::TTGraph {
    cli_support::parse_cpp_graph(path).unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(1);
    })
}

fn parse_cpp_implicit_graph(path: &str) -> tt_graph_cdfa_rust::TTGraph {
    cli_support::parse_cpp_implicit_graph(path).unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(1);
    })
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

fn run_figure6(depth: Option<usize>) {
    let depth = depth.unwrap_or(4);
    tt_graph_cdfa_rust::figures::print_figure6(depth);
}

fn run_bench_paper_table5() {
    let rows = tt_graph_cdfa_rust::bench_corpus::bench_corpus_rows(10);
    tt_graph_cdfa_rust::bench_corpus::print_table5_comparison(&rows);
}
