---
title: raygeo.cnc.machining.entry
sidebar_label: raygeo.cnc.machining.entry
---

## Functions

### `build_entry_workplan()`

```python
build_entry_workplan(
    region_polygon: Sequence[tuple[float, float]],
    entry_point: tuple[float, float],
    r_max: float,
    islands: Sequence[Sequence[tuple[float, float]]] | None = None,
    tool_radius: float = 3,
    step_over: float = 2,
    safe_z: float = 2,
    target_z: float = -5,
    plunge_pitch: float = 1,
    safe_margin: float = 1,
    angular_step: float = 0.1,
) -> list[dict]
```

Build an entry workplan for a single wide region.

Strategy is chosen based on `r_max`: helix+spiral when `r_max >= 2 × tool_diameter`, toroidal ramp
if a carrier is found, or zigzag ramp as fallback.

Use **raygeo.ops.feature.region.find_regions** to obtain region data first.

| Parameter        | Type                                                         | Description                                   |
| ---------------- | ------------------------------------------------------------ | --------------------------------------------- |
| `region_polygon` | `Sequence[tuple[float, float]]`                              | Region boundary as [(x, y), ...].             |
| `entry_point`    | `tuple[float, float]`                                        | Inscribed-circle centre (x, y).               |
| `r_max`          | `float`                                                      | Largest inscribed circle radius in mm.        |
| `islands`        | `Sequence[Sequence[tuple[float, float]]] &#124; None = None` | List of island polygons (default None).       |
| `tool_radius`    | `float = 3`                                                  | Tool radius in mm (default 3.0).              |
| `step_over`      | `float = 2`                                                  | Radial step-over (default 2.0).               |
| `safe_z`         | `float = 2`                                                  | Safe Z height (default 2.0).                  |
| `target_z`       | `float = -5`                                                 | Target cutting depth (default -5.0).          |
| `plunge_pitch`   | `float = 1`                                                  | Helix pitch per revolution (default 1.0).     |
| `safe_margin`    | `float = 1`                                                  | Safety margin from tool edge (default 1.0).   |
| `angular_step`   | `float = 0.1`                                                | Angular step in radians (default 0.1).        |
| _Returns_        | `list[dict]`                                                 | List of WorkplanStep dicts with a "kind" key. |

![Entry workplan: rectangle (Helix+FlatSpiral), H-shape (ToroidalClear), cup (RampEntry).](images/cnc-machining-entry-entry-workplan.png)

*Entry workplan: rectangle (Helix+FlatSpiral), H-shape (ToroidalClear), cup (RampEntry).*
