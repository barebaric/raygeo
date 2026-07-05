---
title: raygeo.ops.cut.search
sidebar_label: raygeo.ops.cut.search
---

## ToolPose

### `heading`

```python
heading: float
```

### `pos`

```python
pos: tuple[float, float, float]
```

## Functions

### `search_frontier_engagement()`

```python
search_frontier_engagement(
    cleared: cleared_area.ClearedArea,
    start: ToolPose,
    radius: float,
    step_length: float,
    advance: float,
    min_cut_area: float,
    max_cut_area: float,
) -> Optional[ToolPose]
```

Walk the frontier forward from `start_pos`, skipping the closest vertex. Returns the first vertex
whose outward cut-area probe falls in `[min, max]`.

The returned position is offset inward (into the cleared area) by `radius - advance` so the tool
starts at the correct engagement depth.

| Parameter      | Type                       | Description                                  |
| -------------- | -------------------------- | -------------------------------------------- |
| `cleared`      | `cleared_area.ClearedArea` | `ClearedArea` instance.                      |
| `start`        | `ToolPose`                 | `ToolPose` seed.                             |
| `radius`       | `float`                    | Disk radius (mm).                            |
| `step_length`  | `float`                    | Forward step distance (mm) for the probe.    |
| `advance`      | `float`                    | Stepover distance (mm) for inward offset.    |
| `min_cut_area` | `float`                    | Minimum cut area (mm²).                      |
| `max_cut_area` | `float`                    | Maximum cut area (mm²), e.g. `float("inf")`. |
| _Returns_      | `Optional[ToolPose]`       | `ToolPose` or `None`.                        |

![Walk forward from the engagement point to find the next frontier match.](images/ops-cut-search-search-frontier-engagement.png)

*Walk forward from the engagement point to find the next frontier match.*

![Multi-island pocket — end positions (triangles) yield resume positions (stars) with outward headings.](images/ops-cut-search-search-frontier-engagement-multi.png)

*Multi-island pocket — end positions (triangles) yield resume positions (stars) with outward
headings.*
