---
title: raygeo.ops.assembly.adaptive
sidebar_label: raygeo.ops.assembly.adaptive
---

## Functions

### `adaptive_clearing()`

```python
adaptive_clearing(
    cleared: ops.cleared_area.ClearedArea,
    pocket_boundary: Sequence[tuple[float, float]],
    islands: Sequence[Sequence[tuple[float, float]]] = [],
    radius: float = 3,
    advance: float = 1.5,
    cut_z: float = -5,
    safe_z: float = 2,
    step_length: float = 0.6,
    max_deflection_deg: float = 30,
    travel_smoothing: int = 50,
    wall_margin: float = 0,
    area_tolerance: float = 1,
    cut_feed_rate: int = 1200,
    travel_rapid_rate: int = 8000,
    cut_power: float = 1,
    start_pos: tuple[float, float] | None = None,
    start_heading: float | None = None,
    expansion_batch_size: int = 1,
    profile: bool = False,
) -> ops.Ops
```

Run forward-stepping adaptive clearing.

Starting from the pre-populated *cleared* area, uses a constant-engagement stepping solver to
generate a continuous spiral toolpath from the seed clearing to the pocket wall.

The caller is responsible for populating *cleared* with entry polygons (e.g. from `adaptive_entry`)
and prepending the entry Ops to the result.

| Parameter              | Type                                           | Description                                                                                                                      |
| ---------------------- | ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `cleared`              | `ops.cleared_area.ClearedArea`                 | `ClearedArea` instance (mutated in place).                                                                                       |
| `pocket_boundary`      | `Sequence[tuple[float, float]]`                | Outer boundary of the pocket.                                                                                                    |
| `islands`              | `Sequence[Sequence[tuple[float, float]]] = []` | List of island (hole) polygons (default []).                                                                                     |
| `radius`               | `float = 3`                                    | Tool radius in mm (default 3.0).                                                                                                 |
| `advance`              | `float = 1.5`                                  | Forward advance per step (default 1.5).                                                                                          |
| `cut_z`                | `float = -5`                                   | Cutting Z height (default -5.0).                                                                                                 |
| `safe_z`               | `float = 2`                                    | Retract Z height for travel (default 2.0).                                                                                       |
| `step_length`          | `float = 0.6`                                  | Forward distance per solver step (default 0.6).                                                                                  |
| `max_deflection_deg`   | `float = 30`                                   | Maximum steering deflection per step in degrees (default 30).                                                                    |
| `travel_smoothing`     | `int = 50`                                     | Reserved (default 50).                                                                                                           |
| `wall_margin`          | `float = 0`                                    | Extra clearance between tool and boundary (default 0.0).                                                                         |
| `area_tolerance`       | `float = 1`                                    | Stop when remaining uncut area drops below this threshold (default 1.0).                                                         |
| `cut_feed_rate`        | `int = 1200`                                   | Feed rate for cutting moves (default 1200).                                                                                      |
| `travel_rapid_rate`    | `int = 8000`                                   | Rapid rate for travel moves (default 8000).                                                                                      |
| `cut_power`            | `float = 1`                                    | Laser power for cutting moves (0.0-1.0, default 1.0).                                                                            |
| `start_pos`            | `tuple[float, float] &#124; None = None`       | Initial tool position (x, y). When None, auto-detected from the cleared-area frontier.                                           |
| `start_heading`        | `float &#124; None = None`                     | Initial tool heading in radians. When None, auto-detected as the CCW tangent at start_pos.                                       |
| `expansion_batch_size` | `int = 1`                                      | Batch cleared-area expansions every N steps (default 1). Larger values improve performance but may slightly reduce path quality. |
| `profile`              | `bool = False`                                 | Print a profiling report to stdout (default False).                                                                              |
| _Returns_              | `ops.Ops`                                      | Ops with cutting commands (entry not included).                                                                                  |

![Forward-stepping constant-engagement clearing cuts (coloured by progress via the full-spectrum turbo gradient) from a central seed clearing (green), with MAT-routed travel links (red dashed) between segments.](images/ops-assembly-adaptive-adaptive-clearing-demo.png)

*Forward-stepping constant-engagement clearing cuts (coloured by progress via the full-spectrum
turbo gradient) from a central seed clearing (green), with MAT-routed travel links (red dashed)
between segments.*
