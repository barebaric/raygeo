---
title: raygeo.sketch
sidebar_label: raygeo.sketch
---

Sketch constraint solving.

Solves 2D geometric constraint systems for parametric sketching: a set of points is moved so that
geometric relationships (horizontal, distance, tangency, ...) hold, using the
trust-region-reflective least-squares algorithm. The caller describes the problem as a flat list of
PointDef entries and constraint objects (see the raygeo.sketch.constraints submodules) and receives
solved positions, per-point constrained (fully-determined) flags, and per-constraint conflict
indices.

## PointDef

Definition of one sketch point.

### `fixed`

```python
fixed: bool
```

Whether the point is pinned and never moves.

### `id`

```python
id: int
```

Unique point id, stable across solves.

### `x`

```python
x: float
```

Current X coordinate.

### `y`

```python
y: float
```

Current Y coordinate.

## Sketch

A complete host-agnostic sketch document.

### `arrays`

```python
arrays: list
```

### `constraints`

```python
constraints: list
```

### `fills`

```python
fills: list
```

### `hidden`

```python
hidden: bool
```

### `last_solve_values`

```python
last_solve_values: dict
```

### `meta`

```python
meta: Any
```

### `name`

```python
name: str
```

### `origin_id`

```python
origin_id: int
```

### `params`

```python
params: Any
```

### `registry`

```python
registry: registry.EntityRegistry
```

### `solved_ctx`

```python
solved_ctx: Optional[dict]
```

### `uid`

```python
uid: str
```

### `add_arc()`

```python
add_arc(
    start: int,
    end: int,
    center: int,
    clockwise: bool = False,
    construction: bool = False,
) -> int
```

| Parameter      | Type           | Description |
| -------------- | -------------- | ----------- |
| `start`        | `int`          |             |
| `end`          | `int`          |             |
| `center`       | `int`          |             |
| `clockwise`    | `bool = False` |             |
| `construction` | `bool = False` |             |
| _Returns_      | `int`          |             |

### `add_bezier()`

```python
add_bezier(
    start: int,
    end: int,
    construction: bool = False,
    cp1: Optional[tuple[float, float]] = None,
    cp2: Optional[tuple[float, float]] = None,
) -> int
```

| Parameter      | Type                                   | Description |
| -------------- | -------------------------------------- | ----------- |
| `start`        | `int`                                  |             |
| `end`          | `int`                                  |             |
| `construction` | `bool = False`                         |             |
| `cp1`          | `Optional[tuple[float, float]] = None` |             |
| `cp2`          | `Optional[tuple[float, float]] = None` |             |
| _Returns_      | `int`                                  |             |

### `add_circle()`

```python
add_circle(center: int, radius: int, construction: bool = False) -> int
```

| Parameter      | Type           | Description |
| -------------- | -------------- | ----------- |
| `center`       | `int`          |             |
| `radius`       | `int`          |             |
| `construction` | `bool = False` |             |
| _Returns_      | `int`          |             |

### `add_line()`

```python
add_line(p1: int, p2: int, construction: bool = False) -> int
```

| Parameter      | Type           | Description |
| -------------- | -------------- | ----------- |
| `p1`           | `int`          |             |
| `p2`           | `int`          |             |
| `construction` | `bool = False` |             |
| _Returns_      | `int`          |             |

### `add_point()`

```python
add_point(x: float, y: float, fixed: bool = False) -> int
```

| Parameter | Type           | Description |
| --------- | -------------- | ----------- |
| `x`       | `float`        |             |
| `y`       | `float`        |             |
| `fixed`   | `bool = False` |             |
| _Returns_ | `int`          |             |

### `apply_undo_state()`

```python
apply_undo_state(state: dict) -> None
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `state`   | `dict` |             |
| _Returns_ | `None` |             |

### `capture_undo_state()`

```python
capture_undo_state() -> dict
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

### `clone_for_geometry()`

```python
clone_for_geometry() -> Sketch
```

| Parameter | Type     | Description |
| --------- | -------- | ----------- |
| _Returns_ | `Sketch` |             |

### `conflicting_constraints()`

```python
conflicting_constraints() -> list
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `list` |             |

### `constrain_coincident()`

```python
constrain_coincident(p1: int, p2: int) -> None
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `p1`      | `int`  |             |
| `p2`      | `int`  |             |
| _Returns_ | `None` |             |

### `constrain_diameter()`

```python
constrain_diameter(circle_id: int, diameter: Any) -> Any
```

| Parameter   | Type  | Description |
| ----------- | ----- | ----------- |
| `circle_id` | `int` |             |
| `diameter`  | `Any` |             |
| _Returns_   | `Any` |             |

### `constrain_distance()`

```python
constrain_distance(p1: int, p2: int, dist: Any) -> Any
```

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| `p1`      | `int` |             |
| `p2`      | `int` |             |
| `dist`    | `Any` |             |
| _Returns_ | `Any` |             |

### `constrain_equal_distance()`

```python
constrain_equal_distance(p1: int, p2: int, p3: int, p4: int) -> None
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `p1`      | `int`  |             |
| `p2`      | `int`  |             |
| `p3`      | `int`  |             |
| `p4`      | `int`  |             |
| _Returns_ | `None` |             |

### `constrain_equal_length()`

```python
constrain_equal_length(entity_ids: Sequence[int]) -> None
```

| Parameter    | Type            | Description |
| ------------ | --------------- | ----------- |
| `entity_ids` | `Sequence[int]` |             |
| _Returns_    | `None`          |             |

### `constrain_horizontal()`

```python
constrain_horizontal(p1: int, p2: int) -> None
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `p1`      | `int`  |             |
| `p2`      | `int`  |             |
| _Returns_ | `None` |             |

### `constrain_perpendicular()`

```python
constrain_perpendicular(l1: int, l2: int) -> None
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `l1`      | `int`  |             |
| `l2`      | `int`  |             |
| _Returns_ | `None` |             |

### `constrain_point_on_line()`

```python
constrain_point_on_line(point_id: int, shape_id: int) -> None
```

| Parameter  | Type   | Description |
| ---------- | ------ | ----------- |
| `point_id` | `int`  |             |
| `shape_id` | `int`  |             |
| _Returns_  | `None` |             |

### `constrain_radius()`

```python
constrain_radius(entity_id: int, radius: Any) -> Any
```

| Parameter   | Type  | Description |
| ----------- | ----- | ----------- |
| `entity_id` | `int` |             |
| `radius`    | `Any` |             |
| _Returns_   | `Any` |             |

### `constrain_symmetry()`

```python
constrain_symmetry(point_ids: Sequence[int], entity_ids: Sequence[int]) -> None
```

| Parameter    | Type            | Description |
| ------------ | --------------- | ----------- |
| `point_ids`  | `Sequence[int]` |             |
| `entity_ids` | `Sequence[int]` |             |
| _Returns_    | `None`          |             |

### `constrain_tangent()`

```python
constrain_tangent(line: int, shape: int) -> None
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `line`    | `int`  |             |
| `shape`   | `int`  |             |
| _Returns_ | `None` |             |

### `constrain_vertical()`

```python
constrain_vertical(p1: int, p2: int) -> None
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `p1`      | `int`  |             |
| `p2`      | `int`  |             |
| _Returns_ | `None` |             |

### `create_fill_geometry()`

```python
create_fill_geometry(fill: Any, exclude_ids: set[int]) -> Optional[Any]
```

| Parameter     | Type            | Description |
| ------------- | --------------- | ----------- |
| `fill`        | `Any`           |             |
| `exclude_ids` | `set[int]`      |             |
| _Returns_     | `Optional[Any]` |             |

### `find_all_closed_loops()`

```python
find_all_closed_loops() -> list[list[tuple[int, bool]]]
```

| Parameter | Type                           | Description |
| --------- | ------------------------------ | ----------- |
| _Returns_ | `list[list[tuple[int, bool]]]` |             |

### `find_axis_coincidence_pairs()`

```python
find_axis_coincidence_pairs() -> set[int]
```

| Parameter | Type       | Description |
| --------- | ---------- | ----------- |
| _Returns_ | `set[int]` |             |

### `find_standalone_point_ids()`

```python
find_standalone_point_ids(entity_ids: Sequence[int]) -> list[int]
```

| Parameter    | Type            | Description |
| ------------ | --------------- | ----------- |
| `entity_ids` | `Sequence[int]` |             |
| _Returns_    | `list[int]`     |             |

### `finish_solve()`

```python
finish_solve() -> None
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `from_dict()`

```python
from_dict(data: dict) -> Sketch
```

| Parameter | Type     | Description |
| --------- | -------- | ----------- |
| `data`    | `dict`   |             |
| _Returns_ | `Sketch` |             |

### `get_coincident_points()`

```python
get_coincident_points(pid: int) -> set[int]
```

| Parameter | Type       | Description |
| --------- | ---------- | ----------- |
| `pid`     | `int`      |             |
| _Returns_ | `set[int]` |             |

### `get_derived_point_ids()`

```python
get_derived_point_ids() -> set[int]
```

| Parameter | Type       | Description |
| --------- | ---------- | ----------- |
| _Returns_ | `set[int]` |             |

### `get_internal_constraints()`

```python
get_internal_constraints(entity_ids: Any) -> list
```

| Parameter    | Type   | Description |
| ------------ | ------ | ----------- |
| `entity_ids` | `Any`  |             |
| _Returns_    | `list` |             |

### `get_loop_at_point()`

```python
get_loop_at_point(mx: float, my: float) -> Optional[list[tuple[int, bool]]]
```

| Parameter | Type                               | Description |
| --------- | ---------------------------------- | ----------- |
| `mx`      | `float`                            |             |
| `my`      | `float`                            |             |
| _Returns_ | `Optional[list[tuple[int, bool]]]` |             |

### `has_conflicts()`

```python
has_conflicts() -> bool
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `is_array_guide_radius_point()`

```python
is_array_guide_radius_point(pid: int) -> bool
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `pid`     | `int`  |             |
| _Returns_ | `bool` |             |

### `is_empty()`

```python
is_empty() -> bool
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `is_fully_constrained()`

```python
is_fully_constrained() -> bool
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `move_point()`

```python
move_point(pid: int, x: float, y: float) -> bool
```

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `pid`     | `int`   |             |
| `x`       | `float` |             |
| `y`       | `float` |             |
| _Returns_ | `bool`  |             |

### `prune_arrays()`

```python
prune_arrays() -> None
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `remove_entities()`

```python
remove_entities(entities: Any) -> None
```

| Parameter  | Type   | Description |
| ---------- | ------ | ----------- |
| `entities` | `Any`  |             |
| _Returns_  | `None` |             |

### `remove_point_if_unused()`

```python
remove_point_if_unused(pid: Optional[int] = None) -> bool
```

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| `pid`     | `Optional[int] = None` |             |
| _Returns_ | `bool`                 |             |

### `set_param()`

```python
set_param(name: str, value: Any) -> None
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `name`    | `str`  |             |
| `value`   | `Any`  |             |
| _Returns_ | `None` |             |

### `solve()`

```python
solve(
    extra_constraints: Optional[Sequence[Any]] = None,
    update_constraint_status: bool = True,
    variable_overrides: Optional[Any] = None,
    excluded_constraints: Optional[set[int]] = None,
    point_scope: Optional[set[int]] = None,
    initial_values: Optional[dict] = None,
    template_functions: Optional[dict] = None,
) -> bool
```

| Parameter                  | Type                             | Description |
| -------------------------- | -------------------------------- | ----------- |
| `extra_constraints`        | `Optional[Sequence[Any]] = None` |             |
| `update_constraint_status` | `bool = True`                    |             |
| `variable_overrides`       | `Optional[Any] = None`           |             |
| `excluded_constraints`     | `Optional[set[int]] = None`      |             |
| `point_scope`              | `Optional[set[int]] = None`      |             |
| `initial_values`           | `Optional[dict] = None`          |             |
| `template_functions`       | `Optional[dict] = None`          |             |
| _Returns_                  | `bool`                           |             |

### `to_dict()`

```python
to_dict() -> dict
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

### `to_geometry()`

```python
to_geometry(resolved_text: Optional[dict] = None) -> Any
```

| Parameter       | Type                    | Description |
| --------------- | ----------------------- | ----------- |
| `resolved_text` | `Optional[dict] = None` |             |
| _Returns_       | `Any`                   |             |

### `validate_fills()`

```python
validate_fills() -> None
```

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

## Solver

Trust-region least-squares solver for sketch constraint systems.

### `conflicting()`

```python
conflicting(threshold: float) -> list[int]
```

Indices of hard constraints whose residual exceeds `threshold` at the final positions.

| Parameter   | Type        | Description |
| ----------- | ----------- | ----------- |
| `threshold` | `float`     |             |
| _Returns_   | `list[int]` |             |

### `constrained()`

```python
constrained() -> list[bool]
```

Per-point fully-constrained flags, aligned with the input point order.

| Parameter | Type         | Description |
| --------- | ------------ | ----------- |
| _Returns_ | `list[bool]` |             |

### `jacobian()`

```python
jacobian() -> list[list[float]]
```

Dense Jacobian rows of the hard constraints at the current positions, over the state points
(non-fixed points touched by any constraint, in input order). For tests and debugging.

| Parameter | Type                | Description |
| --------- | ------------------- | ----------- |
| _Returns_ | `list[list[float]]` |             |

### `max_residuals()`

```python
max_residuals() -> list[float]
```

Maximum absolute residual per hard constraint.

| Parameter | Type          | Description |
| --------- | ------------- | ----------- |
| _Returns_ | `list[float]` |             |

### `nfev()`

```python
nfev() -> int
```

Residual function evaluations of the last solve.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `int` |             |

### `positions()`

```python
positions() -> list[tuple[float, float]]
```

Final `(x, y)` positions, aligned with the input point order.

| Parameter | Type                        | Description |
| --------- | --------------------------- | ----------- |
| _Returns_ | `list[tuple[float, float]]` |             |

### `residuals()`

```python
residuals() -> list[float]
```

All residual values of the hard constraints at the current positions, in constraint order.

| Parameter | Type          | Description |
| --------- | ------------- | ----------- |
| _Returns_ | `list[float]` |             |

### `solve()`

```python
solve(tolerance: float, update_dof: bool) -> bool
```

Runs the solve. Returns True when all components converged.

| Parameter    | Type    | Description                                                                 |
| ------------ | ------- | --------------------------------------------------------------------------- |
| `tolerance`  | `float` | Cost threshold and ftol of the optimizer.                                   |
| `update_dof` | `bool`  | Whether to re-analyze per-point constrained flags after a successful solve. |
| _Returns_    | `bool`  |                                                                             |

### `status()`

```python
status() -> int
```

Termination status of the last solve (>0 means success, mirroring SciPy: 1 gtol, 2 ftol, 3 xtol, 4
both, 0 max_nfev).

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `int` |             |

## Functions

### `conflicting_constraints()`

```python
conflicting_constraints(
    registry: registry.EntityRegistry,
    constraints: Sequence[Any],
    threshold: float = 0.001,
) -> list[int]
```

Indices of hard constraints whose residual exceeds `threshold` at the registry's current positions.

| Parameter     | Type                      | Description |
| ------------- | ------------------------- | ----------- |
| `registry`    | `registry.EntityRegistry` |             |
| `constraints` | `Sequence[Any]`           |             |
| `threshold`   | `float = 0.001`           |             |
| _Returns_     | `list[int]`               |             |

### `solve_registry()`

```python
solve_registry(
    registry: registry.EntityRegistry,
    constraints: Sequence[Any],
    auxiliary_constraints: Optional[Sequence[Any]] = None,
    point_filter: Optional[Sequence[int]] = None,
    tolerance: float = 1e-05,
    update_dof: bool = True,
) -> bool
```

Solves the registry's points in place and refreshes constrained flags.

This is the coarse-grained binding the host calls once per solve: it snapshots the registry, runs
the Rust solver, writes positions and DOF flags back, and refreshes entity constrained flags.

| Parameter               | Type                             | Description |
| ----------------------- | -------------------------------- | ----------- |
| `registry`              | `registry.EntityRegistry`        |             |
| `constraints`           | `Sequence[Any]`                  |             |
| `auxiliary_constraints` | `Optional[Sequence[Any]] = None` |             |
| `point_filter`          | `Optional[Sequence[int]] = None` |             |
| `tolerance`             | `float = 1e-05`                  |             |
| `update_dof`            | `bool = True`                    |             |
| _Returns_               | `bool`                           |             |
