---
title: raygeo.sketch.entities.polygon
sidebar_label: raygeo.sketch.entities.polygon
---

## Polygon

A polygon defined in the local frame of center and handle points.

### `array_copy`

```python
array_copy: bool
```

Whether the entity was created as part of an array copy.

### `center_idx`

```python
center_idx: int
```

Center point id.

### `closed`

```python
closed: bool
```

Whether the polygon contour is closed.

### `constrained`

```python
constrained: bool
```

Whether the entity is fully constrained, as computed by the solver.

### `construction`

```python
construction: bool
```

Whether the entity is construction geometry.

### `fill_color`

```python
fill_color: Optional[tuple[float, float, float, float]]
```

Fill color as (r, g, b, a), or None.

### `handle_idx`

```python
handle_idx: int
```

Handle point id.

### `id`

```python
id: types.EntityID
```

Unique entity id.

### `invisible`

```python
invisible: bool
```

Whether the entity is excluded from toolpath output.

### `rings`

```python
rings: list[list[tuple[float, float]]]
```

All rings in the local frame (outer contour first, then holes).

### `type`

```python
type: str
```

The entity type name (attribute alias).

### `vertices`

```python
vertices: list[tuple[float, float]]
```

The outer contour vertices in the local frame.

### `append_to_geometry()`

```python
append_to_geometry(
    geo: geo.Geometry,
    registry: protocols.Registry,
    forward: bool = True,
) -> None
```

Appends the entity's outline to the geometry.

| Parameter  | Type                 | Description |
| ---------- | -------------------- | ----------- |
| `geo`      | `geo.Geometry`       |             |
| `registry` | `protocols.Registry` |             |
| `forward`  | `bool = True`        |             |
| _Returns_  | `None`               |             |

### `build()`

```python
build(
    id: int,
    center: int,
    handle: int,
    center_pos: tuple[float, float],
    handle_pos: tuple[float, float],
    vertices: Sequence[tuple[float, float]],
    closed: bool = False,
    world_holes: Optional[Sequence[Sequence[tuple[float, float]]]] = None,
) -> tuple[Any, Any, Polygon]
```

Reference factory: builds a polygon from center/handle point ids, outer vertices and optional
world-space hole rings.

| Parameter     | Type                                                       | Description |
| ------------- | ---------------------------------------------------------- | ----------- |
| `id`          | `int`                                                      |             |
| `center`      | `int`                                                      |             |
| `handle`      | `int`                                                      |             |
| `center_pos`  | `tuple[float, float]`                                      |             |
| `handle_pos`  | `tuple[float, float]`                                      |             |
| `vertices`    | `Sequence[tuple[float, float]]`                            |             |
| `closed`      | `bool = False`                                             |             |
| `world_holes` | `Optional[Sequence[Sequence[tuple[float, float]]]] = None` |             |
| _Returns_     | `tuple[Any, Any, Polygon]`                                 |             |

### `characteristic_length()`

```python
characteristic_length(registry: protocols.Registry) -> float
```

The characteristic length used by equal-length constraints.

| Parameter  | Type                 | Description |
| ---------- | -------------------- | ----------- |
| `registry` | `protocols.Registry` |             |
| _Returns_  | `float`              |             |

### `characteristic_length_pairs()`

```python
characteristic_length_pairs() -> list[list[int]]
```

The defining-point pairs of the characteristic length.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `list[list[int]]` |             |

### `clear_helper_references()`

```python
clear_helper_references() -> None
```

Drops this entity's references to its helper entities.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `contains_point()`

```python
contains_point(registry: protocols.Registry, x: float, y: float) -> bool
```

Whether the model-space point lies inside the entity.

| Parameter  | Type                 | Description |
| ---------- | -------------------- | ----------- |
| `registry` | `protocols.Registry` |             |
| `x`        | `float`              |             |
| `y`        | `float`              |             |
| _Returns_  | `bool`               |             |

### `create_fill_geometry()`

```python
create_fill_geometry(registry: protocols.Registry) -> Optional[geo.Geometry]
```

The fill geometry of the entity, if it supports fills.

| Parameter  | Type                     | Description |
| ---------- | ------------------------ | ----------- |
| `registry` | `protocols.Registry`     |             |
| _Returns_  | `Optional[geo.Geometry]` |             |

### `enclosed_signed_area()`

```python
enclosed_signed_area(registry: protocols.Registry) -> Optional[float]
```

The signed area enclosed by the polygon (outer ring minus holes), or None when the frame points are
missing.

| Parameter  | Type                 | Description |
| ---------- | -------------------- | ----------- |
| `registry` | `protocols.Registry` |             |
| _Returns_  | `Optional[float]`    |             |

### `from_dict()`

```python
from_dict(data: dict) -> Polygon
```

Builds a polygon from its reference dictionary form.

| Parameter | Type      | Description |
| --------- | --------- | ----------- |
| `data`    | `dict`    |             |
| _Returns_ | `Polygon` |             |

### `geometry_signature()`

```python
geometry_signature(registry: protocols.Registry) -> tuple
```

Returns the geometry signature used for change detection.

| Parameter  | Type                 | Description |
| ---------- | -------------------- | ----------- |
| `registry` | `protocols.Registry` |             |
| _Returns_  | `tuple`              |             |

### `get_all_frame_point_ids()`

```python
get_all_frame_point_ids(registry: protocols.Registry) -> list[int]
```

Frame point ids of a text box, for containment checks.

| Parameter  | Type                 | Description |
| ---------- | -------------------- | ----------- |
| `registry` | `protocols.Registry` |             |
| _Returns_  | `list[int]`          |             |

### `get_drag_anchor_points()`

```python
get_drag_anchor_points(point_id: int) -> list[int]
```

Point ids that act as drag anchors for the given point.

| Parameter  | Type        | Description |
| ---------- | ----------- | ----------- |
| `point_id` | `int`       |             |
| _Returns_  | `list[int]` |             |

### `get_endpoint_ids()`

```python
get_endpoint_ids() -> list[int]
```

The entity ids of the curve endpoints.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[int]` |             |

### `get_helper_ids()`

```python
get_helper_ids() -> list[int]
```

Auxiliary/child entity ids belonging to this compound entity.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[int]` |             |

### `get_ignorable_unconstrained_points()`

```python
get_ignorable_unconstrained_points() -> list[int]
```

Point ids that can remain unconstrained when this entity is constrained.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[int]` |             |

### `get_junction_point_ids()`

```python
get_junction_point_ids() -> list[int]
```

Point ids acting as junction candidates for chaining.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[int]` |             |

### `get_point_ids()`

```python
get_point_ids() -> list[int]
```

The point ids the entity is defined by.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[int]` |             |

### `get_rigidly_connected_points()`

```python
get_rigidly_connected_points(point_id: int) -> list[int]
```

Point ids that move rigidly with the given point.

| Parameter  | Type        | Description |
| ---------- | ----------- | ----------- |
| `point_id` | `int`       |             |
| _Returns_  | `list[int]` |             |

### `get_state()`

```python
get_state() -> Optional[dict]
```

Solver-relevant discrete state for undo/redo snapshots.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| _Returns_ | `Optional[dict]` |             |

### `get_world_rings()`

```python
get_world_rings(
    registry: protocols.Registry,
) -> list[list[tuple[float, float]]]
```

The polygon's rings in world coordinates.

| Parameter  | Type                              | Description |
| ---------- | --------------------------------- | ----------- |
| `registry` | `protocols.Registry`              |             |
| _Returns_  | `list[list[tuple[float, float]]]` |             |

### `get_world_vertices()`

```python
get_world_vertices(registry: protocols.Registry) -> list[tuple[float, float]]
```

Returns the entity's toolpath geometry in model coordinates. Returns the polygon vertices in world
coordinates.

| Parameter  | Type                        | Description |
| ---------- | --------------------------- | ----------- |
| `registry` | `protocols.Registry`        |             |
| _Returns_  | `list[tuple[float, float]]` |             |

### `hit_test()`

```python
hit_test(
    mx: float,
    my: float,
    threshold: float,
    registry: protocols.Registry,
) -> bool
```

Whether the entity is hit at model coordinates (mx, my).

| Parameter   | Type                 | Description |
| ----------- | -------------------- | ----------- |
| `mx`        | `float`              |             |
| `my`        | `float`              |             |
| `threshold` | `float`              |             |
| `registry`  | `protocols.Registry` |             |
| _Returns_   | `bool`               |             |

### `intersects_rect()`

```python
intersects_rect(
    rect: tuple[float, float, float, float],
    registry: protocols.Registry,
) -> bool
```

Whether the entity intersects the given rect.

| Parameter  | Type                                | Description |
| ---------- | ----------------------------------- | ----------- |
| `rect`     | `tuple[float, float, float, float]` |             |
| `registry` | `protocols.Registry`                |             |
| _Returns_  | `bool`                              |             |

### `is_closed_loop()`

```python
is_closed_loop() -> bool
```

Whether the polygon is a closed loop.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `is_contained_by()`

```python
is_contained_by(
    rect: tuple[float, float, float, float],
    registry: protocols.Registry,
) -> bool
```

Whether the polygon is contained by the given (x1, y1, x2, y2) rect.

| Parameter  | Type                                | Description |
| ---------- | ----------------------------------- | ----------- |
| `rect`     | `tuple[float, float, float, float]` |             |
| `registry` | `protocols.Registry`                |             |
| _Returns_  | `bool`                              |             |

### `is_edge_entity()`

```python
is_edge_entity() -> bool
```

Whether the entity participates in edge/chaining adjacency for loop detection (line, arc, bezier).

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `is_radius_entity()`

```python
is_radius_entity() -> bool
```

Whether the entity has a radius (arc, circle).

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `normalize_vertices()`

```python
normalize_vertices(
    center: tuple[float, float],
    handle: tuple[float, float],
    world_vertices: Sequence[tuple[float, float]],
) -> list[tuple[float, float]]
```

Converts world coordinates to frame-local (u, v) pairs.

| Parameter        | Type                            | Description |
| ---------------- | ------------------------------- | ----------- |
| `center`         | `tuple[float, float]`           |             |
| `handle`         | `tuple[float, float]`           |             |
| `world_vertices` | `Sequence[tuple[float, float]]` |             |
| _Returns_        | `list[tuple[float, float]]`     |             |

### `restore_base_flags()`

```python
restore_base_flags(data: dict) -> None
```

Restores base flags serialized by to_dict() but not accepted as constructor arguments.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `data`    | `dict` |             |
| _Returns_ | `None` |             |

### `set_fill_color()`

```python
set_fill_color(color: Optional[Sequence[float]]) -> bool
```

Sets the fill color; returns whether the entity supports fills.

| Parameter | Type                        | Description |
| --------- | --------------------------- | ----------- |
| `color`   | `Optional[Sequence[float]]` |             |
| _Returns_ | `bool`                      |             |

### `set_state()`

```python
set_state(state: dict) -> None
```

Restores solver-relevant discrete state from a snapshot.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `state`   | `dict` |             |
| _Returns_ | `None` |             |

### `signed_distance_to()`

```python
signed_distance_to(
    px: float,
    py: float,
    registry: protocols.Registry,
) -> Optional[float]
```

Signed distance from the model-space point, if defined.

| Parameter  | Type                 | Description |
| ---------- | -------------------- | ----------- |
| `px`       | `float`              |             |
| `py`       | `float`              |             |
| `registry` | `protocols.Registry` |             |
| _Returns_  | `Optional[float]`    |             |

### `supports_fill()`

```python
supports_fill() -> bool
```

Whether this entity can receive a fill color.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `supports_point_on_curve()`

```python
supports_point_on_curve() -> bool
```

Whether a point can be constrained onto this entity's curve.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `tangent_at()`

```python
tangent_at(
    registry: protocols.Registry,
    point_id: int,
) -> Optional[tuple[float, float]]
```

Tangent direction at the point nearest the model-space point.

| Parameter  | Type                            | Description |
| ---------- | ------------------------------- | ----------- |
| `registry` | `protocols.Registry`            |             |
| `point_id` | `int`                           |             |
| _Returns_  | `Optional[tuple[float, float]]` |             |

### `to_dict()`

```python
to_dict() -> dict
```

Serializes the entity to the reference dictionary shape.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

### `to_geometry()`

```python
to_geometry(registry: protocols.Registry) -> geo.Geometry
```

| Parameter  | Type                 | Description |
| ---------- | -------------------- | ----------- |
| `registry` | `protocols.Registry` |             |
| _Returns_  | `geo.Geometry`       |             |

### `to_outline()`

```python
to_outline(
    registry: protocols.Registry,
) -> tuple[list[tuple[float, float]], bool]
```

Returns (world vertices, closed) for offset processing.

| Parameter  | Type                                     | Description |
| ---------- | ---------------------------------------- | ----------- |
| `registry` | `protocols.Registry`                     |             |
| _Returns_  | `tuple[list[tuple[float, float]], bool]` |             |

### `to_polygon_vertices()`

```python
to_polygon_vertices(
    registry: protocols.Registry,
    forward: bool = True,
) -> list[tuple[float, float]]
```

Returns the entity's outline as polygon vertices.

| Parameter  | Type                        | Description |
| ---------- | --------------------------- | ----------- |
| `registry` | `protocols.Registry`        |             |
| `forward`  | `bool = True`               |             |
| _Returns_  | `list[tuple[float, float]]` |             |

### `to_polyline()`

```python
to_polyline(
    registry: protocols.Registry,
    tolerance: float = 0.1,
) -> list[tuple[float, float]]
```

Returns the entity's centerline as polyline vertices.

| Parameter   | Type                        | Description |
| ----------- | --------------------------- | ----------- |
| `registry`  | `protocols.Registry`        |             |
| `tolerance` | `float = 0.1`               |             |
| _Returns_   | `list[tuple[float, float]]` |             |

### `type_name()`

```python
type_name() -> str
```

The entity type name.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `str` |             |

### `update_constrained_status()`

```python
update_constrained_status(
    registry: protocols.Registry,
    constraints: Any,
) -> None
```

Updates the constrained flag from defining-point status and constraints that dimension this entity's
radius or length.

| Parameter     | Type                 | Description |
| ------------- | -------------------- | ----------- |
| `registry`    | `protocols.Registry` |             |
| `constraints` | `Any`                |             |
| _Returns_     | `None`               |             |

### `write_back()`

```python
write_back() -> None
```

Persists in-place edits back to the owning registry, if any.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |
