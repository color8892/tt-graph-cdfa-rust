# Paper Mapping

This document maps artifacts in `COMPSAC_YAU_final.pdf` to code, tests, CLI
commands, and fixtures in this repository.

Use it together with:

- `PAPER_REPRODUCTION.md` — reproduction protocol and known paper ambiguities
- `docs/artifact-schema.md` — JSON export schema

## Reproduction scope

| Layer | Status | Notes |
|-------|--------|-------|
| TT Graph data model | Reproduced | `TTNode`, `TTGraph` in `src/lib.rs` |
| `d_OPN_set` maintenance | Reproduced | `add_d_opn`, `remove_d_opn`, `rebuild_all_d_opn_sets` |
| Algorithm 1 (summary detection) | Reproduced | `insert_operation_summary_only`, `detect_using_d_opn_set` |
| Algorithm 2 (direct scan) | Reproduced | `insert_operation_direct_only`, `detect_by_direct_scan` |
| Program 1 / Figure 2 | Reproduced | Hardcoded + C++ inputs |
| Program 2 insertion | Reproduced | `Write(v)` into `Act2` |
| Tables 2–4 | Reproduced | Locked by unit tests and `cargo run -- paper` |
| Table 1 / 5 / 6 | Reproduced | `cargo run -- table1|table5|table6` |
| Figure 1 / 3–6 | Reproduced | `cargo run -- figure1|figure3|figure4|figure5|figure6` |
| Performance experiment | Partial | `bench`, `bench-corpus` (paper has no empirical timings) |
| Language frontend | Partial | OpenMP / **implicit `std::thread`** (`cpp-implicit`) |
| IDE / SDE integration | Reproduced | CLI + VS Code extension prototype |

---

## Figures

### Figure 1 — TT Graph notation

| Artifact | Location |
|----------|----------|
| CLI printer | `figures::print_figure1` in `src/figures.rs` |

```powershell
cargo run -- figure1
```

### Figure 2 — Program 1 TT Graph

The running-example graph with AND/XOR/LOOP control nodes, BLOCK scopes, and
activity nodes `Act1`–`Act5`.

| Artifact | Location |
|----------|----------|
| Hardcoded graph | `build_paper_example_graph()` in `src/lib.rs` |
| C++ Clang input (OpenMP) | `examples/paper_program1/program1.cpp` → `clang_frontend::parse_cpp_file` |
| C++ implicit input | `examples/paper_program1/program1_plain.cpp` → `clang_frontend::parse_cpp_implicit_file` |
| Graphviz export | `TTGraph::to_dot()` in `src/lib.rs`; CLI: `cargo run -- dot` |
| JSON export | `export::graph_to_json` in `src/export.rs`; CLI: `cargo run -- export-json examples/paper_program1/program1.cpp` |

**Tests**

| Test | File | What it checks |
|------|------|----------------|
| `paper_table_2_d_opn_sets_match_program_1` | `src/lib.rs` | Hardcoded graph matches Table 2 |
| `parses_program1_cpp_into_matching_d_opn_sets` | `src/clang_frontend.rs` | C++ Clang parser → same `d_OPN_set` as hardcoded |
| `parses_program1_plain_cpp_into_matching_d_opn_sets` | `src/clang_frontend.rs` | Implicit C++ (`std::thread`) → same `d_OPN_set` |

**CLI**

```powershell
cargo run -- paper          # prints Figure 2 context before Program 2
cargo run -- cpp examples/paper_program1/program1.cpp
cargo run -- cpp-implicit examples/paper_program1/program1_plain.cpp
cargo run -- dot > tt_graph.dot
```

---

## Tables

### Table 2 — `d_OPN_set` for Program 1 (BLOCK nodes B1–B5)

Per-BLOCK summary records before Program 2 insertion.

| Artifact | Location |
|----------|----------|
| Assertion helper | `assert_d_opn` in `src/lib.rs` tests module |
| CLI printer | `print_paper_table_2` in `src/main.rs` |
| Full row list | `paper_table_2_d_opn_sets_match_program_1` test in `src/lib.rs` |

**Tests**

| Test | File |
|------|------|
| `paper_table_2_d_opn_sets_match_program_1` | `src/lib.rs` |
| `parses_program1_cpp_into_matching_d_opn_sets` | `src/clang_frontend.rs` |

**CLI**

```powershell
cargo run -- paper
cargo run -- cpp examples/paper_program1/program1.cpp
```

---

### Table 3 — Newly constructed anomalies after Program 2 insertion

CCA entries created when `Write(v)` is inserted into `Act2`.

| Artifact | Location |
|----------|----------|
| Detection entry point | `TTGraph::insert_operation` in `src/lib.rs` |
| Summary strategy | `insert_operation_summary_only` → `detect_using_d_opn_set` |
| Direct baseline | `insert_operation_direct_only` → `detect_by_direct_scan` |
| CCA normalization | `normalize_cca_entry`, `related_operations` in `src/lib.rs` |
| CLI printer | `print_entries`, `print_cca_sets` in `src/main.rs` |

**Expected entries (7 new CCA tuples)**

Locked by `paper_write_insertion_matches_direct_scan` in `src/lib.rs`:

- `WriteWrite(v, Act2, Act3)`
- `WriteRead(v, Act2, Act3)`
- `WriteRead(v, Act2, Act4)`
- `WriteRead(v, Act2, Act5)`
- `WriteRead(v, Act2, Xor1)`
- `WriteKill(v, Act2, Act4)`
- `WriteKill(v, Act2, Act5)`

**Tests**

| Test | File |
|------|------|
| `paper_write_insertion_matches_direct_scan` | `src/lib.rs` |
| `parses_nested_compound_statement_without_orphans` | `src/clang_frontend.rs` |
| `exports_nodes_d_opn_sets_and_cca_sets` | `src/export.rs` |
| `tiny_graph_after_insertion_matches_golden_json_fixture` | `src/export.rs` |

**CLI**

```powershell
cargo run -- paper
cargo run -- demo
```

**Fixture**

- `fixtures/tiny_after_insertion.json` — golden JSON for a minimal insertion case

---

### Table 4 — Updated `d_OPN_set` of B1 after insertion

After Program 2, `d_OPN_set(v, Write, B1) = {Act1, Act2}`.

| Artifact | Location |
|----------|----------|
| Summary update | `add_d_opn` called from `insert_operation_summary_only` |
| CLI printer | `print_paper_table_4` in `src/main.rs` |

**Tests**

| Test | File | Assertion |
|------|------|-----------|
| `paper_write_insertion_matches_direct_scan` | `src/lib.rs` | B1 Write row = `{Act1, Act2}` |
| `parsed_program1_cpp_reproduces_program_2_insertion` | `src/clang_frontend.rs` | Same via C++ parser |

**CLI**

```powershell
cargo run -- paper
```

---

## Algorithms

### Algorithm 1 — Incremental detection using `d_OPN_set`

Insert an operation into a TNode, update ancestor BLOCK summaries, detect new
CCAs at ancestor AND nodes using maintained `d_OPN_set` records.

| Step | Function | File |
|------|----------|------|
| Insert op into TNode | `insert_operation_summary_only` | `src/lib.rs` |
| Walk scope chain | loop over `scope_arc` in `insert_operation_summary_only` | `src/lib.rs` |
| Update BLOCK summary | `add_d_opn` | `src/lib.rs` |
| Detect at AND parent | `detect_using_d_opn_set` | `src/lib.rs` |
| Record CCA on AND node | `record_cca` | `src/lib.rs` |
| Public wrapper (runs both strategies) | `insert_operation` | `src/lib.rs` |

**Tests**

| Test | File |
|------|------|
| `paper_write_insertion_matches_direct_scan` | `src/lib.rs` |
| `read_insertion_keeps_write_read_tuple_order` | `src/lib.rs` |
| `synthetic_graph_summary_matches_direct_scan` | `src/lib.rs` |
| `nested_loop_xor_and_graph_summary_matches_direct_scan` | `src/lib.rs` |

**CLI**

```powershell
cargo run -- paper
cargo run -- analyze-cpp examples/paper_program1/program1.cpp insert Act2 v Write
```

---

### Algorithm 2 — Direct-scan baseline

Scan reachable sibling BLOCK nodes and compare operation sequences directly.

| Step | Function | File |
|------|----------|------|
| Insert op into TNode | `insert_operation_direct_only` | `src/lib.rs` |
| Enumerate sibling blocks | `other_blocks` | `src/lib.rs` |
| Reachable NOP nodes | `reachable_nop_nodes` | `src/lib.rs` |
| Compare operations | `detect_by_direct_scan` | `src/lib.rs` |
| Equivalence check | `DetectionResult::matches_direct_scan` | `src/lib.rs` |

**Tests**

Same as Algorithm 1 — every insertion test asserts
`result.matches_direct_scan()`.

**CLI**

```powershell
cargo run -- paper    # prints both Algorithm 1 and Algorithm 2 results
```

### Supporting definitions (paper terminology → code)

| Paper concept | Code |
|---------------|------|
| TNode | `TTNode` |
| NOP node (Activity or Control) | `TTNode::is_nop_node()` |
| BLOCK node | `NodeType::Block` |
| AND / XOR / LOOP control | `ControlType::{And, Xor, Loop}` |
| Operation (Read/Write/Kill) | `Operation`, `OperationType` |
| CCA types (W&W, W&R, W&K, R&K) | `CcaType::{WriteWrite, WriteRead, WriteKill, ReadKill}` |
| `d_OPN_set(v, op, B)` | `TTNode.d_opn_set: HashMap<(String, OperationType), HashSet<String>>` |
| CCA set on AND node | `TTNode.cca_sets` |
| Sequence arc | `sequence_arc` |
| Branch arc | `branch_arc` |
| Scope arc | `scope_arc` |

---

## Programs

### Program 1

Paper's initial running example (Figure 2, Table 2).

| Input form | Path | Parser |
|------------|------|--------|
| Hardcoded | — | `build_paper_example_graph()` |
| C++ Clang (OpenMP) | `examples/paper_program1/program1.cpp` | `clang_frontend::parse_cpp_file` |

### Program 2

Insert `Write(v)` into `Act2`.

| Artifact | Location |
|----------|----------|
| Next step | `graph.insert_operation("Act2", "v", OperationType::Write)` |
| JSON export after insertion | `run_export_paper_json` in `src/main.rs` applies Program 2 before export |

---

## Experiments and performance

The paper's performance evaluation is only partially reproduced.

| Paper claim | Reproduction | Location |
|-------------|--------------|----------|
| Summary faster than direct scan on large graphs | Trend demonstrated | `bench_depth` in `src/main.rs` |
| Equivalence under benchmark load | Asserted per iteration | `matches_direct_scan` in `benchmark_rows` |
| Exact paper timings / datasets | Not reproduced | Synthetic graphs only |

| Artifact | Location |
|----------|----------|
| Synthetic graph builder | `build_synthetic_full_and_graph` in `src/lib.rs` |
| Benchmark driver | `run_benchmark`, `benchmark_rows` in `src/main.rs` |
| CSV export | `run_benchmark_csv`, `benchmark_row_to_csv` in `src/main.rs` |

**Parameters (current)**

- Depths: 4, 6, 8
- Iterations: 20
- Matching stride: 16
- Insertion: `Write(target)` into a fixed leaf activity

**Tests**

| Test | File |
|------|------|
| `synthetic_graph_summary_matches_direct_scan` | `src/lib.rs` |
| `benchmark_csv_row_is_machine_readable` | `src/main.rs` |

**CLI**

```powershell
cargo run --release -- bench
cargo run --quiet --release -- bench-csv > benchmark.csv
```

**CI**

- `.github/workflows/ci.yml` — `bench-csv` smoke test on every push/PR

---

## Extensions beyond the paper

These are implemented but not part of the original paper's core contribution.

| Feature | Location | Tests |
|---------|----------|-------|
| Deletion of inserted operations | `delete_operation` and `delete_operation_with_recompute_check` in `src/lib.rs` | `delete_inserted_operation_updates_summaries_and_cca_sets`, `deletion_matches_recomputed_baseline_for_nested_control_flow`, `deletion_verification_missing_operation_matches_recomputed_noop` |
| Nested LOOP/XOR/AND correctness | `build_nested_control_flow_graph` in `src/lib.rs` tests | `nested_loop_xor_and_graph_summary_matches_direct_scan` |
| JSON artifact export | `src/export.rs` | `exports_*`, golden fixture test |
| Graphviz DOT export | `TTGraph::to_dot` | manual via `cargo run -- dot` |

---

## Explicit gaps (not mapped to paper artifacts)

These items are called out in `PAPER_REPRODUCTION.md` as out of scope. No
paper Figure/Table/Algorithm maps to them yet.

| Gap | Current C++ Support | Future |
|-----|--------------------|--------|
| Implicit parallel from arbitrary C++ | `cpp-implicit` supports `std::thread` + printf/assign/free only | pthread, `std::async`, CFG-only inference |
| IDE / SDE integration | VS Code prototype in `vscode-extension` | Editor plugin, LSP, file watcher |
| Large real-world benchmark corpus | Paper has no timing table; repo corpus is synthetic + Program 1 | Linux-kernel-scale programs, published CSV |
| Production-grade validation | Library APIs use `expect` on invalid input | `Result`-based API + `validate_graph` |
| Persistence | Paper may assume tool integration | Beyond JSON/DOT/CSV artifacts |

---

## Verification checklist

Run these commands to audit the mapping end-to-end:

```powershell
# Unit tests (all paper-locked assertions)
cargo test

# Interactive paper walkthrough (Figure 2 → Table 2 → Program 2 → Tables 3–4)
cargo run -- paper

# Parser paths into the same graph
cargo run -- cpp examples/paper_program1/program1.cpp
cargo run -- cpp-implicit examples/paper_program1/program1_plain.cpp
cargo run -- figure4
cargo run --release -- bench-corpus

# Generic analysis path
cargo run -- analyze-cpp examples/paper_program1/program1.cpp insert Act2 v Write

# Machine-readable artifacts
cargo run --quiet -- export-json examples/paper_program1/program1.cpp > reproduction.json
cargo run --quiet -- export-paper-json examples/paper_program1/program1.cpp > reproduction-after-insertion.json
cargo run --release -- bench
cargo run --quiet --release -- bench-csv > benchmark.csv
```

Expected: the full test suite passes (requires libclang for C++ tests); `paper` output shows Algorithm 1 == Algorithm 2;
`export-json` emits `schema_version: 1` per `docs/artifact-schema.md`.
`export-paper-json` uses the same schema after applying the Program 2 insertion.
