---
title: raygeo.geo.types
sidebar_label: raygeo.geo.types
sidebar_position: 44
---

Type aliases used throughout the raygeo API.

Point — a 2D coordinate as `(x, y)`. Point3D — a 3D coordinate as `(x, y, z)`. Point2DOr3D — union
of Point and Point3D for functions that accept both. Polygon — a list of points forming a closed
shape. Rect — a bounding rectangle as `(x_min, y_min, x_max, y_max)`. TransformMatrix — a 4x4 affine
transformation matrix for 2D/3D transforms. Edge — a pair of points `((x1, y1), (x2, y2))`.
CubicBezier — four control points `(p0, p1, p2, p3)`.

## Type Aliases

### CubicBezier

Type: `tuple[tuple[float, float], tuple[float, float], tuple[float, float], tuple[float, float]]`

### Edge

Type: `tuple[tuple[float, float], tuple[float, float]]`

### Point

Type: `tuple[float, float]`

### Point2DOr3D

Type: `Point | Point3D`

### Point3D

Type: `tuple[float, float, float]`

### Polygon

Type: `list[Point]`

### Polygon3D

Type: `list[tuple[float, float, float]]`

### Rect

Type: `tuple[float, float, float, float]`

### Rect3D

Type: `tuple[float, float, float, float, float, float]`

### TransformMatrix

Type: `list[list[float]] | numpy.ndarray[tuple[int, ...], Any]`
