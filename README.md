# TT Graph CDFA Rust

[![CI](https://github.com/color8892/tt-graph-cdfa-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/color8892/tt-graph-cdfa-rust/actions/workflows/ci.yml)

Rust implementation of the TT Graph concurrent dataflow anomaly detection
approach from the COMPSAC 2025 paper, with CLI reproduction commands,
benchmarking, JSON export, and a prototype VS Code integration.

This repository is an independent reproduction. It is not authored by the
paper authors and is intended for algorithmic verification, education, and
tooling experiments around the paper's `d_OPN_set` strategy.

## What This Repo Does

The code models Task-Transaction Graphs as typed Rust data structures, maintains
per-BLOCK `d_OPN_set` summaries, and compares incremental summary-based
detection against a direct-scan baseline. The included C++ frontend reconstructs
the paper examples from scoped C++ inputs and exports diagnostics for editor
prototypes.

## Quickstart

Install a recent Rust toolchain. The default build enables the C++ frontend, so
LLVM/libclang must be available. On Windows, set `LIBCLANG_PATH` to the folder
that contains `libclang.dll`.

```powershell
cargo test
cargo run -- paper
cargo run -- diagnostics-cpp examples/paper_program1/program1.cpp
cargo run -- export-json examples/paper_program1/program1.cpp
```

For the core Rust library without libclang:

```powershell
cargo test --no-default-features
```

## Common Commands

```powershell
cargo run -- demo
cargo run -- paper
cargo run -- cpp examples/paper_program1/program1.cpp
cargo run -- analyze-cpp examples/paper_program1/program1.cpp insert Act2 v Write
cargo run -- diagnostics-cpp examples/paper_program1/program1.cpp
cargo run -- export-json examples/paper_program1/program1.cpp
cargo run --release -- bench-corpus --csv
```

Expected outputs:

- `paper` prints the reproduction tables, CCA entries, and deletion check.
- `diagnostics-cpp` prints editor diagnostics JSON and exits non-zero on parse errors.
- `export-json` prints a stable JSON artifact with nodes, `d_opn_set`, and CCA sets.
- `bench-corpus --csv` prints machine-readable benchmark rows.

## Current Scope

- Paper Program 1 reconstruction and Program 2 insertion check.
- Incremental `d_OPN_set` strategy plus direct-scan comparison.
- Nested AND/XOR/LOOP correctness tests and synthetic AND benchmarks.
- C++ frontend for scoped paper examples, including an implicit `std::thread` variant.
- JSON artifact and diagnostics export for downstream tooling.
- Verified deletion extension for the paper's future-work direction.
- VS Code prototype with Problems diagnostics and Mermaid webview graph.

## Deletion Extension

The paper lists deletion as future work. This crate implements a conservative
extension for deleting an operation from a TT Graph node:

- remove the operation from the target node;
- propagate the removal through ancestor BLOCK `d_OPN_set` summaries;
- recompute CCA sets for affected AND controls;
- verify selected deletion paths against a full recomputation baseline.

The implementation is intentionally framed as an engineering extension, not as a
new published algorithm proof. Tests cover the paper Program 2 deletion back to
Program 1, missing-operation no-op behavior, and nested LOOP/XOR/AND deletion
against full recomputation.

## Documentation

- [PAPER_REPRODUCTION.md](PAPER_REPRODUCTION.md): reproduction protocol and expected outputs.
- [docs/paper-mapping.md](docs/paper-mapping.md): paper figure/table/algorithm mapping to code, tests, and CLI.
- [BENCHMARK.md](BENCHMARK.md): benchmark commands, cases, and interpretation notes.
- [docs/artifact-schema.md](docs/artifact-schema.md): JSON artifact schema and versioning rules.
- [CONTRIBUTING.md](CONTRIBUTING.md): development checks and release notes.
- [vscode-extension/README.md](vscode-extension/README.md): extension setup, commands, and limitations.

## IDE Prototype

The `vscode-extension/` folder contains a VS Code prototype that calls the Rust
analyzer binary, publishes CCA diagnostics to the Problems panel, and renders a
Mermaid TT Graph webview. Build the Rust binary first:

```powershell
cargo build
```

Then follow [vscode-extension/README.md](vscode-extension/README.md). The
extension targets the scoped paper examples and is not a full multi-file C++
language server.
