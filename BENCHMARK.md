# Benchmark Notes

This benchmark compares two insertion-time detection strategies:

- `d_OPN_set`: query maintained per-BLOCK operation summaries.
- direct scan: traverse reachable nodes in sibling BLOCK subgraphs.

The benchmark generates synthetic full binary AND graphs, inserts
`Write(target)` into the leftmost activity node, and reports whether both
strategies produce the same CCA set.

Command:

```powershell
cargo run --release -- bench
```

CSV artifact command:

```powershell
cargo run --quiet --release -- bench-csv > benchmark.csv
```

Latest local result:

```text
iterations=20, matching_stride=16, insertion=Write(target)

depth  nodes  leaves  target reads  CCA  d_OPN_set us  direct us  x faster  match
    4     61      16             1    1          11.5       30.0       2.6   true
    6    253      64             4    4          28.5      194.4       6.8   true
    8   1021     256            16   16          59.5     2907.4      48.9   true
```

Interpretation:

The benchmark supports the paper's main intuition: when the graph is large but
the edited variable is sparse, `d_OPN_set` avoids scanning large sibling
subtrees. The speedup grows with graph depth in this setup.

Complexity note:

The paper's simplified `O(log n)` claim is best read as a sparse-output binary
tree case. A more precise insertion cost is:

```text
O(h * b + k)
```

Where:

- `h` is the number of ancestor scopes visited through `scopeArc`
- `b` is the number of sibling BLOCK nodes checked at each AND node
- `k` is the number of matching nodes / generated CCA entries

The output term matters because any implementation must at least materialize
the anomalies it reports.
