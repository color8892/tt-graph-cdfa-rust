# Paper Reproduction Protocol

This crate is intended to reproduce the algorithmic content of
`COMPSAC_YAU_final.pdf`, not just demonstrate a similar idea.

Run the full paper reproduction harness:

```powershell
cargo run -- paper
```

Run the C++ Clang reproduction of Program 1:

```powershell
cargo run -- cpp examples/paper_program1/program1.cpp
```

Run the implicit C++ reproduction (no OpenMP / `#pragma tt` / `tt_print`):

```powershell
cargo run -- cpp-implicit examples/paper_program1/program1_plain.cpp
```

Reproduce paper figures and extra tables:

```powershell
cargo run -- figure4
cargo run -- table5
cargo run --release -- bench-corpus
```

Run generic C++ analysis with an insertion:

```powershell
cargo run -- analyze-cpp examples/paper_program1/program1.cpp insert Act2 v Write
```

Export the parsed reproduction as a JSON artifact:

```powershell
cargo run --quiet -- export-json examples/paper_program1/program1.cpp > reproduction.json
```

Export the parsed reproduction after applying the paper's Program 2 insertion:

```powershell
cargo run --quiet -- export-paper-json examples/paper_program1/program1.cpp > reproduction-after-insertion.json
```

C++ syntax (`examples/paper_program1/program1.cpp`) is parsed by libclang. Parallel AND
structure is inferred from OpenMP `#pragma omp parallel sections` / `#pragma omp
section` regions (no `#pragma tt` required). Each section body is recovered from
source text when libclang exposes the OpenMP region as an opaque AST node, then
re-parsed for CFG extraction. Dataflow operations use helper calls
`tt_print(...)` / `tt_kill(...)` plus normal assignment / `while` / `if` AST
nodes.

```cpp
void program1() {
#pragma omp parallel sections
  {
#pragma omp section
    { tt_print(v); v = 10; while (i < 20) { ... } }
#pragma omp section
    { tt_print(v); v = 1000; if (v % 2 == 0) { ... } else { ... } }
  }
}
```

Legacy `#pragma tt parallel` / `#pragma tt branch` scaffolding
(`examples/paper_program1/program1_pragma.cpp`) remains supported for older inputs.

The `paper` command reproduces these parts of the paper:

- Program 1 / Figure 2: the hardcoded TT Graph for the running example.
- Table 2: `d_OPN_set` records for BLOCK nodes B1 through B5.
- Program 2: insertion of `Write(v)` into `Act2`.
- Algorithm 1: summary-based detection using `d_OPN_set`.
- Algorithm 2: direct-scan baseline over reachable sibling BLOCK nodes.
- Table 3: newly constructed anomalies caused by the insertion.
- Table 4: updated `d_OPN_set(v, Write, B1) = {Act1, Act2}`.

Correctness checks:

```powershell
cargo test
```

The tests lock down:

- Table 2 `d_OPN_set` contents.
- C++ Clang parser output for `examples/paper_program1/program1.cpp` against the hardcoded
  Program 1 graph.
- Program 2 insertion results.
- Algorithm 1 / Algorithm 2 equivalence.
- Nested AND/XOR/LOOP summary propagation against direct scan.
- Deletion extension behavior against a full recomputation baseline.
- JSON export of nodes, `d_OPN_set`, and CCA sets after insertion, including a
  golden fixture regression test.
- Versioned JSON artifact schema in `docs/artifact-schema.md`.
- Figure/Table/Algorithm mapping in `docs/paper-mapping.md`.

Known paper issues handled by this reproduction:

- The paper text sometimes writes `CCA_W&D`; this implementation treats it as
  a typo and uses the defined `CCA_W&R` / `CCA_W&K` sets.
- The paper's anomaly tuple ordering is not always explicit when the inserted
  operation is `Read` or `Kill`. This implementation normalizes tuple order so
  `WriteRead(v, a, b)` always stores the Write node first and the Read node
  second.
- Deletion is future work in the paper. This crate implements a conservative,
  verified deletion extension by removing the operation from ancestor
  `d_OPN_set` records, recomputing affected AND-node CCA sets, and checking
  selected deletion paths against a full recomputation baseline.

Out of scope for the current reproduction:

- Parsing arbitrary C++ without OpenMP parallel-section markers or
  `tt_print` / `tt_kill` hooks. The Clang frontend still needs explicit
  concurrency scaffolding (OpenMP sections or legacy `#pragma tt`) plus
  dataflow helper calls.

- IDE / SDE integration.
- Persistence beyond plain JSON / DOT / CSV artifacts.
- A faithful reimplementation of the paper's exact formatting errors.

See `docs/paper-mapping.md` for the full artifact index and explicit gap list.
