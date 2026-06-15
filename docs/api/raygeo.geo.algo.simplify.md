---
title: raygeo.geo.algo.simplify
sidebar_label: raygeo.geo.algo.simplify
sidebar_position: 12
---

Polyline simplification using the Ramer-Douglas-Peucker algorithm.

Reduces the number of points in a polyline while preserving the overall shape within a given
tolerance.

## Functions

### `simplify_polyline()`

`simplify_polyline(points: collections.abc.Sequence[types.Point], tolerance: float) -> types.Polygon`

Simplify a polyline using the Ramer-Douglas-Peucker algorithm.

**Returns:** Simplified point sequence.

| Parameter    | Type                                    | Description                         |
| ------------ | --------------------------------------- | ----------------------------------- |
| `points`     | `collections.abc.Sequence[types.Point]` | Sequence of (x, y) points.          |
| `tolerance`  | `float`                                 | Simplification tolerance.           |
| _Returns_    | `types.Polygon`                         |                                     |
| _Complexity_ |                                         | O(n log n) average time, O(n) space |

![Simplify and linearize](images/simplify.png)

_Simplify and linearize_
