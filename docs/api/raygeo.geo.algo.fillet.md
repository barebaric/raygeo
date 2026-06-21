---
title: raygeo.geo.algo.fillet
sidebar_label: raygeo.geo.algo.fillet
sidebar_position: 9
---

Pure-geometry fillet operations.

Domain-neutral utilities for creating circular fillet arcs, appending them to polylines, and
trimming to safe spans.

- `create_fillet_polyline` — circular arc tangent to a direction.
- `append_end_fillets` — fillet both ends of an open polyline.
- `trim_to_safe_fillet_span` — longest sub-span whose end fillets avoid obstacles.

## Functions

### `append_end_fillets()`

```python
append_end_fillets(
    polyline: Sequence[tuple[float, float]],
    radius: float,
    sweep_angle: float,
    side: float,
) -> list[tuple[float, float]]
```

Append fillet arcs to both ends of an open polyline.

A reversed fillet is added at the start and a forward fillet at the end, producing a smooth rounded
path.

| Parameter     | Type                            | Description                      |
| ------------- | ------------------------------- | -------------------------------- |
| `polyline`    | `Sequence[tuple[float, float]]` | Input open polyline.             |
| `radius`      | `float`                         | Fillet radius.                   |
| `sweep_angle` | `float`                         | Arc sweep angle in radians.      |
| `side`        | `float`                         | Offset side (+1 left, -1 right). |
| _Returns_     | `list[tuple[float, float]]`     | Full polyline with fillets.      |

![``append_end_fillets`` rounds both ends of an open polyline with reversed-start / forward-end fillet arcs](images/geo-algo-fillet-append-end-fillets.png)

_`append_end_fillets` rounds both ends of an open polyline with reversed-start / forward-end fillet
arcs_

### `create_fillet_polyline()`

```python
create_fillet_polyline(
    p: tuple[float, float],
    dir: tuple[float, float],
    radius: float,
    sweep_angle: float,
    side: float,
    reverse: bool,
) -> tuple[tuple[float, float], list[tuple[float, float]]]
```

Create a circular fillet arc tangent to _dir_ at _p_.

`side` selects the offset side (+1 = left of _dir_, -1 = right). When `reverse` is `True` the arc
curls back opposite to _dir_.

| Parameter     | Type                                                    | Description                                            |
| ------------- | ------------------------------------------------------- | ------------------------------------------------------ |
| `p`           | `tuple[float, float]`                                   | Start point (x, y).                                    |
| `dir`         | `tuple[float, float]`                                   | Tangent direction vector (dx, dy).                     |
| `radius`      | `float`                                                 | Fillet radius.                                         |
| `sweep_angle` | `float`                                                 | Arc sweep angle in radians.                            |
| `side`        | `float`                                                 | Offset side (+1 left, -1 right).                       |
| `reverse`     | `bool`                                                  | Whether the arc is reversed.                           |
| _Returns_     | `tuple[tuple[float, float], list[tuple[float, float]]]` | `(center, polyline)` — arc centre and fillet vertices. |

![``create_fillet_polyline`` generates circular fillet arcs of arbitrary sweep angle, tangent to a direction at a point](images/geo-algo-fillet-create-fillet-polyline.png)

_`create_fillet_polyline` generates circular fillet arcs of arbitrary sweep angle, tangent to a
direction at a point_

![``create_fillet_polyline`` with ``side=+1`` (left) and ``side=-1`` (right) of the direction vector](images/geo-algo-fillet-create-fillet-polyline-side.png)

_`create_fillet_polyline` with `side=+1` (left) and `side=-1` (right) of the direction vector_

### `trim_to_safe_fillet_span()`

```python
trim_to_safe_fillet_span(
    polyline: Sequence[tuple[float, float]],
    outer_boundary: Sequence[tuple[float, float]],
    inner_obstacles: Sequence[Sequence[tuple[float, float]]] = [],
    radius: float = 3,
    margin: float = 0,
) -> tuple[tuple[float, float], tuple[float, float]] | None
```

Find the longest sub-span whose end fillets avoid obstacles.

Shortens from each end until the sweep is clear. Returns `(enter, exit)` or `None`.

| Parameter         | Type                                                          | Description                                  |
| ----------------- | ------------------------------------------------------------- | -------------------------------------------- |
| `polyline`        | `Sequence[tuple[float, float]]`                               | Open polyline to trim.                       |
| `outer_boundary`  | `Sequence[tuple[float, float]]`                               | Outer boundary polygon.                      |
| `inner_obstacles` | `Sequence[Sequence[tuple[float, float]]] = []`                | List of obstacle polygons (default []).      |
| `radius`          | `float = 3`                                                   | Fillet radius (default 3.0).                 |
| `margin`          | `float = 0`                                                   | Extra clearance past tangency (default 0.0). |
| _Returns_         | `tuple[tuple[float, float], tuple[float, float]] &#124; None` |                                              |

![``trim_to_safe_fillet_span`` finds the longest sub-span whose end fillets do not collide with obstacles (red)](images/geo-algo-fillet-trim-to-safe-fillet-span.png)

_`trim_to_safe_fillet_span` finds the longest sub-span whose end fillets do not collide with
obstacles (red)_
