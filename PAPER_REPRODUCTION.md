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

Run the pseudo-code reproduction closer to Program 1:

```powershell
cargo run -- pseudo examples/program1.pseudo
```

Export the parsed reproduction as a JSON artifact after applying the paper's
Program 2 insertion:

```powershell
cargo run --quiet -- export-json examples/program1.pseudo > reproduction.json
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
- Pseudo parser output for `examples/program1.pseudo` against the hardcoded
  Program 1 graph.
- Program 2 insertion results.
- Algorithm 1 / Algorithm 2 equivalence.
- Deletion extension behavior.
- JSON export of nodes, `d_OPN_set`, and CCA sets after insertion.

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

- Parsing real programming languages into TT Graphs. The current pseudo parser
  is intentionally scoped to Program 1 style inputs.
- IDE / SDE integration.
- Persistence beyond plain JSON / DOT / CSV artifacts.
- A faithful reimplementation of the paper's exact formatting errors.
