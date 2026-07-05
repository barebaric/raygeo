---
title: raygeo.geo.shape.point
sidebar_label: raygeo.geo.shape.point
---

Individual point operations.

Provides equality testing within a configurable tolerance, midpoint computation between two points,
2D/3D get_circumcenter of three points, and applying a 4x4 affine transformation matrix to a single
point.

## Functions

### `are_points_equal_3d()`

```python
are_points_equal_3d(
    p1: types.Point3D,
    p2: types.Point3D,
    tolerance: float,
) -> bool
```

Check if two 3D points are equal within tolerance.

| Parameter    | Type            | Description                                |
| ------------ | --------------- | ------------------------------------------ |
| `p1`         | `types.Point3D` | First point (x, y, z).                     |
| `p2`         | `types.Point3D` | Second point (x, y, z).                    |
| `tolerance`  | `float`         | Maximum allowed difference.                |
| _Returns_    | `bool`          | True if points are equal within tolerance. |
| _Complexity_ |                 | O(1) time, O(1) space                      |

### `get_circumcenter()`

```python
get_circumcenter(
    a: types.Point,
    b: types.Point,
    c: types.Point,
) -> tuple[types.Point, float]
```

Compute the get_circumcenter and radius of three 2D points.

Returns the center of the unique circle passing through all three points along with its radius.
Returns `((0.0, 0.0), -1.0)` when the points are collinear.

| Parameter    | Type                        | Description                                  |
| ------------ | --------------------------- | -------------------------------------------- |
| `a`          | `types.Point`               | First point (x, y).                          |
| `b`          | `types.Point`               | Second point (x, y).                         |
| `c`          | `types.Point`               | Third point (x, y).                          |
| _Returns_    | `tuple[types.Point, float]` | `(center, radius)` where center is `(x, y)`. |
| _Complexity_ |                             | O(1) time, O(1) space                        |

### `get_circumcenter_3d()`

```python
get_circumcenter_3d(
    a: types.Point3D,
    b: types.Point3D,
    c: types.Point3D,
) -> Optional[types.Point3D]
```

Compute the get_circumcenter of three 3D points.

Returns the center of the unique circle passing through all three points. Returns `None` when the
points are collinear.

| Parameter    | Type                      | Description                                    |
| ------------ | ------------------------- | ---------------------------------------------- |
| `a`          | `types.Point3D`           | First point (x, y, z).                         |
| `b`          | `types.Point3D`           | Second point (x, y, z).                        |
| `c`          | `types.Point3D`           | Third point (x, y, z).                         |
| _Returns_    | `Optional[types.Point3D]` | Circumcenter (x, y, z) or `None` if collinear. |
| _Complexity_ |                           | O(1) time, O(1) space                          |

![Circumcenter of three 3D points with circumcircle](images/geo-shape-point-circumcenter-3d.png)

*Circumcenter of three 3D points with circumcircle*

### `get_midpoint_3d()`

```python
get_midpoint_3d(p1: types.Point3D, p2: types.Point3D) -> types.Point3D
```

Get the midpoint between two 3D points.

| Parameter    | Type            | Description             |
| ------------ | --------------- | ----------------------- |
| `p1`         | `types.Point3D` | First point (x, y, z).  |
| `p2`         | `types.Point3D` | Second point (x, y, z). |
| _Returns_    | `types.Point3D` | Midpoint (x, y, z).     |
| _Complexity_ |                 | O(1) time, O(1) space   |

![Midpoint of a 3D segment](images/geo-shape-point-midpoint-3d.png)

*Midpoint of a 3D segment*

### `rotate_point()`

```python
rotate_point(point: types.Point, angle: float) -> types.Point
```

Rotate a 2D point around the origin.

| Parameter    | Type          | Description                                    |
| ------------ | ------------- | ---------------------------------------------- |
| `point`      | `types.Point` | Point (x, y) to rotate.                        |
| `angle`      | `float`       | Rotation angle in radians (counter-clockwise). |
| _Returns_    | `types.Point` | Rotated point (x, y).                          |
| _Complexity_ |               | O(1) time, O(1) space                          |

### `transform_point_3d()`

```python
transform_point_3d(
    matrix: Sequence[Sequence[float]],
    x: float,
    y: float,
    z: float,
) -> types.Point3D
```

Apply an affine transformation matrix to a 3D point.

| Parameter    | Type                        | Description                       |
| ------------ | --------------------------- | --------------------------------- |
| `matrix`     | `Sequence[Sequence[float]]` | 4x4 affine transformation matrix. |
| `x`          | `float`                     | X coordinate.                     |
| `y`          | `float`                     | Y coordinate.                     |
| `z`          | `float`                     | Z coordinate.                     |
| _Returns_    | `types.Point3D`             | Transformed point (x, y, z).      |
| _Complexity_ |                             | O(1) time, O(1) space             |
