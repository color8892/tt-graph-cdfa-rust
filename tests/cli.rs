use std::process::Command;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tt-graph-cdfa-rust"))
}

#[test]
fn help_lists_primary_commands() {
    let output = binary().arg("--help").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("TT-Graph based Concurrent Data Flow Anomaly"));
    assert!(stdout.contains("export-json"));
    assert!(stdout.contains("bench-corpus"));
    assert!(stdout.contains("diagnostics-cpp"));
}

#[test]
fn unknown_command_exits_with_usage_error() {
    let output = binary().arg("unknown-command").output().unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unrecognized subcommand"));
    assert!(stderr.contains("Usage:"));
}

#[test]
fn no_command_keeps_demo_as_default() {
    let output = binary().output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("TT Graph CDFA Rust demo"));
    assert!(stdout.contains("Insertion: Write(v) into Act2"));
}

#[test]
#[cfg(feature = "clang")]
fn export_json_outputs_parseable_schema() {
    let output = binary()
        .args(["export-json", "examples/paper_program1/program1.cpp"])
        .output()
        .unwrap();

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
    let output = binary()
        .args(["diagnostics-cpp", "examples/missing.cpp"])
        .output()
        .unwrap();

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
