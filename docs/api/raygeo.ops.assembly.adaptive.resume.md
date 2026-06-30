---
title: raygeo.ops.assembly.adaptive.resume
sidebar_label: raygeo.ops.assembly.adaptive.resume
---

## Functions

### `emit_resume_travel()`

```python
emit_resume_travel(
    ops: ops.Ops,
    cleared: ops.cut.cleared_area.ClearedArea,
    axis: geo.algo.medial_axis.MedialAxis | None,
    from_pt: tuple[float, float],
    to_pt: tuple[float, float],
    pocket_boundary: Sequence[tuple[float, float]],
    islands: Sequence[Sequence[tuple[float, float]]] = [],
    radius: float = 3,
    cut_z: float = -5,
) -> None
```

Emit a safe resume travel from *from_pt* to *to_pt* into *ops*.

| Parameter         | Type                                           | Description |
| ----------------- | ---------------------------------------------- | ----------- |
| `ops`             | `ops.Ops`                                      |             |
| `cleared`         | `ops.cut.cleared_area.ClearedArea`             |             |
| `axis`            | `geo.algo.medial_axis.MedialAxis &#124; None`  |             |
| `from_pt`         | `tuple[float, float]`                          |             |
| `to_pt`           | `tuple[float, float]`                          |             |
| `pocket_boundary` | `Sequence[tuple[float, float]]`                |             |
| `islands`         | `Sequence[Sequence[tuple[float, float]]] = []` |             |
| `radius`          | `float = 3`                                    |             |
| `cut_z`           | `float = -5`                                   |             |
| _Returns_         | `None`                                         |             |

### `mat_resume_target()`

```python
mat_resume_target(
    axis: geo.algo.medial_axis.MedialAxis,
    cleared: ops.cut.cleared_area.ClearedArea,
    tool: ops.assembly.adaptive.tool.Tool,
    cut_direction: str,
    step_length: float,
    pocket_boundary: Sequence[tuple[float, float]],
    islands: Sequence[Sequence[tuple[float, float]]],
    valid_tool_area: Sequence[Sequence[tuple[float, float]]],
) -> ops.cut.search.ToolPose | None
```

Pick a resume target via MAT-guided frontier walk.

| Parameter         | Type                                      | Description        |
| ----------------- | ----------------------------------------- | ------------------ |
| `axis`            | `geo.algo.medial_axis.MedialAxis`         |                    |
| `cleared`         | `ops.cut.cleared_area.ClearedArea`        |                    |
| `tool`            | `ops.assembly.adaptive.tool.Tool`         |                    |
| `cut_direction`   | `str`                                     | `"cw"` or `"ccw"`. |
| `step_length`     | `float`                                   |                    |
| `pocket_boundary` | `Sequence[tuple[float, float]]`           |                    |
| `islands`         | `Sequence[Sequence[tuple[float, float]]]` |                    |
| `valid_tool_area` | `Sequence[Sequence[tuple[float, float]]]` |                    |
| _Returns_         | `ops.cut.search.ToolPose &#124; None`     |                    |

### `search_reengagement()`

```python
search_reengagement(
    cleared: ops.cut.cleared_area.ClearedArea,
    segment_start: tuple[float, float],
    cut_direction: tuple[float, float],
    radius: float,
    step_length: float,
    advance: float,
    min_cut_area: float,
    valid_tool_area: Sequence[Sequence[tuple[float, float]]],
) -> ops.cut.search.ToolPose | None
```

SegmentResume: walk forward from segment_start along cut_direction.

| Parameter         | Type                                      | Description |
| ----------------- | ----------------------------------------- | ----------- |
| `cleared`         | `ops.cut.cleared_area.ClearedArea`        |             |
| `segment_start`   | `tuple[float, float]`                     |             |
| `cut_direction`   | `tuple[float, float]`                     |             |
| `radius`          | `float`                                   |             |
| `step_length`     | `float`                                   |             |
| `advance`         | `float`                                   |             |
| `min_cut_area`    | `float`                                   |             |
| `valid_tool_area` | `Sequence[Sequence[tuple[float, float]]]` |             |
| _Returns_         | `ops.cut.search.ToolPose &#124; None`     |             |

### `smooth_travel_path()`

```python
smooth_travel_path(
    from_pt: tuple[float, float],
    raw: Sequence[tuple[float, float]],
    obstacles: Sequence[Sequence[tuple[float, float]]] = [],
    clearance: float = 1,
) -> list[tuple[float, float]]
```

Smooth and shorten a cleared-territory travel path.

| Parameter   | Type                                           | Description |
| ----------- | ---------------------------------------------- | ----------- |
| `from_pt`   | `tuple[float, float]`                          |             |
| `raw`       | `Sequence[tuple[float, float]]`                |             |
| `obstacles` | `Sequence[Sequence[tuple[float, float]]] = []` |             |
| `clearance` | `float = 1`                                    |             |
| _Returns_   | `list[tuple[float, float]]`                    |             |

### `try_resume()`

```python
try_resume(
    cleared: ops.cut.cleared_area.ClearedArea,
    ops: ops.Ops,
    tool: ops.assembly.adaptive.tool.Tool,
    pocket_boundary: Sequence[tuple[float, float]],
    islands: Sequence[Sequence[tuple[float, float]]] = [],
    radius: float = 3,
    step_length: float = 0.6,
    advance: float = 1.5,
    cut_z: float = -5,
    max_deflection_deg: float = 30,
    valid_tool_area: Sequence[Sequence[tuple[float, float]]] = [],
    axis: geo.algo.medial_axis.MedialAxis | None = None,
    last_resume_area: float = -1,
    cut_direction: str = 'ccw',
    segment_start: tuple[float, float] = (0, 0),
    segment_heading: float = 0,
) -> bool
```

Try to recover after the tool stalls or is detected as stuck.

| Parameter            | Type                                                 | Description        |
| -------------------- | ---------------------------------------------------- | ------------------ |
| `cleared`            | `ops.cut.cleared_area.ClearedArea`                   |                    |
| `ops`                | `ops.Ops`                                            |                    |
| `tool`               | `ops.assembly.adaptive.tool.Tool`                    |                    |
| `pocket_boundary`    | `Sequence[tuple[float, float]]`                      |                    |
| `islands`            | `Sequence[Sequence[tuple[float, float]]] = []`       |                    |
| `radius`             | `float = 3`                                          |                    |
| `step_length`        | `float = 0.6`                                        |                    |
| `advance`            | `float = 1.5`                                        |                    |
| `cut_z`              | `float = -5`                                         |                    |
| `max_deflection_deg` | `float = 30`                                         |                    |
| `valid_tool_area`    | `Sequence[Sequence[tuple[float, float]]] = []`       |                    |
| `axis`               | `geo.algo.medial_axis.MedialAxis &#124; None = None` |                    |
| `last_resume_area`   | `float = -1`                                         |                    |
| `cut_direction`      | `str = 'ccw'`                                        | `"cw"` or `"ccw"`. |
| `segment_start`      | `tuple[float, float] = (0, 0)`                       |                    |
| `segment_heading`    | `float = 0`                                          |                    |
| _Returns_            | `bool`                                               |                    |
