---
title: raygeo.ops.assembly.wavefront
sidebar_label: raygeo.ops.assembly.wavefront
---

## Functions

### `adaptive_wavefronts()`

```python
adaptive_wavefronts(
    part: ops.part.Part,
    tool_radius: float = 3,
    step_over: float = 2,
    z: float = 0,
    area_tolerance: float = 1,
    precision: float = 0,
    cut_feed_rate: int = 1200,
    cut_power: float = 1,
) -> ops.assembly.result.AssemblyResult
```

Inside-out adaptive wavefronts.

Starting from the cleared state inside *part*, each iteration expands the cleared boundary outward
by *step_over*, clips to the valid tool area (pocket boundary offset inward by *tool_radius*, with
islands excluded), and adds the result back to the part's cleared state. The loop terminates when
the newly added area drops below *area_tolerance*.

Each ring fragment is emitted as `MoveTo` + `LineTo` at height *z* with *cut_feed_rate* applied.

| Parameter        | Type                                 | Description                                                                                                                                 |
| ---------------- | ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `part`           | `ops.part.Part`                      | The part whose `cleared` field tracks accumulated workpiece state and whose geometry defines the pocket boundary and islands.               |
| `tool_radius`    | `float = 3`                          | Tool radius in mm (default 3.0).                                                                                                            |
| `step_over`      | `float = 2`                          | Radial expansion per iteration (default 2.0).                                                                                               |
| `z`              | `float = 0`                          | Z height for generated commands (default 0.0).                                                                                              |
| `area_tolerance` | `float = 1`                          | Minimum area increase to continue (default 1.0).                                                                                            |
| `precision`      | `float = 0`                          | Edge tolerance for frontier simplification and vertex resampling; smaller values produce denser edges (default 0.0 = use internal default). |
| `cut_feed_rate`  | `int = 1200`                         | Feed rate for cutting moves (default 1200).                                                                                                 |
| `cut_power`      | `float = 1`                          | Laser power for cutting moves (0.0-1.0, default 1.0).                                                                                       |
| _Returns_        | `ops.assembly.result.AssemblyResult` | An **AssemblyResult** with wavefront cutting commands.                                                                                      |

![Adaptive wavefronts expand from the initial cleared disk (blue) to fill the pocket boundary (black)](images/ops-assembly-wavefront-wavefront-rect.png)

*Adaptive wavefronts expand from the initial cleared disk (blue) to fill the pocket boundary
(black)*

![Adaptive wavefronts in a pocket with three islands — contours wrap around each island as they expand](images/ops-assembly-wavefront-wavefront-multi.png)

*Adaptive wavefronts in a pocket with three islands — contours wrap around each island as they
expand*

![Adaptive wavefronts in a Y-shaped channel — contours split and propagate along each branch](images/ops-assembly-wavefront-wavefront-yshape.png)

*Adaptive wavefronts in a Y-shaped channel — contours split and propagate along each branch*

![Adaptive wavefronts in a complex SVG shape — contours adapt to the boundary and wrap around islands](images/ops-assembly-wavefront-wavefront-svg.png)

*Adaptive wavefronts in a complex SVG shape — contours adapt to the boundary and wrap around
islands*
