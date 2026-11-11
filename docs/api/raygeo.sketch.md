---
title: raygeo.sketch
sidebar_label: raygeo.sketch
---

Sketch constraint solving.

Solves 2D geometric constraint systems for parametric sketching: a set of points is moved so that
geometric relationships (horizontal, distance, tangency, ...) hold, using the
trust-region-reflective least-squares algorithm. The caller describes the problem as a flat list of
PointDef entries and constraint objects (see the raygeo.sketch.constraints submodules) and receives
solved positions, per-point constrained (fully-determined) flags, and per-constraint conflict
indices.

## PointDef

Definition of one sketch point.

### `fixed`

```python
fixed: bool
```

Whether the point is pinned and never moves.

### `id`

```python
id: int
```

Unique point id, stable across solves.

### `x`

```python
x: float
```

Current X coordinate.

### `y`

```python
y: float
```

Current Y coordinate.

## Solver

Trust-region least-squares solver for sketch constraint systems.

### `conflicting()`

```python
conflicting(threshold: float) -> list[int]
```

Indices of hard constraints whose residual exceeds `threshold` at the final positions.

| Parameter   | Type        | Description |
| ----------- | ----------- | ----------- |
| `threshold` | `float`     |             |
| _Returns_   | `list[int]` |             |

### `constrained()`

```python
constrained() -> list[bool]
```

Per-point fully-constrained flags, aligned with the input point order.

| Parameter | Type         | Description |
| --------- | ------------ | ----------- |
| _Returns_ | `list[bool]` |             |

### `max_residuals()`

```python
max_residuals() -> list[float]
```

Maximum absolute residual per hard constraint.

| Parameter | Type          | Description |
| --------- | ------------- | ----------- |
| _Returns_ | `list[float]` |             |

### `positions()`

```python
positions() -> list[tuple[float, float]]
```

Final `(x, y)` positions, aligned with the input point order.

| Parameter | Type                        | Description |
| --------- | --------------------------- | ----------- |
| _Returns_ | `list[tuple[float, float]]` |             |

### `solve()`

```python
solve(tolerance: float, update_dof: bool) -> bool
```

Runs the solve. Returns True when all components converged.

| Parameter    | Type    | Description                                                                 |
| ------------ | ------- | --------------------------------------------------------------------------- |
| `tolerance`  | `float` | Cost threshold and ftol of the optimizer.                                   |
| `update_dof` | `bool`  | Whether to re-analyze per-point constrained flags after a successful solve. |
| _Returns_    | `bool`  |                                                                             |
