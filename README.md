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

Run the paper demo:

```powershell
cargo run -- demo
```

Run the paper reproduction harness:

```powershell
cargo run -- paper
```

Run the parser-based reproduction from the toy Program 1 input:

```powershell
cargo run -- parse examples/program1.tt
```

Run the C++ Clang reproduction of the paper's Program 1:

```powershell
cargo run -- cpp examples/program1.cpp
```

Run generic C++ analysis, optionally with an insertion:

```powershell
cargo run -- analyze-cpp examples/program1.cpp
cargo run -- analyze-cpp examples/program1.cpp insert Act2 v Write
```

Run the C-like subset reproduction (no libclang required):

```powershell
cargo run -- c examples/program1.c
```

Run generic C subset analysis:

```powershell
cargo run -- analyze-c examples/program1.c
cargo run -- analyze-c examples/program1.c insert Act2 v Write
```

Run the pseudo-code reproduction closer to the paper's Program 1:

```powershell
cargo run -- pseudo examples/program1.pseudo
```

Run a nested `split / branch / join` parser example:

```powershell
cargo run -- pseudo examples/nested_split.pseudo
```

Run generic pseudo-code analysis, optionally with an insertion:

```powershell
cargo run -- analyze-pseudo examples/nested_split.pseudo
cargo run -- analyze-pseudo examples/nested_split.pseudo insert Act1 x Write
```

Export a JSON artifact containing the parsed TT Graph, `d_OPN_set` rows, and
CCA sets after the Program 2 insertion:

```powershell
cargo run --quiet -- export-json examples/program1.cpp > reproduction.json
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

The default build uses the `clang` feature (libclang) for the C++ frontend.
Disable it with `cargo build --no-default-features` to keep a zero-dependency
core library plus the C subset / pseudo / toy parsers.

C++ frontend prerequisites:

- LLVM/Clang with `libclang` available on the machine
- Windows: install [LLVM](https://releases.llvm.org/) and set `LIBCLANG_PATH`
  to the directory containing `libclang.dll`
- Linux: `sudo apt install libclang-dev clang`

Current implemented scope:

- insertion-only detection from the paper's main algorithm
- parser-based reconstruction of the paper's Program 1 from `examples/program1.tt`
- C++ reconstruction via libclang from `examples/program1.cpp` (OpenMP parallel
  sections → AND; legacy `#pragma tt` still supported)
- implicit C++ reconstruction from `examples/program1_plain.cpp` (`std::thread` +
  `printf`/`free`, no TT markers) via `cargo run -- cpp-implicit`
- paper Figure 1/3–6 and Table 1/5/6 CLI (`cargo run -- figure4`, `table5`, …)
- expanded benchmark corpus (`cargo run --release -- bench-corpus`)
- C-like subset reconstruction of the paper's Program 1 from `examples/program1.c`
- pseudo-code reconstruction of the paper's Program 1 from `examples/program1.pseudo`
- nested `split / branch / join` parsing for structured pseudo programs
- generic pseudo-code analysis CLI with optional insertion detection
- deletion support as a prototype extension
- `d_OPN_set` strategy and direct scan baseline
- nested AND/XOR/LOOP correctness tests against direct scan
- synthetic full binary AND benchmark
- Graphviz DOT export
- JSON artifact export for graph summaries and CCA sets
- golden JSON fixture coverage for exporter regressions
- CSV benchmark export

See `PAPER_REPRODUCTION.md` for the reproduction protocol and known paper
typos/ambiguities handled by the implementation.

See `docs/paper-mapping.md` for a Figure/Table/Algorithm → code/test/CLI mapping.

See `docs/artifact-schema.md` for the JSON artifact schema.

This crate reproduces the paper's core algorithms and running example, not a
full software-development-environment system. Gaps (language frontend, IDE
integration, large-scale benchmarks) are listed in `docs/paper-mapping.md`.
