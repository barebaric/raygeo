---
title: raygeo.ops.material.fold
sidebar_label: raygeo.ops.material.fold
---

The fold kernel: aggregate material effects against one stock.

## Functions

### `fold_effects()`

```python
fold_effects(spec: spec.MaterialFoldSpec) -> state.MaterialState
```

Fold the spec's entries against the stock into a snapshot.

Runs the prismatic fold only: through-cut classification, void union clipped to the stock, the burn
surface map, provenance, and escalation signals. The GIL is released while folding.

| Parameter | Type                    | Description |
| --------- | ----------------------- | ----------- |
| `spec`    | `spec.MaterialFoldSpec` |             |
| _Returns_ | `state.MaterialState`   |             |
