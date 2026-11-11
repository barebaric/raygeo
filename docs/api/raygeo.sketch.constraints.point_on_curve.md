---
title: raygeo.sketch.constraints.point_on_curve
sidebar_label: raygeo.sketch.constraints.point_on_curve
---

## PointOnCurveConstraint

Enforces a point lies on a cubic Bezier curve. The control points are stored as offsets from their
owning endpoints; curves without control points use the straight-segment residual (`is_line`).

### `cp1_offset`

```python
cp1_offset: tuple[float, float]
```

First control point as offset from the start point.

### `cp2_offset`

```python
cp2_offset: tuple[float, float]
```

Second control point as offset from the end point.

### `end`

```python
end: int
```

Curve end point.

### `is_line`

```python
is_line: bool
```

Whether the curve degenerates to a straight segment.

### `point`

```python
point: int
```

The constrained point.

### `start`

```python
start: int
```

Curve start point.
