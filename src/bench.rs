use crate::{OperationType, build_synthetic_full_and_graph};
use std::time::{Duration, Instant};

/// A single row of benchmark results.
#[derive(Debug, Clone)]
pub struct BenchmarkRow {
    pub depth: usize,
    pub node_count: usize,
    pub leaf_count: usize,
    pub matching_leaf_count: usize,
    pub result_count: usize,
    pub summary_median: Duration,
    pub direct_median: Duration,
    pub matches_direct_scan: bool,
}

impl BenchmarkRow {
    /// Returns the speedup factor of summary-based insertion compared to direct scan.
    pub fn speedup(&self) -> f64 {
        let summary_ns = self.summary_median.as_nanos() as f64;
        let direct_ns = self.direct_median.as_nanos() as f64;
        if summary_ns == 0.0 {
            f64::INFINITY
        } else {
            direct_ns / summary_ns
        }
    }
}

/// Runs the benchmark suite and prints the formatted results table.
pub fn run_benchmark() {
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

/// Runs the benchmark suite and prints the results in CSV format.
pub fn run_benchmark_csv() {
    let (_, _, rows) = benchmark_rows();

    println!("{}", benchmark_csv_header());
    for row in rows {
        println!("{}", benchmark_row_to_csv(&row));
    }
}

/// Generates benchmark rows for depths [4, 6, 8].
pub fn benchmark_rows() -> (usize, usize, Vec<BenchmarkRow>) {
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

pub fn benchmark_csv_header() -> &'static str {
    "depth,nodes,leaves,target_reads,cca,d_opn_set_us,direct_us,x_faster,match"
}

pub fn benchmark_row_to_csv(row: &BenchmarkRow) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
