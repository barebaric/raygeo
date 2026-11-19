---
title: raygeo.sketch.components
sidebar_label: raygeo.sketch.components
---

## Functions

### `compute_constraint_components()`

```python
compute_constraint_components(
    registry: registry.EntityRegistry,
    constraints: Sequence[Any],
) -> list[set[int]]
```

Partitions all points referenced by constraints into connected components of the constraint graph.

| Parameter     | Type                      | Description |
| ------------- | ------------------------- | ----------- |
| `registry`    | `registry.EntityRegistry` |             |
| `constraints` | `Sequence[Any]`           |             |
| _Returns_     | `list[set[int]]`          |             |

### `get_referenced_points()`

```python
get_referenced_points(
    registry: registry.EntityRegistry,
    constraint: Any,
) -> list[int]
```

Returns all point ids a constraint couples: the points it references directly plus the points of the
entities it references.

| Parameter    | Type                      | Description |
| ------------ | ------------------------- | ----------- |
| `registry`   | `registry.EntityRegistry` |             |
| `constraint` | `Any`                     |             |
| _Returns_    | `list[int]`               |             |
