---
title: raygeo.geo.algo.clipping
sidebar_label: raygeo.geo.algo.clipping
sidebar_position: 5
---

Line and polygon clipping operations.

Provides functions for clipping line segments against rectangles and polygon regions, as well as
converting between float and Clipper integer coordinate systems.

## Functions

### `clip_line_segment_with_polygons()`

```python
clip_line_segment_with_polygons(
    p1: types.Point3D,
    p2: types.Point3D,
    regions: Sequence[Sequence[types.Point]],
) -> list[tuple[types.Point3D, types.Point3D]]
```

Clip line segments that fall within polygon regions.

| Parameter    | Type                                        | Description                                                                                  |
| ------------ | ------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `p1`         | `types.Point3D`                             | Start point of the line segment.                                                             |
| `p2`         | `types.Point3D`                             | End point of the line segment.                                                               |
| `regions`    | `Sequence[Sequence[types.Point]]`           | Polygon regions to clip against.                                                             |
| _Returns_    | `list[tuple[types.Point3D, types.Point3D]]` | List of clipped segments.                                                                    |
| _Complexity_ |                                             | O(n \* m) time, O(n) space where n is the number of regions and m their average vertex count |

![Line clipped to polygon](images/clipping-polygon.png)

_Line clipped to polygon_

### `clip_line_segment_with_rect()`

```python
clip_line_segment_with_rect(
    p1: types.Point3D,
    p2: types.Point3D,
    rect: types.Rect,
) -> Optional[tuple[types.Point3D, types.Point3D]]
```

Clip a line segment with a rectangle.

| Parameter    | Type                                            | Description                                      |
| ------------ | ----------------------------------------------- | ------------------------------------------------ |
| `p1`         | `types.Point3D`                                 | Start point of the line segment.                 |
| `p2`         | `types.Point3D`                                 | End point of the line segment.                   |
| `rect`       | `types.Rect`                                    | Clipping rectangle (x_min, y_min, x_max, y_max). |
| _Returns_    | `Optional[tuple[types.Point3D, types.Point3D]]` | Clipped segment or None if fully outside.        |
| _Complexity_ |                                                 | O(1) time, O(1) space                            |

![Line clipped to rectangle](images/clipping-rect.png)

_Line clipped to rectangle_

### `from_clipper()`

```python
from_clipper(polygon: list[tuple[int, int]]) -> list[tuple[float, float]]
```

Convert a polygon from Clipper coordinates.

| Parameter    | Type                        | Description                     |
| ------------ | --------------------------- | ------------------------------- |
| `polygon`    | `list[tuple[int, int]]`     | Integer polygon from Clipper.   |
| _Returns_    | `list[tuple[float, float]]` | Polygon with float coordinates. |
| _Complexity_ |                             | O(n) time, O(n) space           |

### `subtract_polygons_from_line_segment()`

```python
subtract_polygons_from_line_segment(
    p1: types.Point3D,
    p2: types.Point3D,
    regions: Sequence[Sequence[types.Point]],
) -> list[tuple[types.Point3D, types.Point3D]]
```

Subtract polygon regions from a line segment.

| Parameter    | Type                                        | Description                                                                                  |
| ------------ | ------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `p1`         | `types.Point3D`                             | Start point of the line segment.                                                             |
| `p2`         | `types.Point3D`                             | End point of the line segment.                                                               |
| `regions`    | `Sequence[Sequence[types.Point]]`           | List of polygon regions to subtract.                                                         |
| _Returns_    | `list[tuple[types.Point3D, types.Point3D]]` | List of remaining segments after subtraction.                                                |
| _Complexity_ |                                             | O(n \* m) time, O(n) space where n is the number of regions and m their average vertex count |

![Subtract polygon from line](images/clipping-subtract.png)

_Subtract polygon from line_

### `to_clipper()`

```python
to_clipper(polygon: types.Polygon) -> list[tuple[int, int]]
```

Convert a polygon to Clipper coordinates.

| Parameter    | Type                    | Description                                   |
| ------------ | ----------------------- | --------------------------------------------- |
| `polygon`    | `types.Polygon`         | Input polygon as a list of (x, y) points.     |
| _Returns_    | `list[tuple[int, int]]` | Polygon with integer coordinates for Clipper. |
| _Complexity_ |                         | O(n) time, O(n) space                         |
