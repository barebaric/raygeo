---
title: raygeo.ops.assembly.wavefront
sidebar_label: raygeo.ops.assembly.wavefront
---

## Functions

### `adaptive_wavefronts()`

```python
adaptive_wavefronts(
    part: ops.part.Part,
    step_over: float = 2,
    z: float = 0,
    area_tolerance: float = 1,
    precision: float = 0,
    cut_feed_rate: int = 1200,
    cut_power: float = 1,
) -> ops.assembly.AssemblyResult
```

Inside-out adaptive wavefronts.

Finds the largest inscribed circle inside *part*'s boundary, seeds the cleared area with concentric
rings spaced *step_over* apart, then iteratively expands the frontier outward by *step_over*,
clipping to the boundary. The loop terminates when the newly added area drops below
*area_tolerance*.

Each ring fragment is emitted as `MoveTo` + `LineTo` at height *z* with *cut_feed_rate* applied.

| Parameter        | Type                          | Description                                                                                                                                 |
| ---------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `part`           | `ops.part.Part`               | The part whose `stock_region` defines the pocket boundary and islands.                                                                      |
| `step_over`      | `float = 2`                   | Radial expansion per iteration (default 2.0).                                                                                               |
| `z`              | `float = 0`                   | Z height for generated commands (default 0.0).                                                                                              |
| `area_tolerance` | `float = 1`                   | Minimum area increase to continue (default 1.0).                                                                                            |
| `precision`      | `float = 0`                   | Edge tolerance for frontier simplification and vertex resampling; smaller values produce denser edges (default 0.0 = use internal default). |
| `cut_feed_rate`  | `int = 1200`                  | Feed rate for cutting moves (default 1200).                                                                                                 |
| `cut_power`      | `float = 1`                   | Laser power for cutting moves (0.0-1.0, default 1.0).                                                                                       |
| _Returns_        | `ops.assembly.AssemblyResult` | An **AssemblyResult** with wavefront cutting commands.                                                                                      |

![Adaptive wavefronts expand from the initial cleared disk (blue) to fill the pocket boundary (black)](images/ops-assembly-wavefront-wavefront-rect.png)

*Adaptive wavefronts expand from the initial cleared disk (blue) to fill the pocket boundary
(black)*

![Adaptive wavefronts in a pocket with three islands — contours wrap around each island as they expand](images/ops-assembly-wavefront-wavefront-multi.png)

*Adaptive wavefronts in a pocket with three islands — contours wrap around each island as they
expand*

![Adaptive wavefronts in a Y-shaped channel — contours split and propagate along each branch](images/ops-assembly-wavefront-wavefront-yshape.png)

*Adaptive wavefronts in a Y-shaped channel — contours split and propagate along each branch*

### `adaptive_wavefronts_multi_pocket()`

```python
adaptive_wavefronts_multi_pocket(
    part: ops.part.Part,
    step_over: float = 2,
    offset_mm: float = 0,
    area_tolerance: float = 0.01,
    precision: float = 0,
    cut_feed_rate: int = 1200,
    cut_power: float = 1,
    profile: bool = False,
) -> ops.assembly.AssemblyResult
```

Multi-pocket adaptive wavefronts.

Extracts all pockets from *part.geometry*, optionally offsets the boundary inward by *offset_mm*,
seeds each pocket with concentric rings spaced *step_over* apart, and runs wavefront expansion
inside each pocket. Returns the combined result.

**Raises:** `ValueError` — If the part has no geometry or no closed contours.

| Parameter        | Type                          | Description                                               |
| ---------------- | ----------------------------- | --------------------------------------------------------- |
| `part`           | `ops.part.Part`               | The part whose geometry defines the pockets.              |
| `step_over`      | `float = 2`                   | Radial expansion per iteration (default 2.0).             |
| `offset_mm`      | `float = 0`                   | Inward offset applied to all contours (default 0.0).      |
| `area_tolerance` | `float = 0.01`                | Minimum area increase to continue (default 0.01).         |
| `precision`      | `float = 0`                   | Edge tolerance for frontier simplification (default 0.0). |
| `cut_feed_rate`  | `int = 1200`                  | Feed rate for cutting moves (default 1200).               |
| `cut_power`      | `float = 1`                   | Laser power for cutting moves (0.0-1.0, default 1.0).     |
| `profile`        | `bool = False`                | Print a profiling report to stdout (default False).       |
| _Returns_        | `ops.assembly.AssemblyResult` | An **AssemblyResult** with combined wavefront paths.      |

![Adaptive wavefronts in a complex SVG shape — contours adapt to the boundary and wrap around islands](images/ops-assembly-wavefront-wavefront-svg.png)

*Adaptive wavefronts in a complex SVG shape — contours adapt to the boundary and wrap around
islands*
