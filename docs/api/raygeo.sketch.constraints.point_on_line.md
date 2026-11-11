---
title: raygeo.sketch.constraints.point_on_line
sidebar_label: raygeo.sketch.constraints.point_on_line
---

## PointOnLineConstraint

Enforces a point lies on a line or circle/arc circumference.

### `point`

```python
point: int
```

The constrained point.

### `target`

```python
target: PointOnTarget
```

Resolved target geometry.

## PointOnTarget

The resolved target geometry: an infinite line, or a circle/arc circumference.

### Line

The infinite line through two points.

### RadiusCurve

The circle/arc through center and radius-bearing point.
