# Benchmark Notes

This crate compares two insertion-time detection strategies:

- `d_OPN_set`: query maintained per-BLOCK operation summaries (Algorithm 1).
- direct scan: traverse reachable nodes in sibling BLOCK subgraphs (Algorithm 2).

The paper gives **complexity** (Table 5) but **no empirical timing table**. These
benchmarks supplement the paper with reproducible measurements in this repo.

## Quick commands

```powershell
# Original synthetic full-binary AND benchmark
cargo run --release -- bench
cargo run --quiet --release -- bench-csv > benchmark.csv

# Expanded corpus (paper graph, figure6 depths, chain AND, plain C++)
cargo run --release -- bench-corpus
cargo run --quiet --release -- bench-corpus --csv > corpus.csv

# Table 5 theory + empirical figure6 trend
cargo run --release -- bench-paper-table5
```

## Corpus cases (`bench-corpus`)

| case_id | Source | Insertion |
|---------|--------|-----------|
| `paper_program1` | `build_paper_example_graph()` | `Write(v)` into `Act2` |
| `figure6_depth_{4,6,8,10,12}` | Full binary AND tree | `Write(target)` into leftmost leaf |
| `chain_and_depth_{4,6,8,10,12}` | Left-spine AND chain | `Write(target)` into deepest-left leaf |
| `plain_cpp` | `examples/program1_plain.cpp` (implicit parser) | `Write(v)` into `Act2` |

CSV columns:

`case_id,node_count,leaf_count,matching_leaf_count,summary_us,direct_us,speedup,match`

## Original synthetic benchmark

Parameters:

- Depths: 4, 6, 8
- Iterations: 20
- Matching stride: 16
- Insertion: `Write(target)` into leftmost leaf activity

Sample local results:

```text
depth  nodes  leaves  summary_us  direct_us  speedup  match
    4     61      16        11.5       30.0      2.6   true
    6    253      64        28.5      194.4      6.8   true
    8   1021     256        59.5     2907.4     48.9   true
```

## Interpretation

When the graph is large but the edited variable is sparse, `d_OPN_set` avoids
scanning large sibling subtrees. Speedup grows with depth in this setup.

Precise cost (repo form):

```text
O(h * b + k)
```

- `h` — ancestor scopes via `scopeArc`
- `b` — sibling BLOCK branches at each AND
- `k` — emitted CCA entries

Timed sections measure **insertion detection only**; graph construction and
cloning are excluded.