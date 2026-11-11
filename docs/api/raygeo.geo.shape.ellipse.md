---
title: raygeo.geo.shape.ellipse
sidebar_label: raygeo.geo.shape.ellipse
---

Ellipse geometry queries.

Provides parametric point evaluation, uniform sampling over a full turn, circumference membership
and containment tests, and the enclosed area, for an ellipse given by center, semi-axis lengths and
rotation.

## Functions

### `get_ellipse_area()`

```python
get_ellipse_area(rx: float, ry: float) -> float
```

Area enclosed by the ellipse.

| Parameter    | Type    | Description                   |
| ------------ | ------- | ----------------------------- |
| `rx`         | `float` | Semi-axis length along x.     |
| `ry`         | `float` | Semi-axis length along y.     |
| _Returns_    | `float` | Enclosed area (pi * rx * ry). |
| _Complexity_ |         | O(1) time, O(1) space         |

### `get_ellipse_point_at()`

```python
get_ellipse_point_at(
    center: types.Point,
    rx: float,
    ry: float,
    rotation: float,
    t: float,
) -> types.Point
```

Point on the ellipse at parameter `t`.

Evaluates `center + R(rotation) * (rx * cos(t), ry * sin(t))`.

| Parameter    | Type          | Description                                     |
| ------------ | ------------- | ----------------------------------------------- |
| `center`     | `types.Point` | Ellipse center (x, y).                          |
| `rx`         | `float`       | Semi-axis length along the rotated x direction. |
| `ry`         | `float`       | Semi-axis length along the rotated y direction. |
| `rotation`   | `float`       | Rotation of the x axis in radians.              |
| `t`          | `float`       | Curve parameter in radians.                     |
| _Returns_    | `types.Point` | Point (x, y) on the ellipse.                    |
| _Complexity_ |               | O(1) time, O(1) space                           |

### `get_ellipse_samples()`

```python
get_ellipse_samples(
    center: types.Point,
    rx: float,
    ry: float,
    rotation: float,
    segments: int,
) -> list[types.Point]
```

Sample the ellipse uniformly over a full turn.

Returns `segments + 1` points from t = 0 through the closing point at t = 2\*pi.

| Parameter    | Type                | Description                                     |
| ------------ | ------------------- | ----------------------------------------------- |
| `center`     | `types.Point`       | Ellipse center (x, y).                          |
| `rx`         | `float`             | Semi-axis length along the rotated x direction. |
| `ry`         | `float`             | Semi-axis length along the rotated y direction. |
| `rotation`   | `float`             | Rotation of the x axis in radians.              |
| `segments`   | `int`               | Number of segments (sample count - 1).          |
| _Returns_    | `list[types.Point]` | List of points (x, y).                          |
| _Complexity_ |                     | O(n) time, O(n) space for n samples             |

### `is_point_inside_ellipse()`

```python
is_point_inside_ellipse(
    point: types.Point,
    center: types.Point,
    rx: float,
    ry: float,
    rotation: float,
) -> bool
```

Check whether a point lies inside or on the ellipse.

| Parameter    | Type          | Description                                     |
| ------------ | ------------- | ----------------------------------------------- |
| `point`      | `types.Point` | Query point (x, y).                             |
| `center`     | `types.Point` | Ellipse center (x, y).                          |
| `rx`         | `float`       | Semi-axis length along the rotated x direction. |
| `ry`         | `float`       | Semi-axis length along the rotated y direction. |
| `rotation`   | `float`       | Rotation of the x axis in radians.              |
| _Returns_    | `bool`        | True if the point is inside or on the ellipse.  |
| _Complexity_ |               | O(1) time, O(1) space                           |

### `is_point_on_ellipse()`

```python
is_point_on_ellipse(
    point: types.Point,
    center: types.Point,
    rx: float,
    ry: float,
    rotation: float,
    tolerance: float,
) -> bool
```

Check whether a point lies on the ellipse circumference.

The tolerance is interpreted in world units along the radial direction, scaled by the smaller
semi-axis.

| Parameter    | Type          | Description                                                   |
| ------------ | ------------- | ------------------------------------------------------------- |
| `point`      | `types.Point` | Query point (x, y).                                           |
| `center`     | `types.Point` | Ellipse center (x, y).                                        |
| `rx`         | `float`       | Semi-axis length along the rotated x direction.               |
| `ry`         | `float`       | Semi-axis length along the rotated y direction.               |
| `rotation`   | `float`       | Rotation of the x axis in radians.                            |
| `tolerance`  | `float`       | World-unit distance tolerance.                                |
| _Returns_    | `bool`        | True if the point lies on the circumference within tolerance. |
| _Complexity_ |               | O(1) time, O(1) space                                         |
