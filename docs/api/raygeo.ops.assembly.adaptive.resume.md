---
title: raygeo.ops.assembly.adaptive.resume
sidebar_label: raygeo.ops.assembly.adaptive.resume
---

## Functions

### `emit_resume_travel()`

```python
emit_resume_travel(
    part: ops.cut.Part,
    ops: ops.Ops,
    to_pt: tuple[float, float, float],
    radius: float = 3,
    cut_z: float = -5,
    cleared: ops.cut.cleared_area.ClearedArea | None = None,
    from_pt: tuple[float, float, float] = (0, 0, 0),
    axis: geo.algo.medial_axis.MedialAxis | None = None,
) -> None
```

Emit a resume travel to *to_pt* using the routing strategies.

| Parameter | Type                                                  | Description |
| --------- | ----------------------------------------------------- | ----------- |
| `part`    | `ops.cut.Part`                                        |             |
| `ops`     | `ops.Ops`                                             |             |
| `to_pt`   | `tuple[float, float, float]`                          |             |
| `radius`  | `float = 3`                                           |             |
| `cut_z`   | `float = -5`                                          |             |
| `cleared` | `ops.cut.cleared_area.ClearedArea &#124; None = None` |             |
| `from_pt` | `tuple[float, float, float] = (0, 0, 0)`              |             |
| `axis`    | `geo.algo.medial_axis.MedialAxis &#124; None = None`  |             |
| _Returns_ | `None`                                                |             |

### `try_resume()`

```python
try_resume(
    part: ops.cut.Part,
    cleared: ops.cut.cleared_area.ClearedArea,
    ops: ops.Ops,
    tool: ops.assembly.adaptive.tool.Tool,
    radius: float = 3,
    step_over: float = 1.5,
    cut_z: float = -5,
    max_deflection_deg: float = 30,
    valid_tool_area: Sequence[Sequence[tuple[float, float]]] = [],
    axis: geo.algo.medial_axis.MedialAxis | None = None,
    last_resume_area: float = -1,
    cut_direction: str = 'ccw',
    segment_start: tuple[float, float, float] = (0, 0, 0),
    segment_heading: float = 0,
) -> bool
```

Try to recover after the tool stalls or is detected as stuck.

| Parameter            | Type                                                 | Description        |
| -------------------- | ---------------------------------------------------- | ------------------ |
| `part`               | `ops.cut.Part`                                       |                    |
| `cleared`            | `ops.cut.cleared_area.ClearedArea`                   |                    |
| `ops`                | `ops.Ops`                                            |                    |
| `tool`               | `ops.assembly.adaptive.tool.Tool`                    |                    |
| `radius`             | `float = 3`                                          |                    |
| `step_over`          | `float = 1.5`                                        |                    |
| `cut_z`              | `float = -5`                                         |                    |
| `max_deflection_deg` | `float = 30`                                         |                    |
| `valid_tool_area`    | `Sequence[Sequence[tuple[float, float]]] = []`       |                    |
| `axis`               | `geo.algo.medial_axis.MedialAxis &#124; None = None` |                    |
| `last_resume_area`   | `float = -1`                                         |                    |
| `cut_direction`      | `str = 'ccw'`                                        | `"cw"` or `"ccw"`. |
| `segment_start`      | `tuple[float, float, float] = (0, 0, 0)`             |                    |
| `segment_heading`    | `float = 0`                                          |                    |
| _Returns_            | `bool`                                               |                    |
