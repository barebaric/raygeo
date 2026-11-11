---
title: raygeo.sketch
sidebar_label: raygeo.sketch
---

Sketch constraint solving.

Solves 2D geometric constraint systems for parametric sketching: a set of points is moved so that
geometric relationships (horizontal, distance, tangency, ...) hold, using the
trust-region-reflective least-squares algorithm. The caller describes the problem as a flat list of
PointDef entries and typed Constraint variants and receives solved positions, per-point constrained
(fully-determined) flags, and per-constraint conflict indices.

## Constraint

One constraint of a sketch problem. Point ids refer to \[`PointDef`\] ids; dimensional values must
already be resolved (parameter expressions evaluated) by the caller.

### Angle

Clockwise angle in degrees between two lines at their intersection; `e1_far`/`e2_far` select the ray
of each line.

### AspectRatio

`dist(p1, p2) == ratio * dist(p3, p4)`.

### Coincident

Two points are at the same location.

### Collinear

Three points on one line.

### Diameter

Diameter of a circle.

### Distance

Fixed distance between two points.

### Drag

Soft pull of a point towards a target, scaled by `weight`.

### EqualDistance

`dist(p1, p2) == dist(p3, p4)`.

### EqualLength

Equal characteristic lengths, one residual per `(base_a, base_b, other_a, other_b)` tuple.

### Horizontal

Two points share their Y coordinate.

### Parallelogram

`(p_width - p_origin) == (p4 - p_height)`.

### PerpCircles

Two circles intersecting at a right angle.

### PerpLineCenter

A line passing through a circle/arc center.

### PerpLineLine

Two lines perpendicular.

### PointOnBezier

A point on a cubic Bezier; control points as endpoint offsets, `is_line` selects the
straight-segment residual.

### PointOnLine

A point on the infinite line `(l1, l2)`.

### PointOnRadiusCurve

A point on a circle/arc circumference.

### Radius

Radius of a circle/arc.

### Rotational

`p2` is `p1` rotated by `angle_rad` about `center`.

### SymmetryLine

`p1` and `p2` are mirrored across the line `(l1, l2)`.

### SymmetryPoint

`center` is the midpoint of `p1` and `p2`.

### Tangent

A line tangent to a circle/arc.

### Vertical

Two points share their X coordinate.

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
