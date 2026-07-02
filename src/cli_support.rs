use crate::diagnostics::{error_diagnostics_to_json, graph_diagnostics_to_json};
use crate::export::graph_to_json;
use crate::{CcaEntry, CcaType, OperationType, TTGraph, build_paper_example_graph, format_helper};
use std::collections::BTreeSet;
use std::fmt::Write as _;

pub struct CommandOutput {
    pub stdout: String,
    pub exit_code: i32,
}

impl CommandOutput {
    pub fn success(stdout: String) -> Self {
        Self {
            stdout,
            exit_code: 0,
        }
    }

    pub fn failure(stdout: String) -> Self {
        Self {
            stdout,
            exit_code: 1,
        }
    }
}

pub fn demo_output() -> String {
    let mut graph = build_paper_example_graph();
    let result = graph.insert_operation("Act2", "v", OperationType::Write);

    let mut output = String::new();
    let _ = writeln!(output, "TT Graph CDFA Rust demo");
    let _ = writeln!(output, "Insertion: Write(v) into Act2");
    let _ = writeln!(
        output,
        "Summary matches direct scan: {}",
        result.matches_direct_scan()
    );
    let _ = writeln!(output, "Touched AND nodes: {:?}", result.touched_and_nodes);
    let _ = writeln!(
        output,
        "Updated BLOCK summaries: {:?}",
        result.summary_blocks_updated
    );
    let _ = writeln!(output);
    let _ = writeln!(output, "New anomalies from d_OPN_set:");
    push_entries(&mut output, &result.summary_entries);
    let _ = writeln!(output);
    let _ = writeln!(output, "B1 d_OPN_set rows after insertion:");
    push_d_opn_rows(&mut output, &graph, "B1");
    output
}

pub fn delete_demo_output() -> String {
    let mut graph = build_paper_example_graph();
    let insert_result = graph.insert_operation("Act2", "v", OperationType::Write);
    let verification =
        graph.delete_operation_with_recompute_check("Act2", "v", OperationType::Write);
    let delete_result = &verification.deletion;

    let mut output = String::new();
    let _ = writeln!(output, "TT Graph CDFA Rust deletion demo");
    let _ = writeln!(
        output,
        "Setup insertion produced {} CCA entries",
        insert_result.summary_entries.len()
    );
    let _ = writeln!(output, "Deletion: remove Write(v) from Act2");
    let _ = writeln!(
        output,
        "Removed operation: {}",
        delete_result.removed_operation
    );
    let _ = writeln!(
        output,
        "Matches recomputed baseline: {}",
        verification.matches_recomputed_state()
    );
    let _ = writeln!(
        output,
        "Touched AND nodes: {:?}",
        delete_result.touched_and_nodes
    );
    let _ = writeln!(
        output,
        "Updated BLOCK summaries: {:?}",
        delete_result.summary_blocks_updated
    );
    let _ = writeln!(output);
    let _ = writeln!(output, "And1 anomalies after deletion:");
    let mut cca_types: Vec<CcaType> = graph.nodes["And1"].cca_sets.keys().copied().collect();
    cca_types.sort();
    for cca_type in cca_types {
        let _ = writeln!(
            output,
            "  {cca_type:?}: {}",
            graph.nodes["And1"].cca_sets[&cca_type].len()
        );
    }
    let _ = writeln!(output);
    let _ = writeln!(output, "B1 d_OPN_set rows after deletion:");
    push_d_opn_rows(&mut output, &graph, "B1");
    output
}

pub fn dot_output() -> String {
    let mut graph = build_paper_example_graph();
    graph.insert_operation("Act2", "v", OperationType::Write);
    graph.to_dot()
}

pub fn bench_corpus_output(csv: bool) -> String {
    let rows = crate::bench_corpus::bench_corpus_rows(10);
    let mut output = String::new();

    if csv {
        let _ = writeln!(output, "{}", crate::bench_corpus::corpus_csv_header());
        for row in &rows {
            let _ = writeln!(output, "{}", crate::bench_corpus::corpus_row_to_csv(row));
        }
        return output;
    }

    let _ = writeln!(output, "TT Graph CDFA benchmark corpus");
    let _ = writeln!(output, "cases={}", rows.len());
    for row in &rows {
        let _ = writeln!(
            output,
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
    output
}

pub fn export_json_for_source(path: &str) -> Result<String, String> {
    parse_source_graph(path).map(|graph| graph_to_json(&graph))
}

pub fn export_paper_json_for_source(path: &str) -> Result<String, String> {
    let mut graph = parse_source_graph(path)?;
    if !format_helper::is_paper_program_1_graph(&graph) {
        return Err(
            "export-paper-json expects a Program 1 style graph containing Act2".to_string(),
        );
    }
    graph.insert_operation("Act2", "v", OperationType::Write);
    Ok(graph_to_json(&graph))
}

pub fn diagnostics_json_for_cpp(path: &str, language: &str, implicit: bool) -> CommandOutput {
    #[cfg(feature = "clang")]
    {
        let parsed = if implicit {
            crate::clang_frontend::parse_cpp_implicit_file_with_locations(path)
        } else {
            crate::clang_frontend::parse_cpp_file_with_locations(path)
        };

        match parsed {
            Ok(parsed) => CommandOutput::success(graph_diagnostics_to_json(
                &parsed.graph,
                path,
                language,
                &parsed.source_locations,
            )),
            Err(error) => CommandOutput::failure(error_diagnostics_to_json(path, language, &error)),
        }
    }

    #[cfg(not(feature = "clang"))]
    {
        let error = "C++ Clang frontend is disabled; rebuild with `--features clang`";
        CommandOutput::failure(error_diagnostics_to_json(path, language, error))
    }
}

pub fn parse_source_graph(path: &str) -> Result<TTGraph, String> {
    if path.ends_with(".cpp") || path.ends_with(".cc") || path.ends_with(".cxx") {
        parse_cpp_graph(path)
    } else {
        Err(format!(
            "unsupported source path `{path}`; expected .cpp, .cc, or .cxx"
        ))
    }
}

pub fn parse_cpp_graph(path: &str) -> Result<TTGraph, String> {
    #[cfg(feature = "clang")]
    {
        crate::clang_frontend::parse_cpp_file(path)
            .map_err(|error| format!("failed to parse {path}: {error}"))
    }

    #[cfg(not(feature = "clang"))]
    {
        let _ = path;
        Err("C++ Clang frontend is disabled; rebuild with `--features clang`".to_string())
    }
}

pub fn parse_cpp_implicit_graph(path: &str) -> Result<TTGraph, String> {
    #[cfg(feature = "clang")]
    {
        crate::clang_frontend::parse_cpp_implicit_file(path)
            .map_err(|error| format!("failed to parse {path}: {error}"))
    }

    #[cfg(not(feature = "clang"))]
    {
        let _ = path;
        Err("C++ Clang frontend is disabled; rebuild with `--features clang`".to_string())
    }
}

fn push_entries(output: &mut String, entries: &BTreeSet<(CcaType, CcaEntry)>) {
    for (cca_type, entry) in entries {
        let _ = writeln!(
            output,
            "  {cca_type:?}: ({}, {}, {})",
            entry.variable, entry.first_node, entry.second_node
        );
    }
}

fn push_d_opn_rows(output: &mut String, graph: &TTGraph, block_id: &str) {
    for (variable, op, node_ids) in graph.sorted_d_opn_rows(block_id) {
        let _ = writeln!(
            output,
            "  d_OPN_set({variable}, {op:?}, {block_id}) = {node_ids:?}"
        );
    }
}
