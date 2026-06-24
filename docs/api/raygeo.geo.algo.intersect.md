---
title: raygeo.geo.algo.intersect
sidebar_label: raygeo.geo.algo.intersect
---

Geometry intersection utilities.

Low-level intersection primitives for ray-segment and segment-segment tests, plus higher-level
self-intersection and cross-intersection checks on geometry command arrays.

## Functions

### `get_ray_line_intersection()`

```python
get_ray_line_intersection(
    origin: tuple[float, float],
    direction: tuple[float, float],
    a: tuple[float, float],
    b: tuple[float, float],
) -> tuple[float, float] | None
```

Intersect a ray with a line segment.

Given a ray starting at origin in the given direction, and a line segment from a to b, returns the
intersection point if the ray hits the segment (including endpoints) in the forward direction, or
None if there is no intersection.

| Parameter    | Type                              | Description                         |
| ------------ | --------------------------------- | ----------------------------------- |
| `origin`     | `tuple[float, float]`             | Ray start point (x, y).             |
| `direction`  | `tuple[float, float]`             | Ray direction vector (dx, dy).      |
| `a`          | `tuple[float, float]`             | Line segment start point (x, y).    |
| `b`          | `tuple[float, float]`             | Line segment end point (x, y).      |
| _Returns_    | `tuple[float, float] &#124; None` | Intersection point (x, y), or None. |
| _Complexity_ |                                   | O(1) time, O(1) space               |

![Ray–line segment intersection: the ray from origin O hits segments S₁ and S₂ (marked), misses S₃](images/geo-algo-intersect-ray-line.png)

*Ray–line segment intersection: the ray from origin O hits segments S₁ and S₂ (marked), misses S₃*

### `get_ray_polygon_intersection()`

```python
get_ray_polygon_intersection(
    origin: tuple[float, float],
    direction: tuple[float, float],
    polygon: list[tuple[float, float]],
) -> tuple[float, float] | None
```

Intersect a ray with a polygon boundary.

Given a ray starting at origin in the given direction, and a closed polygon defined by a list of
vertices, returns the closest intersection point with any edge of the polygon (including edge
endpoints), or None if the ray does not hit the polygon in the forward direction.

| Parameter    | Type                              | Description                                         |
| ------------ | --------------------------------- | --------------------------------------------------- |
| `origin`     | `tuple[float, float]`             | Ray start point (x, y).                             |
| `direction`  | `tuple[float, float]`             | Ray direction vector (dx, dy).                      |
| `polygon`    | `list[tuple[float, float]]`       | List of polygon vertices [(x1, y1), (x2, y2), ...]. |
| _Returns_    | `tuple[float, float] &#124; None` | Closest intersection point (x, y), or None.         |
| _Complexity_ |                                   | O(N) time, O(1) space                               |

![Ray–polygon intersection: the ray from origin O hits the polygon boundary at the closest intersection point along the ray direction.](images/geo-algo-intersect-ray-polygon.png)

*Ray–polygon intersection: the ray from origin O hits the polygon boundary at the closest
intersection point along the ray direction.*
