use std::process::{Command, Output};
use std::sync::{Mutex, OnceLock};

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tt-graph-cdfa-rust"))
}

fn run_binary(args: &[&str]) -> Output {
    static CLI_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = CLI_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    binary().args(args).output().unwrap()
}

#[test]
fn help_lists_primary_commands() {
    let output = run_binary(&["--help"]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("TT-Graph based Concurrent Data Flow Anomaly"));
    assert!(stdout.contains("export-json"));
    assert!(stdout.contains("bench-corpus"));
    assert!(stdout.contains("diagnostics-cpp"));
}

#[test]
fn unknown_command_exits_with_usage_error() {
    let output = run_binary(&["unknown-command"]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unrecognized subcommand"));
    assert!(stderr.contains("Usage:"));
}

#[test]
#[cfg(feature = "clang")]
fn analyze_cpp_missing_insert_node_exits_with_clear_error() {
    let output = run_binary(&[
        "analyze-cpp",
        "examples/paper_program1/program1.cpp",
        "insert",
        "MissingAct",
        "v",
        "Write",
    ]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("missing TT Graph node `MissingAct`"));
    assert!(!stderr.to_ascii_lowercase().contains("panic"));
}

#[test]
#[cfg(feature = "clang")]
fn analyze_cpp_invalid_operation_exits_with_usage_error() {
    let output = run_binary(&[
        "analyze-cpp",
        "examples/paper_program1/program1.cpp",
        "insert",
        "Act2",
        "v",
        "BadOp",
    ]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unknown operation `BadOp`"));
    assert!(!stderr.to_ascii_lowercase().contains("panic"));
}

#[test]
#[cfg(feature = "clang")]
fn analyze_cpp_missing_insert_args_exits_with_usage_error() {
    let output = run_binary(&[
        "analyze-cpp",
        "examples/paper_program1/program1.cpp",
        "insert",
        "Act2",
        "v",
    ]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("usage: cargo run -- analyze-cpp"));
    assert!(!stderr.to_ascii_lowercase().contains("panic"));
}

#[test]
#[cfg(feature = "clang")]
fn analyze_cpp_valid_insert_succeeds() {
    let output = run_binary(&[
        "analyze-cpp",
        "examples/paper_program1/program1.cpp",
        "insert",
        "Act2",
        "v",
        "Write",
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("Matches direct scan: true"));
    assert!(stdout.contains("After insertion: Write(v) into Act2"));
    assert!(!stderr.to_ascii_lowercase().contains("panic"));
}

#[test]
fn no_command_keeps_demo_as_default() {
    let output = run_binary(&[]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("TT Graph CDFA Rust demo"));
    assert!(stdout.contains("Insertion: Write(v) into Act2"));
}

#[test]
#[cfg(feature = "clang")]
fn export_json_outputs_parseable_schema() {
    let output = run_binary(&["export-json", "examples/paper_program1/program1.cpp"]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["schema_version"], 1);
    assert!(parsed["nodes"].as_array().unwrap().len() >= 10);
    assert!(parsed["d_opn_set"].as_array().unwrap().len() >= 5);
    assert!(parsed["cca_sets"].as_array().is_some());
}

#[test]
#[cfg(feature = "clang")]
fn diagnostics_cpp_error_outputs_parseable_json() {
    let output = run_binary(&["diagnostics-cpp", "examples/missing.cpp"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["schema_version"], 1);
    assert_eq!(parsed["source"]["path"], "examples/missing.cpp");
    assert_eq!(parsed["source"]["language"], "cpp");
    assert!(
        parsed["error"]
            .as_str()
            .unwrap()
            .contains("examples/missing.cpp")
    );
    assert!(parsed["diagnostics"].as_array().unwrap().is_empty());
}
