---
title: raygeo.geo.shape.rect
sidebar_label: raygeo.geo.shape.rect
sidebar_position: 30
---

Rectangle intersection and containment tests.

Provides functions to test whether two axis-aligned rectangles intersect, whether one rectangle
fully contains another, and utilities for computing the union bounding rectangle of multiple
geometries.

## Functions

### `do_rects_intersect()`

```python
do_rects_intersect(r1: types.Rect, r2: types.Rect) -> bool
```

Check if two rectangles intersect.

| Parameter    | Type         | Description                                    |
| ------------ | ------------ | ---------------------------------------------- |
| `r1`         | `types.Rect` | First rectangle (x_min, y_min, x_max, y_max).  |
| `r2`         | `types.Rect` | Second rectangle (x_min, y_min, x_max, y_max). |
| _Returns_    | `bool`       | True if the rectangles intersect.              |
| _Complexity_ |              | O(1) time, O(1) space                          |

### `does_rect_contain_rect()`

```python
does_rect_contain_rect(outer: types.Rect, inner: types.Rect) -> bool
```

Check if one rectangle contains another.

| Parameter    | Type         | Description                                   |
| ------------ | ------------ | --------------------------------------------- |
| `outer`      | `types.Rect` | Outer rectangle (x_min, y_min, x_max, y_max). |
| `inner`      | `types.Rect` | Inner rectangle (x_min, y_min, x_max, y_max). |
| _Returns_    | `bool`       | True if outer fully contains inner.           |
| _Complexity_ |              | O(1) time, O(1) space                         |

### `does_rect_intersect_rect()`

```python
does_rect_intersect_rect(r1: types.Rect, r2: types.Rect) -> bool
```

Check if two rectangles intersect.

| Parameter    | Type         | Description                                    |
| ------------ | ------------ | ---------------------------------------------- |
| `r1`         | `types.Rect` | First rectangle (x_min, y_min, x_max, y_max).  |
| `r2`         | `types.Rect` | Second rectangle (x_min, y_min, x_max, y_max). |
| _Returns_    | `bool`       | True if the rectangles intersect.              |
| _Complexity_ |              | O(1) time, O(1) space                          |

### `get_combined_rect()`

```python
get_combined_rect(geometries: list[Geometry]) -> types.Rect
```

Compute the union bounding box of multiple geometries.

| Parameter    | Type             | Description                                               |
| ------------ | ---------------- | --------------------------------------------------------- |
| `geometries` | `list[Geometry]` | List of Geometry objects.                                 |
| _Returns_    | `types.Rect`     | Union bounding rectangle (x_min, y_min, x_max, y_max).    |
| _Complexity_ |                  | O(n) time, O(1) space where n is the number of geometries |

### `is_point_inside_rect()`

```python
is_point_inside_rect(point: types.Point, rect: types.Rect) -> bool
```

Check if a point is inside a rectangle.

| Parameter    | Type          | Description                                |
| ------------ | ------------- | ------------------------------------------ |
| `point`      | `types.Point` | Point (x, y) to test.                      |
| `rect`       | `types.Rect`  | Rectangle (x_min, y_min, x_max, y_max).    |
| _Returns_    | `bool`        | True if the point is inside the rectangle. |
| _Complexity_ |               | O(1) time, O(1) space                      |
