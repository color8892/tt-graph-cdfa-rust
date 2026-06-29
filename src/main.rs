use std::env;
use std::fs;
use std::time::{Duration, Instant};

use tt_graph_cdfa_rust::{
    CcaEntry, CcaType, NodeType, OperationType, build_paper_example_graph,
    build_synthetic_full_and_graph, export::graph_to_json,
};

#[derive(Debug)]
struct BenchmarkRow {
    depth: usize,
    node_count: usize,
    leaf_count: usize,
    matching_leaf_count: usize,
    result_count: usize,
    summary_median: Duration,
    direct_median: Duration,
    matches_direct_scan: bool,
}

impl BenchmarkRow {
    fn speedup(&self) -> f64 {
        let summary_ns = self.summary_median.as_nanos() as f64;
        let direct_ns = self.direct_median.as_nanos() as f64;
        if summary_ns == 0.0 {
            f64::INFINITY
        } else {
            direct_ns / summary_ns
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("bench") => run_benchmark(),
        Some("bench-csv") => run_benchmark_csv(),
        Some("delete-demo") => run_delete_demo(),
        Some("dot") => run_dot(),
        Some("export-json") => run_export_json(args.get(2).map(String::as_str)),
        Some("paper") => run_paper_reproduction(),
        Some("parse") => run_parse(args.get(2).map(String::as_str)),
        Some("pseudo") => run_pseudo(args.get(2).map(String::as_str)),
        Some("demo") | None => run_demo(),
        Some(command) => {
            eprintln!("unknown command: {command}");
            eprintln!("usage: cargo run -- paper");
            eprintln!("       cargo run -- parse examples/program1.tt");
            eprintln!("       cargo run -- pseudo examples/program1.pseudo");
            eprintln!("       cargo run -- export-json examples/program1.pseudo");
            eprintln!("usage: cargo run -- demo");
            eprintln!("       cargo run -- delete-demo");
            eprintln!("       cargo run -- dot");
            eprintln!("       cargo run --release -- bench");
            eprintln!("       cargo run --release -- bench-csv");
            std::process::exit(2);
        }
    }
}

fn run_export_json(path: Option<&str>) {
    let mut graph = parse_pseudo_graph(path.unwrap_or("examples/program1.pseudo"));
    graph.insert_operation("Act2", "v", OperationType::Write);
    print!("{}", graph_to_json(&graph));
}

fn run_pseudo(path: Option<&str>) {
    let path = path.unwrap_or("examples/program1.pseudo");
    let mut graph = parse_pseudo_graph(path);

    println!("Parsed TT Graph from pseudo-code {path}");
    if is_paper_program_1_graph(&graph) {
        println!("Table 2 d_OPN_set reproduction from parsed pseudo-code");
        print_paper_table_2(&graph);

        let result = graph.insert_operation("Act2", "v", OperationType::Write);
        println!();
        println!("After pseudo Program 2 insertion: Write(v) into Act2");
        println!("Matches direct scan: {}", result.matches_direct_scan());
        println!("New CCA entries:");
        print_entries(&result.summary_entries);
    } else {
        println!("d_OPN_set rows from parsed pseudo-code");
        print_all_d_opn_rows(&graph);
    }
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
    print_paper_table_2(&graph);

    let result = graph.insert_operation("Act2", "v", OperationType::Write);
    println!();
    println!("After parsed Program 2 insertion: Write(v) into Act2");
    println!("Matches direct scan: {}", result.matches_direct_scan());
    println!("New CCA entries:");
    print_entries(&result.summary_entries);
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
    print_entries(&result.summary_entries);
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
    print_paper_table_2(&graph);

    let initial_counts = anomaly_counts(&graph, "And1");
    println!();
    println!("Initial CCA counts on And1, computed from Program 1");
    print_counts(&initial_counts);

    let result = graph.insert_operation("Act2", "v", OperationType::Write);
    println!();
    println!("Program 2 insertion: add Write(v) into Act2");
    println!(
        "Algorithm 1 d_OPN_set result matches Algorithm 2 direct scan: {}",
        result.matches_direct_scan()
    );
    println!();
    println!("Table 3 reproduction: newly constructed anomalies from insertion");
    print_entries(&result.summary_entries);
    println!();
    println!("Algorithm 2 direct-scan baseline for the same insertion");
    print_entries(&result.direct_entries);
    println!();
    println!("Table 4 reproduction: updated d_OPN_set of B1");
    print_paper_table_4(&graph);

    let after_insert_counts = anomaly_counts(&graph, "And1");
    println!();
    println!("Table 3 full CCA sets on And1 after insertion");
    print_cca_sets(&graph, "And1");
    println!();
    println!("CCA counts on And1 after insertion");
    print_counts(&after_insert_counts);

    let delete_result = graph.delete_operation("Act2", "v", OperationType::Write);
    let after_delete_counts = anomaly_counts(&graph, "And1");
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

fn run_benchmark() {
    let (iterations, matching_stride, rows) = benchmark_rows();

    println!("TT Graph CDFA Rust benchmark");
    println!(
        "Measures insertion detection only; graph construction and cloning are outside timed sections."
    );
    println!("iterations={iterations}, matching_stride={matching_stride}, insertion=Write(target)");
    println!();
    println!(
        "{:>5}  {:>5}  {:>6}  {:>12}  {:>3}  {:>12}  {:>9}  {:>8}  {:>5}",
        "depth",
        "nodes",
        "leaves",
        "target reads",
        "CCA",
        "d_OPN_set us",
        "direct us",
        "x faster",
        "match"
    );
    println!(
        "{:>5}  {:>5}  {:>6}  {:>12}  {:>3}  {:>12}  {:>9}  {:>8}  {:>5}",
        "-----",
        "-----",
        "------",
        "------------",
        "---",
        "------------",
        "---------",
        "--------",
        "-----"
    );
    for row in rows {
        println!(
            "{:>5}  {:>5}  {:>6}  {:>12}  {:>3}  {:>12.1}  {:>9.1}  {:>8.1}  {:>5}",
            row.depth,
            row.node_count,
            row.leaf_count,
            row.matching_leaf_count,
            row.result_count,
            row.summary_median.as_nanos() as f64 / 1_000.0,
            row.direct_median.as_nanos() as f64 / 1_000.0,
            row.speedup(),
            row.matches_direct_scan
        );
    }
}

fn run_benchmark_csv() {
    let (_, _, rows) = benchmark_rows();

    println!("{}", benchmark_csv_header());
    for row in rows {
        println!("{}", benchmark_row_to_csv(&row));
    }
}

fn benchmark_rows() -> (usize, usize, Vec<BenchmarkRow>) {
    let depths = [4, 6, 8];
    let iterations = 20;
    let matching_stride = 16;
    let mut rows = Vec::new();

    for depth in depths {
        rows.push(bench_depth(depth, iterations, matching_stride));
    }

    (iterations, matching_stride, rows)
}

fn bench_depth(depth: usize, iterations: usize, matching_stride: usize) -> BenchmarkRow {
    let case = build_synthetic_full_and_graph(depth, matching_stride);
    let mut summary_times = Vec::new();
    let mut direct_times = Vec::new();
    let mut result_count = 0;
    let mut matches_direct_scan = true;

    for _ in 0..iterations {
        let mut summary_graph = case.graph.clone();
        let started_at = Instant::now();
        let summary = summary_graph.insert_operation_summary_only(
            &case.target_node_id,
            "target",
            OperationType::Write,
        );
        summary_times.push(started_at.elapsed());

        let mut direct_graph = case.graph.clone();
        let started_at = Instant::now();
        let direct = direct_graph.insert_operation_direct_only(
            &case.target_node_id,
            "target",
            OperationType::Write,
        );
        direct_times.push(started_at.elapsed());

        result_count = summary.entries.len();
        matches_direct_scan = matches_direct_scan && summary.entries == direct.entries;
    }

    summary_times.sort();
    direct_times.sort();

    BenchmarkRow {
        depth,
        node_count: case.node_count,
        leaf_count: case.leaf_count,
        matching_leaf_count: case.matching_leaf_count,
        result_count,
        summary_median: summary_times[summary_times.len() / 2],
        direct_median: direct_times[direct_times.len() / 2],
        matches_direct_scan,
    }
}

fn benchmark_csv_header() -> &'static str {
    "depth,nodes,leaves,target_reads,cca,d_opn_set_us,direct_us,x_faster,match"
}

fn benchmark_row_to_csv(row: &BenchmarkRow) -> String {
    format!(
        "{},{},{},{},{},{:.1},{:.1},{:.1},{}",
        row.depth,
        row.node_count,
        row.leaf_count,
        row.matching_leaf_count,
        row.result_count,
        row.summary_median.as_nanos() as f64 / 1_000.0,
        row.direct_median.as_nanos() as f64 / 1_000.0,
        row.speedup(),
        row.matches_direct_scan
    )
}

fn print_entries(entries: &std::collections::BTreeSet<(CcaType, CcaEntry)>) {
    for (cca_type, entry) in entries {
        println!(
            "  {cca_type:?}: ({}, {}, {})",
            entry.variable, entry.first_node, entry.second_node
        );
    }
}

fn print_paper_table_2(graph: &tt_graph_cdfa_rust::TTGraph) {
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

fn print_paper_table_4(graph: &tt_graph_cdfa_rust::TTGraph) {
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

fn print_d_opn_rows(graph: &tt_graph_cdfa_rust::TTGraph, rows: &[(&str, &str, OperationType)]) {
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

fn print_all_d_opn_rows(graph: &tt_graph_cdfa_rust::TTGraph) {
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

fn is_paper_program_1_graph(graph: &tt_graph_cdfa_rust::TTGraph) -> bool {
    [
        "And1", "B1", "B2", "B3", "B4", "B5", "Act1", "Act2", "Act3", "Act4", "Act5", "Loop1",
        "Xor1",
    ]
    .iter()
    .all(|node_id| graph.nodes.contains_key(*node_id))
}

fn anomaly_counts(graph: &tt_graph_cdfa_rust::TTGraph, and_id: &str) -> Vec<(CcaType, usize)> {
    let mut rows: Vec<(CcaType, usize)> = graph.nodes[and_id]
        .cca_sets
        .iter()
        .map(|(cca_type, entries)| (*cca_type, entries.len()))
        .collect();
    rows.sort_by_key(|(cca_type, _)| *cca_type);
    rows
}

fn print_counts(counts: &[(CcaType, usize)]) {
    for (cca_type, count) in counts {
        println!("  {cca_type:?}: {count}");
    }
}

fn print_cca_sets(graph: &tt_graph_cdfa_rust::TTGraph, and_id: &str) {
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
    use std::time::Duration;

    use super::{
        BenchmarkRow, benchmark_csv_header, benchmark_row_to_csv, is_paper_program_1_graph,
    };
    use tt_graph_cdfa_rust::{build_paper_example_graph, build_synthetic_full_and_graph};

    #[test]
    fn benchmark_csv_row_is_machine_readable() {
        let row = BenchmarkRow {
            depth: 4,
            node_count: 61,
            leaf_count: 16,
            matching_leaf_count: 1,
            result_count: 1,
            summary_median: Duration::from_nanos(6_800),
            direct_median: Duration::from_nanos(20_400),
            matches_direct_scan: true,
        };

        assert_eq!(
            benchmark_csv_header(),
            "depth,nodes,leaves,target_reads,cca,d_opn_set_us,direct_us,x_faster,match"
        );
        assert_eq!(benchmark_row_to_csv(&row), "4,61,16,1,1,6.8,20.4,3.0,true");
    }

    #[test]
    fn classifies_only_the_paper_program_1_graph_as_paper_output() {
        let paper_graph = build_paper_example_graph();
        let synthetic_graph = build_synthetic_full_and_graph(1, 1).graph;

        assert!(is_paper_program_1_graph(&paper_graph));
        assert!(!is_paper_program_1_graph(&synthetic_graph));
    }
}
