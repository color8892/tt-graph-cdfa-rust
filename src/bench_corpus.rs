use std::time::{Duration, Instant};

use crate::{
    OperationType, SyntheticGraphCase, TTGraph, build_chain_and_graph, build_paper_example_graph,
    build_synthetic_full_and_graph,
};

#[derive(Debug, Clone)]
pub struct CorpusRow {
    pub case_id: String,
    pub node_count: usize,
    pub leaf_count: usize,
    pub matching_leaf_count: usize,
    pub summary_median_us: f64,
    pub direct_median_us: f64,
    pub speedup: f64,
    pub matches_direct_scan: bool,
}

pub fn corpus_cases() -> Vec<CorpusCase> {
    let mut cases = vec![CorpusCase::PaperProgram1];
    for depth in [4, 6, 8, 10, 12] {
        cases.push(CorpusCase::Figure6 {
            depth,
            matching_stride: 16,
        });
        cases.push(CorpusCase::ChainAnd {
            depth,
            matching_stride: 16,
        });
    }
    cases.push(CorpusCase::PlainCpp);
    cases
}

#[derive(Debug, Clone)]
pub enum CorpusCase {
    PaperProgram1,
    Figure6 {
        depth: usize,
        matching_stride: usize,
    },
    ChainAnd {
        depth: usize,
        matching_stride: usize,
    },
    PlainCpp,
}

impl CorpusCase {
    pub fn id(&self) -> String {
        match self {
            Self::PaperProgram1 => "paper_program1".to_string(),
            Self::Figure6 { depth, .. } => format!("figure6_depth_{depth}"),
            Self::ChainAnd { depth, .. } => format!("chain_and_depth_{depth}"),
            Self::PlainCpp => "plain_cpp".to_string(),
        }
    }

    pub fn materialize(&self) -> Result<MaterializedCase, String> {
        match self {
            Self::PaperProgram1 => {
                let graph = build_paper_example_graph();
                Ok(MaterializedCase {
                    case_id: self.id(),
                    graph,
                    target_node_id: "Act2".to_string(),
                    node_count: 15,
                    leaf_count: 5,
                    matching_leaf_count: 1,
                })
            }
            Self::Figure6 {
                depth,
                matching_stride,
            } => {
                let case = build_synthetic_full_and_graph(*depth, *matching_stride);
                Ok(materialized_from_synthetic(self.id(), case))
            }
            Self::ChainAnd {
                depth,
                matching_stride,
            } => {
                let case = build_chain_and_graph(*depth, *matching_stride);
                Ok(materialized_from_synthetic(self.id(), case))
            }
            Self::PlainCpp => {
                #[cfg(feature = "clang")]
                {
                    let graph = crate::clang_frontend::parse_cpp_implicit_file(
                        "examples/program1_plain.cpp",
                    )?;
                    Ok(MaterializedCase {
                        case_id: self.id(),
                        graph,
                        target_node_id: "Act2".to_string(),
                        node_count: 15,
                        leaf_count: 5,
                        matching_leaf_count: 1,
                    })
                }
                #[cfg(not(feature = "clang"))]
                {
                    Err("plain_cpp corpus requires the clang feature".to_string())
                }
            }
        }
    }
}

pub struct MaterializedCase {
    pub case_id: String,
    pub graph: TTGraph,
    pub target_node_id: String,
    pub node_count: usize,
    pub leaf_count: usize,
    pub matching_leaf_count: usize,
}

fn materialized_from_synthetic(case_id: String, case: SyntheticGraphCase) -> MaterializedCase {
    MaterializedCase {
        case_id,
        graph: case.graph,
        target_node_id: case.target_node_id,
        node_count: case.node_count,
        leaf_count: case.leaf_count,
        matching_leaf_count: case.matching_leaf_count,
    }
}

pub fn bench_corpus_rows(iterations: usize) -> Vec<CorpusRow> {
    corpus_cases()
        .into_iter()
        .filter_map(|case| case.materialize().ok())
        .map(|materialized| bench_materialized(&materialized, iterations))
        .collect()
}

fn bench_materialized(case: &MaterializedCase, iterations: usize) -> CorpusRow {
    let (variable, op) = if matches!(case.case_id.as_str(), "paper_program1" | "plain_cpp") {
        ("v", OperationType::Write)
    } else {
        ("target", OperationType::Write)
    };

    let mut summary_times = Vec::with_capacity(iterations);
    let mut direct_times = Vec::with_capacity(iterations);
    let mut matches = true;

    for _ in 0..iterations {
        let mut summary_graph = case.graph.clone();
        let start = Instant::now();
        let summary =
            summary_graph.insert_operation_summary_only(&case.target_node_id, variable, op);
        summary_times.push(start.elapsed());

        let mut direct_graph = case.graph.clone();
        let start = Instant::now();
        let direct = direct_graph.insert_operation_direct_only(&case.target_node_id, variable, op);
        direct_times.push(start.elapsed());

        matches &= summary.entries == direct.entries;
    }

    let summary_median = median_duration(&summary_times);
    let direct_median = median_duration(&direct_times);
    let speedup = if summary_median.as_nanos() == 0 {
        f64::INFINITY
    } else {
        direct_median.as_secs_f64() / summary_median.as_secs_f64()
    };

    CorpusRow {
        case_id: case.case_id.clone(),
        node_count: case.node_count,
        leaf_count: case.leaf_count,
        matching_leaf_count: case.matching_leaf_count,
        summary_median_us: summary_median.as_secs_f64() * 1_000_000.0,
        direct_median_us: direct_median.as_secs_f64() * 1_000_000.0,
        speedup,
        matches_direct_scan: matches,
    }
}

fn median_duration(samples: &[Duration]) -> Duration {
    assert!(!samples.is_empty(), "median requires at least one sample");
    let mut sorted = samples.to_vec();
    sorted.sort();
    sorted[sorted.len() / 2]
}

pub fn corpus_csv_header() -> &'static str {
    "case_id,node_count,leaf_count,matching_leaf_count,summary_us,direct_us,speedup,match"
}

pub fn corpus_row_to_csv(row: &CorpusRow) -> String {
    format!(
        "{},{},{},{},{:.1},{:.1},{:.1},{}",
        row.case_id,
        row.node_count,
        row.leaf_count,
        row.matching_leaf_count,
        row.summary_median_us,
        row.direct_median_us,
        row.speedup,
        row.matches_direct_scan
    )
}

pub fn print_table5_comparison(rows: &[CorpusRow]) {
    crate::figures::print_table5();
    println!();
    println!("Empirical trend on synthetic/full-binary cases (median insertion detection):");
    for row in rows
        .iter()
        .filter(|row| row.case_id.starts_with("figure6_"))
    {
        println!(
            "  {}: nodes={} speedup={:.1}x match={}",
            row.case_id, row.node_count, row.speedup, row.matches_direct_scan
        );
    }
}
