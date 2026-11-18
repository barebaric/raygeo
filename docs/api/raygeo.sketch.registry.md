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
    start: types.EntityID,
    end: types.EntityID,
    center: types.EntityID,
    clockwise: bool = False,
    construction: bool = False,
) -> types.EntityID
```

Adds an arc and returns its id.

| Parameter      | Type             | Description |
| -------------- | ---------------- | ----------- |
| `start`        | `types.EntityID` |             |
| `end`          | `types.EntityID` |             |
| `center`       | `types.EntityID` |             |
| `clockwise`    | `bool = False`   |             |
| `construction` | `bool = False`   |             |
| _Returns_      | `types.EntityID` |             |

### `add_bezier()`

```python
add_bezier(
    start: types.EntityID,
    end: types.EntityID,
    construction: bool = False,
    cp1: Optional[tuple[float, float]] = None,
    cp2: Optional[tuple[float, float]] = None,
) -> types.EntityID
```

Adds a bezier and returns its id.

| Parameter      | Type                                   | Description |
| -------------- | -------------------------------------- | ----------- |
| `start`        | `types.EntityID`                       |             |
| `end`          | `types.EntityID`                       |             |
| `construction` | `bool = False`                         |             |
| `cp1`          | `Optional[tuple[float, float]] = None` |             |
| `cp2`          | `Optional[tuple[float, float]] = None` |             |
| _Returns_      | `types.EntityID`                       |             |

### `add_circle()`

```python
add_circle(
    center: types.EntityID,
    radius_point: types.EntityID,
    construction: bool = False,
) -> types.EntityID
```

Adds a circle and returns its id.

| Parameter      | Type             | Description |
| -------------- | ---------------- | ----------- |
| `center`       | `types.EntityID` |             |
| `radius_point` | `types.EntityID` |             |
| `construction` | `bool = False`   |             |
| _Returns_      | `types.EntityID` |             |

### `add_ellipse()`

```python
add_ellipse(
    center: types.EntityID,
    radius_x_point: types.EntityID,
    radius_y_point: types.EntityID,
    construction: bool = False,
) -> types.EntityID
```

Adds an ellipse and returns its id.

| Parameter        | Type             | Description |
| ---------------- | ---------------- | ----------- |
| `center`         | `types.EntityID` |             |
| `radius_x_point` | `types.EntityID` |             |
| `radius_y_point` | `types.EntityID` |             |
| `construction`   | `bool = False`   |             |
| _Returns_        | `types.EntityID` |             |

### `add_entity()`

```python
add_entity(obj: Any) -> int
```

Adds an entity object to the registry and returns its id.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| `obj`     | `Any` |             |
| _Returns_ | `int` |             |

### `add_line()`

```python
add_line(
    p1: types.EntityID,
    p2: types.EntityID,
    construction: bool = False,
) -> types.EntityID
```

Adds a line and returns its id.

| Parameter      | Type             | Description |
| -------------- | ---------------- | ----------- |
| `p1`           | `types.EntityID` |             |
| `p2`           | `types.EntityID` |             |
| `construction` | `bool = False`   |             |
| _Returns_      | `types.EntityID` |             |

### `add_point()`

```python
add_point(x: float, y: float, fixed: bool = False) -> types.EntityID
```

Adds a point and returns its id.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `x`       | `float`          |             |
| `y`       | `float`          |             |
| `fixed`   | `bool = False`   |             |
| _Returns_ | `types.EntityID` |             |

### `add_points()`

```python
add_points(points: Any) -> None
```

Appends pre-built points given as `(id, x, y, fixed)` tuples.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `points`  | `Any`  |             |
| _Returns_ | `None` |             |

### `add_polygon()`

```python
add_polygon(
    center: types.EntityID,
    handle: types.EntityID,
    vertices: Sequence[tuple[float, float]],
    closed: bool = False,
    construction: bool = False,
    rings: Optional[Sequence[Sequence[tuple[float, float]]]] = None,
) -> types.EntityID
```

Adds a polygon and returns its id.

| Parameter      | Type                                                       | Description |
| -------------- | ---------------------------------------------------------- | ----------- |
| `center`       | `types.EntityID`                                           |             |
| `handle`       | `types.EntityID`                                           |             |
| `vertices`     | `Sequence[tuple[float, float]]`                            |             |
| `closed`       | `bool = False`                                             |             |
| `construction` | `bool = False`                                             |             |
| `rings`        | `Optional[Sequence[Sequence[tuple[float, float]]]] = None` |             |
| _Returns_      | `types.EntityID`                                           |             |

### `add_text_box()`

```python
add_text_box(
    origin: types.EntityID,
    width: types.EntityID,
    height: types.EntityID,
    content: str = '',
    font_family: str = 'sans-serif',
    font_size: float = 10.0,
    bold: bool = False,
    italic: bool = False,
) -> types.EntityID
```

Adds a text box and returns its id.

| Parameter     | Type                 | Description |
| ------------- | -------------------- | ----------- |
| `origin`      | `types.EntityID`     |             |
| `width`       | `types.EntityID`     |             |
| `height`      | `types.EntityID`     |             |
| `content`     | `str = ''`           |             |
| `font_family` | `str = 'sans-serif'` |             |
| `font_size`   | `float = 10.0`       |             |
| `bold`        | `bool = False`       |             |
| `italic`      | `bool = False`       |             |
| _Returns_     | `types.EntityID`     |             |

### `append_to_geometry()`

```python
append_to_geometry(
    geo: geo.Geometry,
    id: types.EntityID,
    forward: bool,
) -> None
```

Appends the entity's geometry (traversal direction aware) to an existing Geometry object.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `geo`     | `geo.Geometry`   |             |
| `id`      | `types.EntityID` |             |
| `forward` | `bool`           |             |
| _Returns_ | `None`           |             |

### `characteristic_length()`

```python
characteristic_length(id: types.EntityID) -> float
```

Characteristic length used by equal-length constraints.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `float`          |             |

### `clear_helper_references()`

```python
clear_helper_references(id: types.EntityID) -> None
```

Drops the entity's references to its helper entities.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `None`           |             |

### `contains_point()`

```python
contains_point(id: types.EntityID, x: float, y: float) -> bool
```

Whether a point is inside the entity (closed shapes).

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| `x`       | `float`          |             |
| `y`       | `float`          |             |
| _Returns_ | `bool`           |             |

### `drag_anchor_points()`

```python
drag_anchor_points(point_id: types.EntityID) -> list[types.EntityID]
```

| Parameter  | Type                   | Description |
| ---------- | ---------------------- | ----------- |
| `point_id` | `types.EntityID`       |             |
| _Returns_  | `list[types.EntityID]` |             |

### `enclosed_signed_area()`

```python
enclosed_signed_area(id: types.EntityID) -> Optional[float]
```

Signed area enclosed by a closed single-entity loop.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `id`      | `types.EntityID`  |             |
| _Returns_ | `Optional[float]` |             |

### `entities()`

```python
entities() -> list[entities.Entity]
```

All entities as their concrete classes.

| Parameter | Type                    | Description |
| --------- | ----------------------- | ----------- |
| _Returns_ | `list[entities.Entity]` |             |

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
entity_endpoint_ids(id: types.EntityID) -> Optional[list[types.EntityID]]
```

Endpoint ids of an edge entity, or `None`.

| Parameter | Type                             | Description |
| --------- | -------------------------------- | ----------- |
| `id`      | `types.EntityID`                 |             |
| _Returns_ | `Optional[list[types.EntityID]]` |             |

### `entity_ids()`

```python
entity_ids() -> list[types.EntityID]
```

All entity ids in insertion order.

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| _Returns_ | `list[types.EntityID]` |             |

### `entity_point_ids()`

```python
entity_point_ids(id: types.EntityID) -> Optional[list[types.EntityID]]
```

Control point ids of an entity, or `None`.

| Parameter | Type                             | Description |
| --------- | -------------------------------- | ----------- |
| `id`      | `types.EntityID`                 |             |
| _Returns_ | `Optional[list[types.EntityID]]` |             |

### `entity_to_dict()`

```python
entity_to_dict(id: types.EntityID) -> dict
```

Serializes the entity to a reference entity dictionary.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `dict`           |             |

### `entity_type()`

```python
entity_type(id: types.EntityID) -> Optional[str]
```

The serialization type string of an entity, or `None`.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `Optional[str]`  |             |

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
geometry_signature(id: types.EntityID) -> Any
```

The entity's geometry signature (quantized defining points plus kind-specific state), or `None` for
unknown ids.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `Any`            |             |

### `get_drag_anchor_points()`

```python
get_drag_anchor_points(point_id: types.EntityID) -> list[types.EntityID]
```

Points pinned while `point_id` is dragged. Alias for drag_anchor_points (reference naming).

| Parameter  | Type                   | Description |
| ---------- | ---------------------- | ----------- |
| `point_id` | `types.EntityID`       |             |
| _Returns_  | `list[types.EntityID]` |             |

### `get_entity()`

```python
get_entity(id: types.EntityID) -> Optional[entities.Entity]
```

The entity object for an id, or None. Alias for get_entity_object (reference naming).

| Parameter | Type                        | Description |
| --------- | --------------------------- | ----------- |
| `id`      | `types.EntityID`            |             |
| _Returns_ | `Optional[entities.Entity]` |             |

### `get_entity_object()`

```python
get_entity_object(id: types.EntityID) -> Optional[entities.Entity]
```

| Parameter | Type                        | Description |
| --------- | --------------------------- | ----------- |
| `id`      | `types.EntityID`            |             |
| _Returns_ | `Optional[entities.Entity]` |             |

### `get_point()`

```python
get_point(id: types.EntityID) -> Optional[point.SketchPoint]
```

Coordinates of a point as `(x, y)`, or `None`.

| Parameter | Type                          | Description |
| --------- | ----------------------------- | ----------- |
| `id`      | `types.EntityID`              |             |
| _Returns_ | `Optional[point.SketchPoint]` |             |

### `get_point_object()`

```python
get_point_object(id: types.EntityID) -> Optional[point.SketchPoint]
```

The point object for an id (with id/coords/flags), or None.

| Parameter | Type                          | Description |
| --------- | ----------------------------- | ----------- |
| `id`      | `types.EntityID`              |             |
| _Returns_ | `Optional[point.SketchPoint]` |             |

### `get_state()`

```python
get_state(id: types.EntityID) -> Any
```

Solver-relevant state snapshot of an entity, or `None`.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `Any`            |             |

### `helper_ids()`

```python
helper_ids(id: types.EntityID) -> list[types.EntityID]
```

Helper/child ids of a compound entity.

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| `id`      | `types.EntityID`       |             |
| _Returns_ | `list[types.EntityID]` |             |

### `hit_test()`

```python
hit_test(id: types.EntityID, mx: float, my: float, threshold: float) -> bool
```

Whether the point lies within threshold distance of the entity.

| Parameter   | Type             | Description |
| ----------- | ---------------- | ----------- |
| `id`        | `types.EntityID` |             |
| `mx`        | `float`          |             |
| `my`        | `float`          |             |
| `threshold` | `float`          |             |
| _Returns_   | `bool`           |             |

### `ignorable_unconstrained_points()`

```python
ignorable_unconstrained_points(id: types.EntityID) -> list[types.EntityID]
```

Points that can stay unconstrained when the entity is constrained.

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| `id`      | `types.EntityID`       |             |
| _Returns_ | `list[types.EntityID]` |             |

### `intersects_rect()`

```python
intersects_rect(id: types.EntityID, rect: Sequence[float]) -> bool
```

Whether the entity intersects the rect or is contained by it.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `id`      | `types.EntityID`  |             |
| `rect`    | `Sequence[float]` |             |
| _Returns_ | `bool`            |             |

### `is_closed_loop()`

```python
is_closed_loop(id: types.EntityID) -> bool
```

Whether the entity alone forms a closed loop.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `bool`           |             |

### `is_constrained()`

```python
is_constrained(id: types.EntityID) -> bool
```

Whether the entity is fully constrained.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `bool`           |             |

### `is_construction()`

```python
is_construction(id: types.EntityID) -> bool
```

Whether the entity is a construction entity.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `bool`           |             |

### `is_contained_by()`

```python
is_contained_by(id: types.EntityID, rect: Sequence[float]) -> bool
```

Whether the entity is fully strictly contained in the rect.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `id`      | `types.EntityID`  |             |
| `rect`    | `Sequence[float]` |             |
| _Returns_ | `bool`            |             |

### `is_edge_entity()`

```python
is_edge_entity(id: types.EntityID) -> bool
```

Whether the entity participates in edge/chaining adjacency.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `bool`           |             |

### `is_point_used()`

```python
is_point_used(id: types.EntityID) -> bool
```

Whether the point is referenced by any entity.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `bool`           |             |

### `is_radius_entity()`

```python
is_radius_entity(id: types.EntityID) -> bool
```

Whether the entity has a radius.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `bool`           |             |

### `junction_point_ids()`

```python
junction_point_ids(id: types.EntityID) -> list[types.EntityID]
```

Junction point ids of an entity.

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| `id`      | `types.EntityID`       |             |
| _Returns_ | `list[types.EntityID]` |             |

### `load_from_dict()`

```python
load_from_dict(dict: dict) -> None
```

Replaces the model in place from a reference registry dictionary, keeping this registry object (and
any views bound to it) valid.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `dict`    | `dict` |             |
| _Returns_ | `None` |             |

### `mirror()`

```python
mirror(id: types.EntityID, direction: str, position: float) -> None
```

Mirrors entity-internal state across the axis (arc chirality, bezier offsets, polygon winding).

| Parameter   | Type             | Description |
| ----------- | ---------------- | ----------- |
| `id`        | `types.EntityID` |             |
| `direction` | `str`            |             |
| `position`  | `float`          |             |
| _Returns_   | `None`           |             |

### `point_constrained()`

```python
point_constrained(id: types.EntityID) -> bool
```

Whether the point is solver-constrained.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `bool`           |             |

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
point_ids() -> list[types.EntityID]
```

All point ids in insertion order.

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| _Returns_ | `list[types.EntityID]` |             |

### `point_is_fixed()`

```python
point_is_fixed(id: types.EntityID) -> bool
```

Whether a point carries the fixed flag.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `bool`           |             |

### `point_is_in_text_box_frame()`

```python
point_is_in_text_box_frame(id: types.EntityID) -> bool
```

Whether the point is one of a text box frame's corner points.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `bool`           |             |

### `points()`

```python
points() -> list[point.SketchPoint]
```

All sketch points, in id order.

| Parameter | Type                      | Description |
| --------- | ------------------------- | ----------- |
| _Returns_ | `list[point.SketchPoint]` |             |

### `remove_entities()`

```python
remove_entities(entity_ids: Sequence[types.EntityID]) -> None
```

Removes entities by id.

| Parameter    | Type                       | Description |
| ------------ | -------------------------- | ----------- |
| `entity_ids` | `Sequence[types.EntityID]` |             |
| _Returns_    | `None`                     |             |

### `remove_points()`

```python
remove_points(point_ids: Sequence[types.EntityID]) -> None
```

Removes the given points.

| Parameter   | Type                       | Description |
| ----------- | -------------------------- | ----------- |
| `point_ids` | `Sequence[types.EntityID]` |             |
| _Returns_   | `None`                     |             |

### `replace_entity_from_dict()`

```python
replace_entity_from_dict(id: types.EntityID, data: dict) -> bool
```

Restores an entity's state snapshot. Replaces an entity in place from its dictionary form.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| `data`    | `dict`           |             |
| _Returns_ | `bool`           |             |

### `rigidly_connected_points()`

```python
rigidly_connected_points(point_id: types.EntityID) -> list[types.EntityID]
```

Points that move rigidly with `point_id`.

| Parameter  | Type                   | Description |
| ---------- | ---------------------- | ----------- |
| `point_id` | `types.EntityID`       |             |
| _Returns_  | `list[types.EntityID]` |             |

### `set_construction()`

```python
set_construction(id: types.EntityID, construction: bool) -> None
```

Sets the construction flag.

| Parameter      | Type             | Description |
| -------------- | ---------------- | ----------- |
| `id`           | `types.EntityID` |             |
| `construction` | `bool`           |             |
| _Returns_      | `None`           |             |

### `set_entity_constrained()`

```python
set_entity_constrained(id: types.EntityID, constrained: bool) -> bool
```

Sets an entity's solver-constrained flag.

| Parameter     | Type             | Description |
| ------------- | ---------------- | ----------- |
| `id`          | `types.EntityID` |             |
| `constrained` | `bool`           |             |
| _Returns_     | `bool`           |             |

### `set_point_constrained()`

```python
set_point_constrained(id: types.EntityID, constrained: bool) -> bool
```

Sets a point's solver-constrained flag.

| Parameter     | Type             | Description |
| ------------- | ---------------- | ----------- |
| `id`          | `types.EntityID` |             |
| `constrained` | `bool`           |             |
| _Returns_     | `bool`           |             |

### `set_point_position()`

```python
set_point_position(id: types.EntityID, x: float, y: float) -> bool
```

Sets a point's coordinates.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| `x`       | `float`          |             |
| `y`       | `float`          |             |
| _Returns_ | `bool`           |             |

### `set_point_waypoint_type()`

```python
set_point_waypoint_type(id: types.EntityID, value: str) -> bool
```

Sets a point's waypoint role.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| `value`   | `str`            |             |
| _Returns_ | `bool`           |             |

### `set_points()`

```python
set_points(points: Any) -> None
```

Replaces the point list with `(id, x, y, fixed)` tuples.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `points`  | `Any`  |             |
| _Returns_ | `None` |             |

### `set_state()`

```python
set_state(id: types.EntityID, state: dict) -> None
```

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| `state`   | `dict`           |             |
| _Returns_ | `None`           |             |

### `signed_distance_to()`

```python
signed_distance_to(id: types.EntityID, px: float, py: float) -> Optional[float]
```

Signed distance from a point to the entity's locus, or `None` when unsupported.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `id`      | `types.EntityID`  |             |
| `px`      | `float`           |             |
| `py`      | `float`           |             |
| _Returns_ | `Optional[float]` |             |

### `supports_point_on_curve()`

```python
supports_point_on_curve(id: types.EntityID) -> bool
```

Whether a point can be constrained onto this entity's curve.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `bool`           |             |

### `tangent_at()`

```python
tangent_at(
    id: types.EntityID,
    point_id: types.EntityID,
) -> Optional[tuple[float, float]]
```

Tangent vector at a defining point, or `None` when the entity has no directional tangent.

| Parameter  | Type                            | Description |
| ---------- | ------------------------------- | ----------- |
| `id`       | `types.EntityID`                |             |
| `point_id` | `types.EntityID`                |             |
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
to_geometry(id: types.EntityID) -> geo.Geometry
```

The entity as a raygeo Geometry object.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `geo.Geometry`   |             |

### `to_polygon_vertices()`

```python
to_polygon_vertices(
    id: types.EntityID,
    forward: bool,
) -> list[tuple[float, float]]
```

Polygon vertices of the entity (sampled for curves).

| Parameter | Type                        | Description |
| --------- | --------------------------- | ----------- |
| `id`      | `types.EntityID`            |             |
| `forward` | `bool`                      |             |
| _Returns_ | `list[tuple[float, float]]` |             |

### `to_polyline()`

```python
to_polyline(
    id: types.EntityID,
    tolerance: float = 0.1,
) -> list[tuple[float, float]]
```

Sampled polyline of the entity.

| Parameter   | Type                        | Description |
| ----------- | --------------------------- | ----------- |
| `id`        | `types.EntityID`            |             |
| `tolerance` | `float = 0.1`               |             |
| _Returns_   | `list[tuple[float, float]]` |             |

### `to_polylines()`

```python
to_polylines(
    id: types.EntityID,
    tolerance: float = 0.1,
) -> list[list[tuple[float, float]]]
```

Sampled polylines of the entity (multi-contour for text).

| Parameter   | Type                              | Description |
| ----------- | --------------------------------- | ----------- |
| `id`        | `types.EntityID`                  |             |
| `tolerance` | `float = 0.1`                     |             |
| _Returns_   | `list[list[tuple[float, float]]]` |             |

### `update_constrained_status()`

```python
update_constrained_status(
    radius_constrained_entity_ids: Sequence[types.EntityID] = [],
) -> None
```

Re-computes constrained flags. Entities whose radius is explicitly dimensioned are listed in
`radius_constrained_entity_ids`.

| Parameter                       | Type                            | Description |
| ------------------------------- | ------------------------------- | ----------- |
| `radius_constrained_entity_ids` | `Sequence[types.EntityID] = []` |             |
| _Returns_                       | `None`                          |             |

### `waypoint_type()`

```python
waypoint_type(id: types.EntityID) -> Optional[str]
```

Waypoint type string of a point ("sharp"/"smooth"/"symmetric").

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `id`      | `types.EntityID` |             |
| _Returns_ | `Optional[str]`  |             |
