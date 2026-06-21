---
title: raygeo.geo.algo.hsm
sidebar_label: raygeo.geo.algo.hsm
sidebar_position: 12
---

HSM cutting-arc geometry primitives.

Pure geometric helpers for adaptive clearing.

- `find_cutting_arc` — extract the outer (cutting) arc from a bite.
- `fillet_arc_ends` — round both ends of a cutting arc.
- `find_safe_sweep_end` — find the longest safe sub-arc.

For motion assembly (entry strategy, wavefront expansion, peeling, arc linking) see
`raygeo.ops.assembly.hsm`.

## Functions

### `fillet_arc_ends()`

```python
fillet_arc_ends(
    arc: Sequence[tuple[float, float]],
    pocket_boundary: Sequence[tuple[float, float]],
    islands: Sequence[Sequence[tuple[float, float]]] = [],
    tool_radius: float = 3,
    wall_margin: float = 0,
) -> list[tuple[float, float]]
```

Round both ends of a cutting arc with quarter-circle fillets.

The arc is trimmed to the longest sub-arc whose tool sweep (arc + end fillets of _tool_radius_) does
not collide with _pocket_boundary_ or _islands_. A 90° fillet of _tool_radius_ is then appended at
each end.

| Parameter         | Type                                           | Description                                  |
| ----------------- | ---------------------------------------------- | -------------------------------------------- |
| `arc`             | `Sequence[tuple[float, float]]`                | Cutting arc vertices (open polyline).        |
| `pocket_boundary` | `Sequence[tuple[float, float]]`                | Outer boundary of the pocket.                |
| `islands`         | `Sequence[Sequence[tuple[float, float]]] = []` | List of island (hole) polygons (default []). |
| `tool_radius`     | `float = 3`                                    | Tool / fillet radius in mm (default 3.0).    |
| `wall_margin`     | `float = 0`                                    | Extra clearance past tangency (default 0.0). |
| _Returns_         | `list[tuple[float, float]]`                    | Filleted arc as an open polyline.            |

### `find_cutting_arc()`

```python
find_cutting_arc(
    bite: Sequence[tuple[float, float]],
    cleared_fragments: Sequence[Sequence[tuple[float, float]]],
) -> list[tuple[float, float]] | None
```

Extract the cutting arc (outer) vertices from a bite polygon.

The cutting arc is the longest contiguous run of bite vertices that lie _outside_ all cleared
fragments.

| Parameter           | Type                                      | Description                                      |
| ------------------- | ----------------------------------------- | ------------------------------------------------ |
| `bite`              | `Sequence[tuple[float, float]]`           | Bite polygon vertices.                           |
| `cleared_fragments` | `Sequence[Sequence[tuple[float, float]]]` | List of cleared-area polygons.                   |
| _Returns_           | `list[tuple[float, float]] &#124; None`   | The cutting arc polyline, or None if degenerate. |

![Bite polygons from the first peeling iteration with the cutting arc (outer edge) highlighted in red — the cleared area is shown in blue](images/geo-algo-hsm-find-cutting-arc.png)

_Bite polygons from the first peeling iteration with the cutting arc (outer edge) highlighted in red
— the cleared area is shown in blue_

![Cutting arcs from passes without islands](images/geo-algo-hsm-find-cutting-arc-simple.png)

_Cutting arcs from passes without islands_

### `find_safe_sweep_end()`

```python
find_safe_sweep_end(
    arc: Sequence[tuple[float, float]],
    pocket_boundary: Sequence[tuple[float, float]],
    islands: Sequence[Sequence[tuple[float, float]]] = [],
    tool_radius: float = 3,
    wall_margin: float = 0,
) -> tuple[tuple[float, float], tuple[float, float]] | None
```

Find the longest safe sub-arc by iterative sweep shortening.

Returns the two points `(enter, exit)` delimiting the longest sub-arc of _arc_ whose tool sweep
(arc + end fillets of _tool_radius_) does not collide with _pocket_boundary_ or _islands_. Shortens
from each end until the sweep is clear. Returns `None` when no usable safe sub-arc remains.

| Parameter         | Type                                                          | Description                                  |
| ----------------- | ------------------------------------------------------------- | -------------------------------------------- |
| `arc`             | `Sequence[tuple[float, float]]`                               | Cutting arc vertices (open polyline).        |
| `pocket_boundary` | `Sequence[tuple[float, float]]`                               | Outer boundary of the pocket.                |
| `islands`         | `Sequence[Sequence[tuple[float, float]]] = []`                | List of island (hole) polygons (default []). |
| `tool_radius`     | `float = 3`                                                   | Tool radius in mm (default 3.0).             |
| `wall_margin`     | `float = 0`                                                   | Extra clearance past tangency (default 0.0). |
| _Returns_         | `tuple[tuple[float, float], tuple[float, float]] &#124; None` |                                              |
