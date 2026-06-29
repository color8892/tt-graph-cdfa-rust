# Artifact Schema

`cargo run --quiet -- export-json examples/program1.pseudo` emits a stable JSON
artifact for inspecting a parsed TT Graph after the paper's Program 2 insertion.

The current root object has `schema_version: 1`.

```json
{
  "schema_version": 1,
  "nodes": [],
  "d_opn_set": [],
  "cca_sets": []
}
```

## Versioning

`schema_version` changes only when a consumer-visible JSON shape changes. Adding
new top-level sections, renaming fields, changing enum spelling, or changing CCA
tuple ordering requires a version bump.

## Nodes

Each `nodes` entry represents one TT Graph node.

```json
{
  "id": "Act1",
  "node_type": "Activity",
  "control_type": null,
  "operation_sequence": [{"variable": "x", "operation": "Read"}],
  "sequence_arc": null,
  "branch_arc": [],
  "scope_arc": "B1"
}
```

Fields:

- `id`: node identifier.
- `node_type`: `Activity`, `Control`, or `Block`.
- `control_type`: `And`, `Xor`, `Loop`, or `null`.
- `operation_sequence`: operations attached to the node.
- `sequence_arc`: next sequential node id, or `null`.
- `branch_arc`: child BLOCK node ids for control nodes.
- `scope_arc`: parent BLOCK/control id, or `null` for the root control.

## d_OPN_set

Each `d_opn_set` row is the maintained per-BLOCK summary:

```json
{
  "block": "B1",
  "variable": "x",
  "operation": "Write",
  "nodes": ["Act1", "Act2"]
}
```

`operation` is one of `Read`, `Write`, or `Kill`. `nodes` is sorted for stable
diffs.

## CCA Sets

Each `cca_sets` entry contains one non-empty CCA set on an AND node:

```json
{
  "and_node": "And1",
  "cca_type": "WriteRead",
  "entries": [
    {"variable": "x", "first_node": "Act1", "second_node": "Act2"}
  ]
}
```

`cca_type` is one of:

- `WriteWrite`
- `WriteRead`
- `WriteKill`
- `ReadKill`

Tuple ordering is normalized:

- `WriteRead`: `first_node` is the Write node, `second_node` is the Read node.
- `WriteKill`: `first_node` is the Write node, `second_node` is the Kill node.
- `ReadKill`: `first_node` is the Read node, `second_node` is the Kill node.
- `WriteWrite`: insertion/direct-scan normalization preserves the detected pair.
