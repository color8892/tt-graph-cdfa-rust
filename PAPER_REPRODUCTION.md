# Paper Reproduction Protocol

This crate is intended to reproduce the algorithmic content of
`COMPSAC_YAU_final.pdf`, not just demonstrate a similar idea.

Run the full paper reproduction harness:

```powershell
cargo run -- paper
```

Run the parser-based reproduction from the toy input file:

```powershell
cargo run -- parse examples/program1.tt
```

Run the C++ Clang reproduction of Program 1:

```powershell
cargo run -- cpp examples/program1.cpp
```

Run the implicit C++ reproduction (no OpenMP / `#pragma tt` / `tt_print`):

```powershell
cargo run -- cpp-implicit examples/program1_plain.cpp
```

Reproduce paper figures and extra tables:

```powershell
cargo run -- figure4
cargo run -- table5
cargo run --release -- bench-corpus
```

Run generic C++ analysis with an insertion:

```powershell
cargo run -- analyze-cpp examples/program1.cpp insert Act2 v Write
```

Run the C-like subset reproduction of Program 1:

```powershell
cargo run -- c examples/program1.c
```

Run generic C subset analysis with an insertion:

```powershell
cargo run -- analyze-c examples/program1.c insert Act2 v Write
```

Run the pseudo-code reproduction closer to Program 1:

```powershell
cargo run -- pseudo examples/program1.pseudo
```

Run a nested `split / branch / join` pseudo-code example:

```powershell
cargo run -- pseudo examples/nested_split.pseudo
```

Run generic pseudo-code analysis with an insertion:

```powershell
cargo run -- analyze-pseudo examples/nested_split.pseudo insert Act1 x Write
```

Export the parsed reproduction as a JSON artifact:

```powershell
cargo run --quiet -- export-json examples/program1.pseudo > reproduction.json
```

Export the parsed reproduction after applying the paper's Program 2 insertion:

```powershell
cargo run --quiet -- export-paper-json examples/program1.pseudo > reproduction-after-insertion.json
```

Toy syntax currently supports:

```text
and And1 {
  branch B1 {
    activity Act1 { read v; write v; }
    loop Loop1 ops { read i; } body B3 {
      activity Act2 { read v; }
    }
  }
}
```

`ops { ... }` on `loop` and `xor` models control-condition operations that the
paper includes in `d_OPN_set`, such as `Loop1` reading `i` and `Xor1` reading
`v`.

C++ syntax (`examples/program1.cpp`) is parsed by libclang. Parallel AND
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
(`examples/program1_pragma.cpp`) remains supported for older inputs.

C-like subset syntax (`examples/program1.c`) supports:

```c
parallel And1 {
  branch B1 {
    print(v);
    v = 10;
    while (i < 20) { ... }
  }
  branch B2 {
    if (v % 2 == 0) { ... } else { ... }
  }
}
```

`parallel` / `branch` are TT Graph extensions (not ISO C). Statements map to
operations the same way as the pseudo parser:

- `print(expr)` -> `Read` on identifiers in `expr`
- `kill(var)` -> `Kill(var)`
- `lhs = expr` -> `Read` on identifiers in `expr`, then `Write(lhs)`
- `while (cond)` -> LOOP control with `Read` on identifiers in `cond`
- `if (cond) { ... } else { ... }` -> XOR control

Pseudo syntax currently supports a line-oriented subset:

```text
split And1
branch B1
print(v)
v := 10
i := v
while i < 20 do
print(v)
end while
endbranch
join
```

`split / branch / join` blocks can be nested inside branches. Nested splits
produce nested AND control nodes and are included in ancestor `d_OPN_set`
summaries.

The pseudo parser infers operations from statements:

- `print v` -> `Read(v)`
- `v := 10` -> `Write(v)`
- `i := v` -> `Read(v), Write(i)`
- `while i < 20 do` -> `Loop1` stores `Read(i)` in the Program 1 reproduction
- `if v mod 2 == 0 then` -> `Xor1` stores `Read(v)` in the Program 1 reproduction

Consecutive statements are automatically grouped into `Act1`, `Act2`, etc.,
matching the paper's Figure 2 activity-node grouping for Program 1.

The `paper` command reproduces these parts of the paper:

- Program 1 / Figure 2: the hardcoded TT Graph for the running example.
- `examples/program1.tt`: a toy-language encoding of Program 1 that can be
  parsed into the same TT Graph records.
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
- Parser output for `examples/program1.tt` against the hardcoded Program 1
  graph.
- C++ Clang parser output for `examples/program1.cpp` against the hardcoded
  Program 1 graph.
- C subset parser output for `examples/program1.c` against the hardcoded Program 1
  graph.
- Pseudo parser output for `examples/program1.pseudo` against the hardcoded
  Program 1 graph.
- Program 2 insertion results.
- Algorithm 1 / Algorithm 2 equivalence.
- Nested AND/XOR/LOOP summary propagation against direct scan.
- Generic pseudo-code analysis with optional insertion and direct-scan
  equivalence reporting.
- Deletion extension behavior.
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
- Deletion is future work in the paper. This crate implements a conservative
  deletion extension by removing the operation from ancestor `d_OPN_set`
  records and recomputing affected AND-node CCA sets.

Out of scope for the current reproduction:

- Parsing arbitrary C++ without OpenMP parallel-section markers or
  `tt_print` / `tt_kill` hooks. The Clang frontend still needs explicit
  concurrency scaffolding (OpenMP sections or legacy `#pragma tt`) plus
  dataflow helper calls.
- Parsing full ISO C without extensions. The `c_frontend` remains a C-like
  subset with explicit `parallel` / `branch` regions.
- Parsing real programming languages without TT Graph annotations. The pseudo
  parser remains intentionally scoped to Program 1 style inputs.
- IDE / SDE integration.
- Persistence beyond plain JSON / DOT / CSV artifacts.
- A faithful reimplementation of the paper's exact formatting errors.

See `docs/paper-mapping.md` for the full artifact index and explicit gap list.
