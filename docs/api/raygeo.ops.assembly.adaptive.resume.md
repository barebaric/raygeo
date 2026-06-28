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
    tool_pos: tuple[float, float],
    valid_tool_area: Sequence[Sequence[tuple[float, float]]],
) -> tuple[list[tuple[float, float]], float] | None
```

Pick a resume target by walking the MAT to the nearest uncleared node.

| Parameter         | Type                                                  | Description |
| ----------------- | ----------------------------------------------------- | ----------- |
| `axis`            | `geo.algo.medial_axis.MedialAxis`                     |             |
| `cleared`         | `ops.cut.cleared_area.ClearedArea`                    |             |
| `tool_pos`        | `tuple[float, float]`                                 |             |
| `valid_tool_area` | `Sequence[Sequence[tuple[float, float]]]`             |             |
| _Returns_         | `tuple[list[tuple[float, float]], float] &#124; None` |             |

### `nearest_uncleared_node()`

```python
nearest_uncleared_node(
    axis: medial_axis.MedialAxis,
    start: int,
    is_cleared: Sequence[bool],
) -> Optional[int]
```

BFS over the Medial Axis tree from a start node, returning the index of the nearest (fewest hops)
node that is **not** cleared.

| Parameter    | Type                     | Description                                     |
| ------------ | ------------------------ | ----------------------------------------------- |
| `axis`       | `medial_axis.MedialAxis` | `MedialAxis` instance.                          |
| `start`      | `int`                    | Starting node index.                            |
| `is_cleared` | `Sequence[bool]`         | Cleared/uncleared mask (one bool per node).     |
| _Returns_    | `Optional[int]`          | Index of the nearest uncleared node, or `None`. |

### `smooth_travel_path()`

```python
smooth_travel_path(
    from_pt: tuple[float, float],
    raw: Sequence[tuple[float, float]],
    islands: Sequence[Sequence[tuple[float, float]]] = [],
    remaining: Sequence[Sequence[tuple[float, float]]] = [],
    clearance: float = 1,
) -> list[tuple[float, float]]
```

Smooth and shorten a cleared-territory travel path.

| Parameter   | Type                                           | Description |
| ----------- | ---------------------------------------------- | ----------- |
| `from_pt`   | `tuple[float, float]`                          |             |
| `raw`       | `Sequence[tuple[float, float]]`                |             |
| `islands`   | `Sequence[Sequence[tuple[float, float]]] = []` |             |
| `remaining` | `Sequence[Sequence[tuple[float, float]]] = []` |             |
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
    valid_tool_area: Sequence[Sequence[tuple[float, float]]] = [],
    axis: geo.algo.medial_axis.MedialAxis | None = None,
    last_resume_area: float = -1,
) -> bool
```

Try to recover after the tool stalls or is detected as stuck.

| Parameter          | Type                                                 | Description |
| ------------------ | ---------------------------------------------------- | ----------- |
| `cleared`          | `ops.cut.cleared_area.ClearedArea`                   |             |
| `ops`              | `ops.Ops`                                            |             |
| `tool`             | `ops.assembly.adaptive.tool.Tool`                    |             |
| `pocket_boundary`  | `Sequence[tuple[float, float]]`                      |             |
| `islands`          | `Sequence[Sequence[tuple[float, float]]] = []`       |             |
| `radius`           | `float = 3`                                          |             |
| `step_length`      | `float = 0.6`                                        |             |
| `advance`          | `float = 1.5`                                        |             |
| `cut_z`            | `float = -5`                                         |             |
| `valid_tool_area`  | `Sequence[Sequence[tuple[float, float]]] = []`       |             |
| `axis`             | `geo.algo.medial_axis.MedialAxis &#124; None = None` |             |
| `last_resume_area` | `float = -1`                                         |             |
| _Returns_          | `bool`                                               |             |
