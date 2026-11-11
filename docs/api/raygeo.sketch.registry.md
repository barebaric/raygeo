---
title: raygeo.sketch.registry
sidebar_label: raygeo.sketch.registry
---

Sketch document model.

Model is the registry of points and entities of a sketch document: it stores the same data as the
reference entity classes and serializes to and from the same dictionaries (`.ryp` compatible).
Entities are addressed by id; use add_line/add_arc/add_circle/
add_ellipse/add_bezier/add_polygon/add_text_box to extend the document and
entity_point_ids/entity_endpoint_ids/entity_type for polymorphic queries.

## EntityRegistry

The sketch document model: points, entities, id allocation.

Serializes to and from the same dictionaries the reference entities produce (`.ryp` compatible).

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

Adds an arc and returns its id.

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

Adds a bezier and returns its id.

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
add_circle(center: int, radius_point: int, construction: bool = False) -> int
```

Adds a circle and returns its id.

| Parameter      | Type           | Description |
| -------------- | -------------- | ----------- |
| `center`       | `int`          |             |
| `radius_point` | `int`          |             |
| `construction` | `bool = False` |             |
| _Returns_      | `int`          |             |

### `add_constraint_json()`

```python
add_constraint_json(dict: dict) -> int
```

Adds a constraint from a reference constraint dictionary (`.ryp` shape), resolving entity references
through the registry. Returns the constraint handle.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `dict`    | `dict` |             |
| _Returns_ | `int`  |             |

### `add_ellipse()`

```python
add_ellipse(
    center: int,
    radius_x_point: int,
    radius_y_point: int,
    construction: bool = False,
) -> int
```

Adds an ellipse and returns its id.

| Parameter        | Type           | Description |
| ---------------- | -------------- | ----------- |
| `center`         | `int`          |             |
| `radius_x_point` | `int`          |             |
| `radius_y_point` | `int`          |             |
| `construction`   | `bool = False` |             |
| _Returns_        | `int`          |             |

### `add_line()`

```python
add_line(p1: int, p2: int, construction: bool = False) -> int
```

Adds a line and returns its id.

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

Adds a point and returns its id.

| Parameter | Type           | Description |
| --------- | -------------- | ----------- |
| `x`       | `float`        |             |
| `y`       | `float`        |             |
| `fixed`   | `bool = False` |             |
| _Returns_ | `int`          |             |

### `add_points()`

```python
add_points(points: Sequence[tuple[int, float, float, bool]]) -> None
```

Appends pre-built points given as `(id, x, y, fixed)` tuples.

| Parameter | Type                                       | Description |
| --------- | ------------------------------------------ | ----------- |
| `points`  | `Sequence[tuple[int, float, float, bool]]` |             |
| _Returns_ | `None`                                     |             |

### `add_polygon()`

```python
add_polygon(
    center: int,
    handle: int,
    vertices: Sequence[tuple[float, float]],
    closed: bool = False,
    construction: bool = False,
    rings: Optional[Sequence[Sequence[tuple[float, float]]]] = None,
) -> int
```

Adds a polygon and returns its id.

| Parameter      | Type                                                       | Description |
| -------------- | ---------------------------------------------------------- | ----------- |
| `center`       | `int`                                                      |             |
| `handle`       | `int`                                                      |             |
| `vertices`     | `Sequence[tuple[float, float]]`                            |             |
| `closed`       | `bool = False`                                             |             |
| `construction` | `bool = False`                                             |             |
| `rings`        | `Optional[Sequence[Sequence[tuple[float, float]]]] = None` |             |
| _Returns_      | `int`                                                      |             |

### `add_text_box()`

```python
add_text_box(
    origin: int,
    width: int,
    height: int,
    content: str = '',
    font_family: str = 'sans-serif',
    font_size: float = 10.0,
    bold: bool = False,
    italic: bool = False,
) -> int
```

Adds a text box and returns its id.

| Parameter     | Type                 | Description |
| ------------- | -------------------- | ----------- |
| `origin`      | `int`                |             |
| `width`       | `int`                |             |
| `height`      | `int`                |             |
| `content`     | `str = ''`           |             |
| `font_family` | `str = 'sans-serif'` |             |
| `font_size`   | `float = 10.0`       |             |
| `bold`        | `bool = False`       |             |
| `italic`      | `bool = False`       |             |
| _Returns_     | `int`                |             |

### `append_to_geometry()`

```python
append_to_geometry(geo: geo.Geometry, id: int, forward: bool) -> None
```

Appends the entity's geometry (traversal direction aware) to an existing Geometry object.

| Parameter | Type           | Description |
| --------- | -------------- | ----------- |
| `geo`     | `geo.Geometry` |             |
| `id`      | `int`          |             |
| `forward` | `bool`         |             |
| _Returns_ | `None`         |             |

### `characteristic_length()`

```python
characteristic_length(id: int) -> float
```

Characteristic length used by equal-length constraints.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `id`      | `int`   |             |
| _Returns_ | `float` |             |

### `clear_helper_references()`

```python
clear_helper_references(id: int) -> None
```

Drops the entity's references to its helper entities.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `id`      | `int`  |             |
| _Returns_ | `None` |             |

### `constraint_count()`

```python
constraint_count() -> int
```

Number of stored constraints.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `int` |             |

### `constraint_dict()`

```python
constraint_dict(index: int) -> dict
```

The stored reference dictionary of a constraint.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `index`   | `int`  |             |
| _Returns_ | `dict` |             |

### `constraint_residuals()`

```python
constraint_residuals() -> list[float]
```

Residual values of all stored constraints at the current point positions.

| Parameter | Type          | Description |
| --------- | ------------- | ----------- |
| _Returns_ | `list[float]` |             |

### `contains_point()`

```python
contains_point(id: int, x: float, y: float) -> bool
```

Whether a point is inside the entity (closed shapes).

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `id`      | `int`   |             |
| `x`       | `float` |             |
| `y`       | `float` |             |
| _Returns_ | `bool`  |             |

### `drag_anchor_points()`

```python
drag_anchor_points(point_id: int) -> list[int]
```

Points pinned while `point_id` is dragged.

| Parameter  | Type        | Description |
| ---------- | ----------- | ----------- |
| `point_id` | `int`       |             |
| _Returns_  | `list[int]` |             |

### `enclosed_signed_area()`

```python
enclosed_signed_area(id: int) -> Optional[float]
```

Signed area enclosed by a closed single-entity loop.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `id`      | `int`             |             |
| _Returns_ | `Optional[float]` |             |

### `entity_count()`

```python
entity_count() -> int
```

Entity count.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `int` |             |

### `entity_endpoint_ids()`

```python
entity_endpoint_ids(id: int) -> Optional[list[int]]
```

Endpoint ids of an edge entity, or `None`.

| Parameter | Type                  | Description |
| --------- | --------------------- | ----------- |
| `id`      | `int`                 |             |
| _Returns_ | `Optional[list[int]]` |             |

### `entity_ids()`

```python
entity_ids() -> list[int]
```

All entity ids in insertion order.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[int]` |             |

### `entity_point_ids()`

```python
entity_point_ids(id: int) -> Optional[list[int]]
```

Control point ids of an entity, or `None`.

| Parameter | Type                  | Description |
| --------- | --------------------- | ----------- |
| `id`      | `int`                 |             |
| _Returns_ | `Optional[list[int]]` |             |

### `entity_to_dict()`

```python
entity_to_dict(id: int) -> dict
```

Serializes the entity to a reference entity dictionary.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `id`      | `int`  |             |
| _Returns_ | `dict` |             |

### `entity_type()`

```python
entity_type(id: int) -> Optional[str]
```

The serialization type string of an entity, or `None`.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| `id`      | `int`           |             |
| _Returns_ | `Optional[str]` |             |

### `entity_version()`

```python
entity_version() -> int
```

Monotonic entity-list version.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `int` |             |

### `from_dict()`

```python
from_dict(dict: dict) -> EntityRegistry
```

Deserializes the model from a reference registry dictionary.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `dict`    | `dict`           |             |
| _Returns_ | `EntityRegistry` |             |

### `geometry_signature()`

```python
geometry_signature(id: int) -> Any
```

The entity's geometry signature (quantized defining points plus kind-specific state), or `None` for
unknown ids.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| `id`      | `int` |             |
| _Returns_ | `Any` |             |

### `get_point()`

```python
get_point(id: int) -> Optional[tuple[float, float]]
```

Coordinates of a point as `(x, y)`, or `None`.

| Parameter | Type                            | Description |
| --------- | ------------------------------- | ----------- |
| `id`      | `int`                           |             |
| _Returns_ | `Optional[tuple[float, float]]` |             |

### `get_state()`

```python
get_state(id: int) -> Any
```

Solver-relevant state snapshot of an entity, or `None`.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| `id`      | `int` |             |
| _Returns_ | `Any` |             |

### `helper_ids()`

```python
helper_ids(id: int) -> list[int]
```

Helper/child ids of a compound entity.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| `id`      | `int`       |             |
| _Returns_ | `list[int]` |             |

### `hit_test()`

```python
hit_test(id: int, mx: float, my: float, threshold: float) -> bool
```

Whether the point lies within threshold distance of the entity.

| Parameter   | Type    | Description |
| ----------- | ------- | ----------- |
| `id`        | `int`   |             |
| `mx`        | `float` |             |
| `my`        | `float` |             |
| `threshold` | `float` |             |
| _Returns_   | `bool`  |             |

### `ignorable_unconstrained_points()`

```python
ignorable_unconstrained_points(id: int) -> list[int]
```

Points that can stay unconstrained when the entity is constrained.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| `id`      | `int`       |             |
| _Returns_ | `list[int]` |             |

### `intersects_rect()`

```python
intersects_rect(id: int, rect: Sequence[float]) -> bool
```

Whether the entity intersects the rect or is contained by it.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `id`      | `int`             |             |
| `rect`    | `Sequence[float]` |             |
| _Returns_ | `bool`            |             |

### `is_closed_loop()`

```python
is_closed_loop(id: int) -> bool
```

Whether the entity alone forms a closed loop.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `id`      | `int`  |             |
| _Returns_ | `bool` |             |

### `is_constrained()`

```python
is_constrained(id: int) -> bool
```

Whether the entity is fully constrained.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `id`      | `int`  |             |
| _Returns_ | `bool` |             |

### `is_construction()`

```python
is_construction(id: int) -> bool
```

Whether the entity is a construction entity.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `id`      | `int`  |             |
| _Returns_ | `bool` |             |

### `is_contained_by()`

```python
is_contained_by(id: int, rect: Sequence[float]) -> bool
```

Whether the entity is fully strictly contained in the rect.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `id`      | `int`             |             |
| `rect`    | `Sequence[float]` |             |
| _Returns_ | `bool`            |             |

### `is_edge_entity()`

```python
is_edge_entity(id: int) -> bool
```

Whether the entity participates in edge/chaining adjacency.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `id`      | `int`  |             |
| _Returns_ | `bool` |             |

### `is_point_used()`

```python
is_point_used(id: int) -> bool
```

Whether the point is referenced by any entity.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `id`      | `int`  |             |
| _Returns_ | `bool` |             |

### `is_radius_entity()`

```python
is_radius_entity(id: int) -> bool
```

Whether the entity has a radius.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `id`      | `int`  |             |
| _Returns_ | `bool` |             |

### `junction_point_ids()`

```python
junction_point_ids(id: int) -> list[int]
```

Junction point ids of an entity.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| `id`      | `int`       |             |
| _Returns_ | `list[int]` |             |

### `mirror()`

```python
mirror(id: int, direction: str, position: float) -> None
```

Mirrors entity-internal state across the axis (arc chirality, bezier offsets, polygon winding).

| Parameter   | Type    | Description |
| ----------- | ------- | ----------- |
| `id`        | `int`   |             |
| `direction` | `str`   |             |
| `position`  | `float` |             |
| _Returns_   | `None`  |             |

### `point_constrained()`

```python
point_constrained(id: int) -> bool
```

Whether the point is solver-constrained.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `id`      | `int`  |             |
| _Returns_ | `bool` |             |

### `point_count()`

```python
point_count() -> int
```

Point count.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `int` |             |

### `point_ids()`

```python
point_ids() -> list[int]
```

All point ids in insertion order.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[int]` |             |

### `remove_constraint()`

```python
remove_constraint(index: int) -> None
```

Removes a constraint by handle.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `index`   | `int`  |             |
| _Returns_ | `None` |             |

### `remove_entities()`

```python
remove_entities(entity_ids: Sequence[int]) -> None
```

Removes entities by id.

| Parameter    | Type            | Description |
| ------------ | --------------- | ----------- |
| `entity_ids` | `Sequence[int]` |             |
| _Returns_    | `None`          |             |

### `remove_points()`

```python
remove_points(point_ids: Sequence[int]) -> None
```

Removes the given points.

| Parameter   | Type            | Description |
| ----------- | --------------- | ----------- |
| `point_ids` | `Sequence[int]` |             |
| _Returns_   | `None`          |             |

### `rigidly_connected_points()`

```python
rigidly_connected_points(point_id: int) -> list[int]
```

Points that move rigidly with `point_id`.

| Parameter  | Type        | Description |
| ---------- | ----------- | ----------- |
| `point_id` | `int`       |             |
| _Returns_  | `list[int]` |             |

### `set_construction()`

```python
set_construction(id: int, construction: bool) -> None
```

Sets the construction flag.

| Parameter      | Type   | Description |
| -------------- | ------ | ----------- |
| `id`           | `int`  |             |
| `construction` | `bool` |             |
| _Returns_      | `None` |             |

### `set_point_constrained()`

```python
set_point_constrained(id: int, constrained: bool) -> bool
```

Sets a point's solver-constrained flag.

| Parameter     | Type   | Description |
| ------------- | ------ | ----------- |
| `id`          | `int`  |             |
| `constrained` | `bool` |             |
| _Returns_     | `bool` |             |

### `set_point_position()`

```python
set_point_position(id: int, x: float, y: float) -> bool
```

Sets a point's coordinates.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `id`      | `int`   |             |
| `x`       | `float` |             |
| `y`       | `float` |             |
| _Returns_ | `bool`  |             |

### `set_state()`

```python
set_state(id: int, state: dict) -> None
```

Restores an entity's state snapshot.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `id`      | `int`  |             |
| `state`   | `dict` |             |
| _Returns_ | `None` |             |

### `signed_distance_to()`

```python
signed_distance_to(id: int, px: float, py: float) -> Optional[float]
```

Signed distance from a point to the entity's locus, or `None` when unsupported.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `id`      | `int`             |             |
| `px`      | `float`           |             |
| `py`      | `float`           |             |
| _Returns_ | `Optional[float]` |             |

### `supports_point_on_curve()`

```python
supports_point_on_curve(id: int) -> bool
```

Whether a point can be constrained onto this entity's curve.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `id`      | `int`  |             |
| _Returns_ | `bool` |             |

### `tangent_at()`

```python
tangent_at(id: int, point_id: int) -> Optional[tuple[float, float]]
```

Tangent vector at a defining point, or `None` when the entity has no directional tangent.

| Parameter  | Type                            | Description |
| ---------- | ------------------------------- | ----------- |
| `id`       | `int`                           |             |
| `point_id` | `int`                           |             |
| _Returns_  | `Optional[tuple[float, float]]` |             |

### `to_dict()`

```python
to_dict() -> dict
```

Serializes the model to a reference registry dictionary.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

### `to_geometry()`

```python
to_geometry(id: int) -> geo.Geometry
```

The entity as a raygeo Geometry object.

| Parameter | Type           | Description |
| --------- | -------------- | ----------- |
| `id`      | `int`          |             |
| _Returns_ | `geo.Geometry` |             |

### `to_polygon_vertices()`

```python
to_polygon_vertices(id: int, forward: bool) -> list[tuple[float, float]]
```

Polygon vertices of the entity (sampled for curves).

| Parameter | Type                        | Description |
| --------- | --------------------------- | ----------- |
| `id`      | `int`                       |             |
| `forward` | `bool`                      |             |
| _Returns_ | `list[tuple[float, float]]` |             |

### `to_polyline()`

```python
to_polyline(id: int, tolerance: float = 0.1) -> list[tuple[float, float]]
```

Sampled polyline of the entity.

| Parameter   | Type                        | Description |
| ----------- | --------------------------- | ----------- |
| `id`        | `int`                       |             |
| `tolerance` | `float = 0.1`               |             |
| _Returns_   | `list[tuple[float, float]]` |             |

### `to_polylines()`

```python
to_polylines(
    id: int,
    tolerance: float = 0.1,
) -> list[list[tuple[float, float]]]
```

Sampled polylines of the entity (multi-contour for text).

| Parameter   | Type                              | Description |
| ----------- | --------------------------------- | ----------- |
| `id`        | `int`                             |             |
| `tolerance` | `float = 0.1`                     |             |
| _Returns_   | `list[list[tuple[float, float]]]` |             |

### `update_constrained_status()`

```python
update_constrained_status(
    radius_constrained_entity_ids: Sequence[int] = [],
) -> None
```

Re-computes constrained flags. Entities whose radius is explicitly dimensioned are listed in
`radius_constrained_entity_ids`.

| Parameter                       | Type                 | Description |
| ------------------------------- | -------------------- | ----------- |
| `radius_constrained_entity_ids` | `Sequence[int] = []` |             |
| _Returns_                       | `None`               |             |

### `waypoint_type()`

```python
waypoint_type(id: int) -> Optional[str]
```

Waypoint type string of a point ("sharp"/"smooth"/"symmetric").

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| `id`      | `int`           |             |
| _Returns_ | `Optional[str]` |             |
