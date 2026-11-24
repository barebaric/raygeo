---
title: raygeo.sketch.constraints
sidebar_label: raygeo.sketch.constraints
---

## Constants

### ANGLE_WEIGHT

`float`

## Constraint

The common base class of every constraint binding, carrying the reference `Constraint` base surface.
The per-class members live on the subclasses; polymorphism is carried by the Rust-side
`SketchConstraint` trait.

### `expression`

```python
expression: Optional[str]
```

The stored expression string (dimensional constraints).

### `status`

```python
status: int
```

The reference validation status ordinal.

### `user_visible`

```python
user_visible: bool
```

Reference user-visibility flag.

### `value`

```python
value: float
```

The resolved dimension value (dimensional constraints).

### `applies_to_entities()`

```python
@classmethod applies_to_entities(entities: Any) -> bool
```

Whether the constraint type can apply to the given entities.

| Parameter  | Type   | Description |
| ---------- | ------ | ----------- |
| `entities` | `Any`  |             |
| _Returns_  | `bool` |             |

### `can_apply_to()`

```python
@classmethod
can_apply_to(
    _selection: Any,
    _sketch: Optional[Any] = None,
) -> bool
```

Whether the constraint can be applied to the selection.

| Parameter    | Type                   | Description |
| ------------ | ---------------------- | ----------- |
| `_selection` | `Any`                  |             |
| `_sketch`    | `Optional[Any] = None` |             |
| _Returns_    | `bool`                 |             |

### `constrains_radius()`

```python
constrains_radius(registry: protocols.Registry, entity_id: int) -> bool
```

Whether the constraint explicitly defines the radius/length of the given entity.

| Parameter   | Type                 | Description |
| ----------- | -------------------- | ----------- |
| `registry`  | `protocols.Registry` |             |
| `entity_id` | `int`                |             |
| _Returns_   | `bool`               |             |

### `depends_on_entities()`

```python
depends_on_entities(entity_ids: Any) -> bool
```

Whether the constraint references any of the given entities.

| Parameter    | Type   | Description |
| ------------ | ------ | ----------- |
| `entity_ids` | `Any`  |             |
| _Returns_    | `bool` |             |

### `depends_on_points()`

```python
depends_on_points(point_ids: Any) -> bool
```

Whether the constraint references any of the given points.

| Parameter   | Type   | Description |
| ----------- | ------ | ----------- |
| `point_ids` | `Any`  |             |
| _Returns_   | `bool` |             |

### `draw()`

```python
draw(
    ctx: protocols.Canvas,
    registry: Any,
    to_screen: Callable[[tuple[float, float]], tuple[float, float]],
    is_selected: bool = False,
    is_hovered: bool = False,
    point_radius: float = 5.0,
) -> None
```

Draws the constraint's visual representation.

| Parameter      | Type                                                   | Description |
| -------------- | ------------------------------------------------------ | ----------- |
| `ctx`          | `protocols.Canvas`                                     |             |
| `registry`     | `Any`                                                  |             |
| `to_screen`    | `Callable[[tuple[float, float]], tuple[float, float]]` |             |
| `is_selected`  | `bool = False`                                         |             |
| `is_hovered`   | `bool = False`                                         |             |
| `point_radius` | `float = 5.0`                                          |             |
| _Returns_      | `None`                                                 |             |

### `error()`

```python
error(reg: protocols.Registry, params: Optional[Any] = None) -> float
```

The error of the constraint at the registry's positions.

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| `reg`     | `protocols.Registry`   |             |
| `params`  | `Optional[Any] = None` |             |
| _Returns_ | `float`                |             |

### `get_draggable_point()`

```python
get_draggable_point() -> Optional[int]
```

A point id that can be dragged to manipulate this constraint.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| _Returns_ | `Optional[int]` |             |

### `get_edit_subtitle()`

```python
get_edit_subtitle() -> str
```

The user-facing hint for the constraint edit dialog.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `str` |             |

### `get_referenced_entity_ids()`

```python
get_referenced_entity_ids() -> set[int]
```

The set of entity ids this constraint references.

| Parameter | Type       | Description |
| --------- | ---------- | ----------- |
| _Returns_ | `set[int]` |             |

### `get_referenced_point_ids()`

```python
get_referenced_point_ids() -> set[int]
```

The set of point ids this constraint references.

| Parameter | Type       | Description |
| --------- | ---------- | ----------- |
| _Returns_ | `set[int]` |             |

### `get_subtitle()`

```python
get_subtitle(registry: protocols.Registry) -> str
```

A human-readable subtitle describing the constrained entities.

| Parameter  | Type                 | Description |
| ---------- | -------------------- | ----------- |
| `registry` | `protocols.Registry` |             |
| _Returns_  | `str`                |             |

### `get_title()`

```python
get_title() -> str
```

A human-readable title for this constraint.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `str` |             |

### `get_type_key()`

```python
@classmethod get_type_key() -> Optional[str]
```

The string key used to identify this constraint type; None for constraints that cannot be created by
users.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| _Returns_ | `Optional[str]` |             |

### `get_type_name()`

```python
get_type_name() -> str
```

The human-readable name of this constraint type.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `str` |             |

### `gradient()`

```python
gradient(
    reg: protocols.Registry,
    params: Optional[Any] = None,
) -> dict[int, list[tuple[float, float]]]
```

The partial derivatives of the error by point id.

| Parameter | Type                                   | Description |
| --------- | -------------------------------------- | ----------- |
| `reg`     | `protocols.Registry`                   |             |
| `params`  | `Optional[Any] = None`                 |             |
| _Returns_ | `dict[int, list[tuple[float, float]]]` |             |

### `is_hit()`

```python
is_hit(
    sx: float,
    sy: float,
    registry: protocols.Registry,
    to_screen: Callable[[tuple[float, float]], tuple[float, float]],
    element: Optional[Any],
    threshold: float,
) -> bool
```

Whether the constraint's visual representation is hit.

| Parameter   | Type                                                   | Description |
| ----------- | ------------------------------------------------------ | ----------- |
| `sx`        | `float`                                                |             |
| `sy`        | `float`                                                |             |
| `registry`  | `protocols.Registry`                                   |             |
| `to_screen` | `Callable[[tuple[float, float]], tuple[float, float]]` |             |
| `element`   | `Optional[Any]`                                        |             |
| `threshold` | `float`                                                |             |
| _Returns_   | `bool`                                                 |             |

### `is_mirror_compatible()`

```python
is_mirror_compatible() -> bool
```

Whether the constraint survives a mirror of all referenced geometry.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `is_world_anchored()`

```python
is_world_anchored() -> bool
```

Whether the constraint pins geometry to the global axes.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `mirror()`

```python
mirror(_axis: Any) -> None
```

Updates the internal state for a mirror transform; a no-op for mirror-invariant constraints.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `_axis`   | `Any`  |             |
| _Returns_ | `None` |             |

### `remap_references()`

```python
remap_references(_id_map: Mapping[int, int]) -> None
```

Rewrites every id reference through *id_map*; concrete constraints override this.

| Parameter | Type                | Description |
| --------- | ------------------- | ----------- |
| `_id_map` | `Mapping[int, int]` |             |
| _Returns_ | `None`              |             |

### `targets_segment()`

```python
targets_segment(p1: int, p2: int, entity_id: Optional[int] = None) -> bool
```

Whether this constraint restricts the length/distance of the segment (p1, p2) or of the given
entity.

| Parameter   | Type                   | Description |
| ----------- | ---------------------- | ----------- |
| `p1`        | `int`                  |             |
| `p2`        | `int`                  |             |
| `entity_id` | `Optional[int] = None` |             |
| _Returns_   | `bool`                 |             |

### `to_dict()`

```python
to_dict() -> dict
```

Serializes the constraint to a dictionary.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

### `update_from_context()`

```python
update_from_context(context: dict) -> None
```

Re-evaluates the expression (if present) using the provided context and updates `value` and
`status`.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `context` | `dict` |             |
| _Returns_ | `None` |             |

## Functions

### `check_grad()`

```python
check_grad(
    func: Callable[[numpy.NDArray[numpy.float64]], float],
    grad: Callable[[numpy.NDArray[numpy.float64]], numpy.NDArray[numpy.float64]],
    x0: numpy.ArrayLike,
    epsilon: float = 1e-06,
) -> float
```

Compare an analytical gradient with forward differences.

Mirrors `scipy.optimize.check_grad`: the analytical gradient is evaluated at `x0`, each coordinate
is then perturbed by `epsilon` and `func` is re-evaluated, and the L2 norm of the difference between
the analytical and numerical gradients is returned.

| Parameter    | Type                                                                     | Description                             |
| ------------ | ------------------------------------------------------------------------ | --------------------------------------- |
| `func`       | `Callable[[numpy.NDArray[numpy.float64]], float]`                        | Scalar function of a 1-D float64 array. |
| `grad`       | `Callable[[numpy.NDArray[numpy.float64]], numpy.NDArray[numpy.float64]]` | Analytical gradient of `func`.          |
| `x0`         | `numpy.ArrayLike`                                                        | Point at which to check the gradient.   |
| `epsilon`    | `float = 1e-06`                                                          | Forward-difference step size.           |
| _Returns_    | `float`                                                                  | L2 norm of the gradient difference.     |
| _Complexity_ |                                                                          | O(n) function and gradient evaluations  |
