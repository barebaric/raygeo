---
title: raygeo.geo.algo.engagement
sidebar_label: raygeo.geo.algo.engagement
sidebar_position: 9
---

![Engagement angle, area, and chord depth as a function of signed distance from the cleared boundary.](images/geo-algo-engagement-engagement-vs-distance.png)

_Engagement angle, area, and chord depth as a function of signed distance from the cleared
boundary._ Circle-boundary overlap (engagement) metrics.

## Functions

### `compute_engagement()`

```python
compute_engagement(
    d_to_boundary: float,
    radius: float,
) -> tuple[float, float, float]
```

Compute engagement angle, area, and chord depth.

| Parameter       | Type                         | Description                                                                                   |
| --------------- | ---------------------------- | --------------------------------------------------------------------------------------------- |
| `d_to_boundary` | `float`                      | Signed distance from the point to the nearest boundary (mm). Positive = outside the boundary. |
| `radius`        | `float`                      | Disk radius (mm).                                                                             |
| _Returns_       | `tuple[float, float, float]` | `(angle_rad, area, chord_depth)`.                                                             |

![Circle at three signed distances from the boundary. Shaded red arc is the contact arc (engagement).](images/geo-algo-engagement-circle-boundary.png)

_Circle at three signed distances from the boundary. Shaded red arc is the contact arc
(engagement)._

![Engagement heatmap around a circular cleared area. Green = low, red = high engagement.](images/geo-algo-engagement-engagement-heatmap.png)

_Engagement heatmap around a circular cleared area. Green = low, red = high engagement._
