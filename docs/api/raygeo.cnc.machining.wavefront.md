---
title: raygeo.cnc.machining.wavefront
sidebar_label: raygeo.cnc.machining.wavefront
---

## Functions

### `build_wavefront_workplan()`

```python
build_wavefront_workplan(
    pocket_boundary: Sequence[tuple[float, float]],
    islands: Sequence[Sequence[tuple[float, float]]] | None = None,
    tool_radius: float = 3,
    step_over: float = 2,
    target_z: float = -5,
    safe_margin: float = 1,
    angular_step: float = 0.1,
    area_tolerance: float = 1,
    precision: float = 0,
) -> list[dict]
```

Build a spiral-seed + wavefront-expansion workplan.

Finds the largest inscribed circle, emits a `FlatSpiral` step that seeds the pocket centre with a
cleared disk, then a `Wavefront` step that expands outward. No helical plunge is produced: the
spiral disk already covers the area the helix used to, so the wavefront seed is identical to the
legacy `adaptive_entry` path.

Combine with **raygeo.cnc.machining.plan.Workplan** to turn the steps into a toolpath.

| Parameter         | Type                                                         | Description                                                 |
| ----------------- | ------------------------------------------------------------ | ----------------------------------------------------------- |
| `pocket_boundary` | `Sequence[tuple[float, float]]`                              | Outer boundary as [(x, y), ...].                            |
| `islands`         | `Sequence[Sequence[tuple[float, float]]] &#124; None = None` | List of island polygons (default None).                     |
| `tool_radius`     | `float = 3`                                                  | Tool radius in mm (default 3.0).                            |
| `step_over`       | `float = 2`                                                  | Radial step-over (default 2.0).                             |
| `target_z`        | `float = -5`                                                 | Target cutting depth (default -5.0).                        |
| `safe_margin`     | `float = 1`                                                  | Safety margin from tool edge (default 1.0).                 |
| `angular_step`    | `float = 0.1`                                                | Angular step in radians (default 0.1).                      |
| `area_tolerance`  | `float = 1`                                                  | Convergence area tolerance (default 1.0).                   |
| `precision`       | `float = 0`                                                  | Vertex resampling spacing, 0 to disable (default 0.0).      |
| _Returns_         | `list[dict]`                                                 | List of WorkplanStep dicts (`FlatSpiral` then `Wavefront`). |
