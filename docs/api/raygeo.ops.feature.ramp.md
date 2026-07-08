---
title: raygeo.ops.feature.ramp
sidebar_label: raygeo.ops.feature.ramp
---

## Functions

### `find_ramp_carrier()`

```python
find_ramp_carrier(
    boundary: Sequence[tuple[float, float]],
    islands: Sequence[Sequence[tuple[float, float]]] | None = None,
    tool_radius: float = 3,
    max_ramp_angle_deg: float = 45,
) -> tuple[tuple[float, float], tuple[float, float]] | None
```

Find the longest straight carrier segment suitable for ramp entry.

Returns a `((x1, y1), (x2, y2))` tuple representing the longest straight-line segment within the
valid tool-centre region (boundary eroded by `tool_radius` minus dilated islands) that is long
enough for a ramp descent of one pass at the given maximum ramp angle.

The returned segment is oriented so the start point has the smaller coordinate on the dominant axis.

| Parameter            | Type                                                          | Description                                           |
| -------------------- | ------------------------------------------------------------- | ----------------------------------------------------- |
| `boundary`           | `Sequence[tuple[float, float]]`                               | Outer boundary polygon as `[(x, y), ...]`.            |
| `islands`            | `Sequence[Sequence[tuple[float, float]]] &#124; None = None`  | List of island polygons (optional).                   |
| `tool_radius`        | `float = 3`                                                   | Tool radius in mm.                                    |
| `max_ramp_angle_deg` | `float = 45`                                                  | Maximum ramp angle in degrees.                        |
| _Returns_            | `tuple[tuple[float, float], tuple[float, float]] &#124; None` | `((x1, y1), (x2, y2))` or `None` if no carrier found. |

![Three scenarios showing the ramp carrier returned by . Left: an L-shaped pocket with no island — the longest inside-segment on the dominant axis. Centre: a wide rectangle with a central blocking island; the dashed gray rectangle shows the dilated no-go band, and the navy carrier avoids it entirely. Right: a T-shaped pocket with one island in the vertical stem — the carrier occupies the wide horizontal arm while steering clear of the island.](images/ops-feature-ramp-ramp-carrier-slots.png)

*Three scenarios showing the ramp carrier returned by `find_ramp_carrier`. Left: an L-shaped pocket
with no island — the longest inside-segment on the dominant axis. Centre: a wide rectangle with a
central blocking island; the dashed gray rectangle shows the dilated no-go band, and the navy
carrier avoids it entirely. Right: a T-shaped pocket with one island in the vertical stem — the
carrier occupies the wide horizontal arm while steering clear of the island.*
