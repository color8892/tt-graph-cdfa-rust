# TT Graph CDFA Rust Prototype

This is a Rust port of the Python TT Graph CDFA prototype for
`COMPSAC_YAU_final.pdf`. It is still prototype code, but the graph model is
structured as a reusable Rust library.

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

Run the pseudo-code reproduction closer to the paper's Program 1:

```powershell
cargo run -- pseudo examples/program1.pseudo
```

Export a JSON artifact containing the parsed TT Graph, `d_OPN_set` rows, and
CCA sets after the Program 2 insertion:

```powershell
cargo run --quiet -- export-json examples/program1.pseudo > reproduction.json
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

The Rust version intentionally uses no external crates. That keeps the artifact
easy to build on a machine with only Cargo installed and avoids package registry
network access.

Current implemented scope:

- insertion-only detection from the paper's main algorithm
- parser-based reconstruction of the paper's Program 1 from `examples/program1.tt`
- pseudo-code reconstruction of the paper's Program 1 from `examples/program1.pseudo`
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
