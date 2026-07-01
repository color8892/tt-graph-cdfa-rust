# TT Graph CDFA Rust Implementation

[![CI](https://github.com/color8892/tt-graph-cdfa-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/color8892/tt-graph-cdfa-rust/actions/workflows/ci.yml)

This repository is an independent Rust implementation and reproduction of the algorithms and concepts proposed in the research paper:

> **An Approach to Incrementally Detecting Concurrent Dataflow Anomalies in Software Development**
> *Authors: Koko Harianto, Feng-Jian Wang, Stephen S. Yau, Mohit B Badiyani, William Cheng-Chung Chu*
> *Published in: COMPSAC 2025*

**Disclaimer**: The authors of this codebase are not the authors of the paper. This repository is created solely for algorithmic reproduction, performance benchmarking, and educational verification of the paper's `d_OPN_set` strategy.

The graph model is structured as a reusable Rust library.

Question:

Can the paper's `d_OPN_set` branch summary be implemented as a typed Rust graph
model, and does it reproduce direct concurrent dataflow anomaly detection while
running faster on synthetic full binary AND graphs?

## Quickstart

Install a recent Rust toolchain and, for the default C++ frontend, install LLVM
with `libclang` available. On Windows, set `LIBCLANG_PATH` to the folder that
contains `libclang.dll`.

```powershell
cargo test
cargo run -- demo
cargo run -- paper
cargo run -- cpp examples/paper_program1/program1.cpp
```

For a zero-libclang library build, use:

```powershell
cargo test --no-default-features
```

## Common Commands

Run the paper demo:

```powershell
cargo run -- demo
```

Run the paper reproduction harness:

```powershell
cargo run -- paper
```

Run the C++ Clang reproduction of the paper's Program 1:

```powershell
cargo run -- cpp examples/paper_program1/program1.cpp
```

Run generic C++ analysis, optionally with an insertion:

```powershell
cargo run -- analyze-cpp examples/paper_program1/program1.cpp
cargo run -- analyze-cpp examples/paper_program1/program1.cpp insert Act2 v Write
```

Export a JSON artifact containing the parsed TT Graph, `d_OPN_set` rows, and
CCA sets:

```powershell
cargo run --quiet -- export-json examples/paper_program1/program1.cpp > reproduction.json
```

Export editor diagnostics JSON for the VS Code prototype:

```powershell
cargo run --quiet -- diagnostics-cpp examples/paper_program1/program1.cpp
cargo run --quiet -- diagnostics-cpp-implicit examples/paper_program1/program1_plain.cpp
```

Export a paper Program 2 JSON artifact after inserting `Write(v)` into `Act2`:

```powershell
cargo run --quiet -- export-paper-json examples/paper_program1/program1.cpp > reproduction-after-insertion.json
```

Run the deletion demo:

```powershell
cargo run -- delete-demo
```

Export a Graphviz DOT view of the paper graph after insertion:

```powershell
cargo run -- dot > tt_graph.dot
```

Run the benchmark:

```powershell
cargo run --release -- bench
```

Export benchmark results as CSV:

```powershell
cargo run --quiet --release -- bench-csv > benchmark.csv
```

Run tests:

```powershell
cargo test
```

Expected high-level outputs:

- `demo` prints the Program 2 insertion and reports that summary detection
  matches direct scan.
- `paper` prints the paper reproduction tables, CCA entries, and deletion
  extension check.
- `export-json` prints a JSON artifact with `schema_version`, `nodes`,
  `d_opn_set`, and `cca_sets`.
- `diagnostics-cpp` prints editor diagnostics JSON; parse errors are also
  reported as JSON on stdout with a non-zero exit code.

The default build uses the `clang` feature (libclang) for the C++ frontend.
Disable it with `cargo build --no-default-features` to keep the core library
available without the libclang-dependent C++ frontend.

C++ frontend prerequisites:

- LLVM/Clang with `libclang` available on the machine
- Windows: install [LLVM](https://releases.llvm.org/) and set `LIBCLANG_PATH`
  to the directory containing `libclang.dll`
- Linux: `sudo apt install libclang-dev clang`

## Reproduction and Benchmark Notes

- Run reproduction commands from the repository root so relative example paths
  resolve correctly.
- Benchmark numbers depend on CPU, power settings, compiler version, and build
  mode. Use `cargo run --release -- bench-corpus` or `bench-csv` for repeatable
  machine-readable runs.
- JSON artifact schemas are intended to be stable for downstream scripts and
  the VS Code prototype. Add regression tests when changing JSON fields.

## Release and Publishing

Tagged releases use the `v*.*.*` tag pattern. The release workflow validates the
crate, packages it with `cargo package`, and attaches the `.crate` artifact to a
GitHub release.

Publishing to crates.io is intentionally manual. Run the release workflow with
`publish_crate=true` from GitHub Actions and configure `CARGO_REGISTRY_TOKEN` in
the `crates-io` environment before publishing.

Current implemented scope:

- insertion-only detection from the paper's main algorithm
- C++ reconstruction via libclang from `examples/paper_program1/program1.cpp` (OpenMP parallel
  sections to AND; legacy `#pragma tt` still supported)
- implicit C++ reconstruction from `examples/paper_program1/program1_plain.cpp` (`std::thread` +
  `printf`/`free`, no TT markers) via `cargo run -- cpp-implicit`
- paper Figure 1/3-6 and Table 1/5/6 CLI (`cargo run -- figure4`, `table5`, etc.)
- expanded benchmark corpus (`cargo run --release -- bench-corpus`)
- deletion support as a prototype extension
- `d_OPN_set` strategy and direct scan baseline
- nested AND/XOR/LOOP correctness tests against direct scan
- synthetic full binary AND benchmark
- Graphviz DOT export
- JSON artifact export for graph summaries and CCA sets
- editor diagnostics JSON export for the VS Code IDE prototype
- VS Code prototype extension with Problems diagnostics and SVG TT Graph view
- golden JSON fixture coverage for exporter regressions
- CSV benchmark export

See `PAPER_REPRODUCTION.md` for the reproduction protocol and known paper
typos/ambiguities handled by the implementation.

See `docs/paper-mapping.md` for a Figure/Table/Algorithm to code/test/CLI mapping.

See `docs/artifact-schema.md` for the JSON artifact schema.

## IDE Prototype

The `vscode-extension/` folder contains a VS Code prototype that calls the Rust
analyzer binary, publishes CCA diagnostics to the Problems panel, and renders a
TT Graph SVG webview. Build the Rust binary first:

```powershell
cargo build
```

Then follow `vscode-extension/README.md` to install extension dependencies and
run the Extension Development Host. The extension currently targets the scoped
C++ paper examples and is not a full multi-file C++ language server.

This crate reproduces the paper's core algorithms and running example. The
VS Code extension is a first IDE/SDE prototype for the scoped paper examples,
not a complete multi-file C++ language-server product. Remaining gaps are
listed in `docs/paper-mapping.md`.
