---
title: raygeo.ops.assembly.hsm
sidebar_label: raygeo.ops.assembly.hsm
sidebar_position: 53
---

## Functions

### `adaptive_entry()`

```python
adaptive_entry(
    pocket_boundary: Sequence[tuple[float, float]],
    islands: Sequence[Sequence[tuple[float, float]]] = [],
    tool_radius: float = 3,
    step_over: float = 2,
    safe_z: float = 2,
    target_z: float = -5,
    plunge_pitch: float = 1,
    safe_margin: float = 1,
    angular_step: float = 0.1,
    cut_feed_rate: int = 1200,
) -> tuple[ops.Ops, list[list[tuple[float, float]]]]
```

Fast central clearing entry.

Finds the optimal entry pole using `find_largest_circle`, then generates either a helix->spiral
(wide area) or zigzag ramp (tight slot).

The returned _cleared_polygons_ should be inserted into a `ClearedArea` via `add_cleared_polygons`.

| Parameter         | Type                                              | Description                                                                                                                                             |
| ----------------- | ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `pocket_boundary` | `Sequence[tuple[float, float]]`                   | Outer boundary of the pocket.                                                                                                                           |
| `islands`         | `Sequence[Sequence[tuple[float, float]]] = []`    | List of island (hole) polygons (default []).                                                                                                            |
| `tool_radius`     | `float = 3`                                       | Tool radius in mm (default 3.0).                                                                                                                        |
| `step_over`       | `float = 2`                                       | Radial step-over per spiral revolution (default 2.0).                                                                                                   |
| `safe_z`          | `float = 2`                                       | Safe (retract) Z height (default 2.0).                                                                                                                  |
| `target_z`        | `float = -5`                                      | Target cutting depth (default -5.0).                                                                                                                    |
| `plunge_pitch`    | `float = 1`                                       | Vertical descent per helix revolution (default 1.0).                                                                                                    |
| `safe_margin`     | `float = 1`                                       | Extra margin from tool edge to boundary (default 1.0).                                                                                                  |
| `angular_step`    | `float = 0.1`                                     | Angular step in radians for path vertices (default 0.1).                                                                                                |
| `cut_feed_rate`   | `int = 1200`                                      | Feed rate for the entry path (default 1200).                                                                                                            |
| _Returns_         | `tuple[ops.Ops, list[list[tuple[float, float]]]]` | `(ops, cleared_polygons)` where \*ops\* is an `Ops` with the entry toolpath and \*cleared_polygons\* is a list of polygons to add to the `ClearedArea`. |

![Adaptive clearing — Helix → Spiral in a pocket with three islands](images/ops-assembly-hsm-entry-multi.png)

_Adaptive clearing — Helix → Spiral in a pocket with three islands_

![Adaptive clearing — Helix → Spiral in an L-shaped pocket](images/ops-assembly-hsm-entry-lshape.png)

_Adaptive clearing — Helix → Spiral in an L-shaped pocket_

![Adaptive clearing — ZigZag Ramp in a tight slot](images/ops-assembly-hsm-entry-tight.png)

_Adaptive clearing — ZigZag Ramp in a tight slot_

### `adaptive_peeling()`

```python
adaptive_peeling(
    cleared: geo.algo.cleared_area.ClearedArea,
    pocket_boundary: Sequence[tuple[float, float]],
    islands: Sequence[Sequence[tuple[float, float]]] = [],
    tool_radius: float = 3,
    step_over: float = 2,
    cut_z: float = -5,
    safe_z: float = 2,
    wall_margin: float = 0,
    travel_smoothing: int = 50,
    area_tolerance: float = 1,
    cut_feed_rate: int = 1200,
    travel_rapid_rate: int = 8000,
) -> ops.Ops
```

Run the peeling (D-cut) clearing strategy and return an Ops.

Starting from the _cleared_ state (mutated in place), each iteration expands the cleared boundary
outward by _step_over_, clips to the valid tool area, computes crescent-shaped "bites", and
generates a D-cut for each bite. The individual passes are linked into a single Ops: each cutting
arc at _cut_z_ (LineTo with _cut_feed_rate_) followed by a travel segment at _safe_z_ (MoveTo with
_travel_rapid_rate_). The Medial Axis of the pocket is used to route travel around obstacles.

| Parameter           | Type                                           | Description                                                 |
| ------------------- | ---------------------------------------------- | ----------------------------------------------------------- |
| `cleared`           | `geo.algo.cleared_area.ClearedArea`            | `ClearedArea` instance (mutated in place).                  |
| `pocket_boundary`   | `Sequence[tuple[float, float]]`                | Outer boundary of the pocket.                               |
| `islands`           | `Sequence[Sequence[tuple[float, float]]] = []` | List of island (hole) polygons (default []).                |
| `tool_radius`       | `float = 3`                                    | Tool radius in mm (default 3.0).                            |
| `step_over`         | `float = 2`                                    | Radial expansion per iteration (default 2.0).               |
| `cut_z`             | `float = -5`                                   | Cutting Z height (default -5.0).                            |
| `safe_z`            | `float = 2`                                    | Retract Z height for travel segments (default 2.0).         |
| `wall_margin`       | `float = 0`                                    | Extra clearance between tool sweep and walls (default 0.0). |
| `travel_smoothing`  | `int = 50`                                     | Gaussian smoothing for MAT-routed travel (default 50).      |
| `area_tolerance`    | `float = 1`                                    | Convergence tolerance in square mm (default 1.0).           |
| `cut_feed_rate`     | `int = 1200`                                   | Feed rate for cutting moves (default 1200).                 |
| `travel_rapid_rate` | `int = 8000`                                   | Rapid rate for travel moves (default 8000).                 |
| _Returns_           | `ops.Ops`                                      | Ops with cutting and travel commands.                       |

![adaptive_peeling on a rectangular pocket — cutting arcs (blue, solid) at cut depth and travel links (orange, dashed) at safe Z](images/ops-assembly-hsm-adaptive-peeling-2d.png)

_adaptive_peeling on a rectangular pocket — cutting arcs (blue, solid) at cut depth and travel links
(orange, dashed) at safe Z_

![adaptive_peeling (3-D) — Z colouring shows cutting depth (blue) vs travel height (red)](images/ops-assembly-hsm-adaptive-peeling-3d.png)

_adaptive_peeling (3-D) — Z colouring shows cutting depth (blue) vs travel height (red)_

![adaptive_peeling on a pocket with three islands — travel links route around obstacles](images/ops-assembly-hsm-adaptive-peeling-multi.png)

_adaptive_peeling on a pocket with three islands — travel links route around obstacles_

### `adaptive_wavefronts()`

```python
adaptive_wavefronts(
    cleared: geo.algo.cleared_area.ClearedArea,
    pocket_boundary: Sequence[tuple[float, float]],
    islands: Sequence[Sequence[tuple[float, float]]] = [],
    tool_radius: float = 3,
    step_over: float = 2,
    z: float = 0,
    area_tolerance: float = 1,
    cut_feed_rate: int = 1200,
) -> ops.Ops
```

Inside-out adaptive wavefronts.

Starting from the _cleared_ state, each iteration expands the cleared boundary outward by
_step_over_, clips to the valid tool area (pocket boundary offset inward by _tool_radius_, with
islands excluded), and adds the result back to _cleared_. The loop terminates when the newly added
area drops below _area_tolerance_.

Each ring fragment is emitted as `MoveTo` + `LineTo` at height _z_ with _cut_feed_rate_ applied.

| Parameter         | Type                                           | Description                                      |
| ----------------- | ---------------------------------------------- | ------------------------------------------------ |
| `cleared`         | `geo.algo.cleared_area.ClearedArea`            | `ClearedArea` instance (mutated in place).       |
| `pocket_boundary` | `Sequence[tuple[float, float]]`                | Outer boundary of the pocket.                    |
| `islands`         | `Sequence[Sequence[tuple[float, float]]] = []` | List of island (hole) polygons (default []).     |
| `tool_radius`     | `float = 3`                                    | Tool radius in mm (default 3.0).                 |
| `step_over`       | `float = 2`                                    | Radial expansion per iteration (default 2.0).    |
| `z`               | `float = 0`                                    | Z height for generated commands (default 0.0).   |
| `area_tolerance`  | `float = 1`                                    | Minimum area increase to continue (default 1.0). |
| `cut_feed_rate`   | `int = 1200`                                   | Feed rate for cutting moves (default 1200).      |
| _Returns_         | `ops.Ops`                                      | Ops with wavefront cutting commands.             |

![Adaptive wavefronts expanding outward from the initial cleared disk (blue) to fill the pocket boundary (black)](images/ops-assembly-hsm-wavefront-rect.png)

_Adaptive wavefronts expanding outward from the initial cleared disk (blue) to fill the pocket
boundary (black)_

![Adaptive wavefronts in a pocket with three islands — contours wrap around each island as they expand](images/ops-assembly-hsm-wavefront-multi.png)

_Adaptive wavefronts in a pocket with three islands — contours wrap around each island as they
expand_

![Adaptive wavefronts in a Y-shaped channel — contours split and propagate along each branch](images/ops-assembly-hsm-wavefront-yshape.png)

_Adaptive wavefronts in a Y-shaped channel — contours split and propagate along each branch_

### `link_arcs_to_ops()`

```python
link_arcs_to_ops(
    arcs: Sequence[Sequence[tuple[float, float]]],
    uncleared: Sequence[Sequence[tuple[float, float]]] = [],
    cut_z: float = -1,
    safe_z: float = 5,
    mat: tuple[list[tuple[float, float]], list[tuple[int, int]]] | None = None,
    safe_margin: float = 0,
    smoothing_amount: int = 50,
    preserve_order: bool = False,
    cut_feed_rate: int = 1200,
    travel_rapid_rate: int = 8000,
) -> ops.Ops
```

Link filleted arcs into an Ops with MAT-routed travel.

Consecutive arcs are joined by travel segments (MoveTo) at _safe_z_. When the direct line would
cross (or pass within _safe_margin_ of) any polygon in _uncleared_, the connection uses the Medial
Axis to route around obstacles, then smoothed.

Cutting arcs are emitted as LineTo at _cut_z_ with _cut_feed_rate_; travel links as MoveTo at
_safe_z_ with _travel_rapid_rate_.

| Parameter           | Type                                                                         | Description                                                                                                            |
| ------------------- | ---------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `arcs`              | `Sequence[Sequence[tuple[float, float]]]`                                    | Sequence of arcs (each a list of (x, y) points).                                                                       |
| `uncleared`         | `Sequence[Sequence[tuple[float, float]]] = []`                               | Areas to avoid during travel (default []).                                                                             |
| `cut_z`             | `float = -1`                                                                 | Cutting Z height (default -1.0).                                                                                       |
| `safe_z`            | `float = 5`                                                                  | Safe (rapid) Z height (default 5.0).                                                                                   |
| `mat`               | `tuple[list[tuple[float, float]], list[tuple[int, int]]] &#124; None = None` | Optional `(nodes, edges)` tuple from `compute_medial_axis` for obstacle-aware routing.                                 |
| `safe_margin`       | `float = 0`                                                                  | Minimum distance from uncleared polygons for a direct travel line to be considered safe (default 0 = no margin check). |
| `smoothing_amount`  | `int = 50`                                                                   | Gaussian smoothing amount (0-200) applied to MAT-routed travel (default 50).                                           |
| `preserve_order`    | `bool = False`                                                               | Keep arc order as given instead of nearest-neighbour reordering (default False).                                       |
| `cut_feed_rate`     | `int = 1200`                                                                 | Feed rate for cutting moves (default 1200).                                                                            |
| `travel_rapid_rate` | `int = 8000`                                                                 | Rapid rate for travel moves (default 8000).                                                                            |
| _Returns_           | `ops.Ops`                                                                    | Ops with cutting LineTo and travel MoveTo commands.                                                                    |

![Pre-computed filleted arcs linked into an Ops with MAT-routed travel segments](images/ops-assembly-hsm-link-arcs.png)

_Pre-computed filleted arcs linked into an Ops with MAT-routed travel segments_
