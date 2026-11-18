---
title: raygeo.sketch.entities
sidebar_label: raygeo.sketch.entities
---

## Entity

The common base class of every entity binding: a nominal `Entity` type for `isinstance` checks and
annotations. The per-class members live on the subclasses; polymorphism is carried by the Rust-side
`Entity` trait.

### `array_copy`

```python
array_copy: bool
```

Whether the entity is part of an array.

### `constrained`

```python
constrained: bool
```

Whether the entity's geometry is fully determined by the solver.

### `construction`

```python
construction: bool
```

Whether the entity is construction geometry.

### `fill_color`

```python
fill_color: Optional[tuple[float, float, float, float]]
```

The fill color, when one is set.

### `id`

```python
id: int
```

The unique entity id.

### `invisible`

```python
invisible: bool
```

Whether the entity is hidden from output generation.

### `type`

```python
type: str
```

The entity kind label (`"line"`, `"arc"`, ...).

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

### `as_offset_item()`

```python
as_offset_item(sketch: Any) -> Optional[Any]
```

The offsettable item this entity forms on its own; the reference default is None.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| `sketch`  | `Any`           |             |
| _Returns_ | `Optional[Any]` |             |

### `characteristic_length()`

```python
characteristic_length(registry: protocols.Registry) -> float
```

The entity's characteristic length.

| Parameter  | Type                 | Description |
| ---------- | -------------------- | ----------- |
| `registry` | `protocols.Registry` |             |
| _Returns_  | `float`              |             |

### `characteristic_length_pairs()`

```python
characteristic_length_pairs() -> list[list[int]]
```

The entity's characteristic-length point pairs.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `list[list[int]]` |             |

### `clear_helper_references()`

```python
clear_helper_references() -> None
```

Clears the entity's helper point references.

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
enclosed_signed_area(registry: protocols.Registry) -> float
```

The signed area the entity encloses on its own; raises `NotImplementedError` when unsupported.

| Parameter  | Type                 | Description |
| ---------- | -------------------- | ----------- |
| `registry` | `protocols.Registry` |             |
| _Returns_  | `float`              |             |

### `geometry_signature()`

```python
geometry_signature(registry: protocols.Registry) -> tuple
```

The geometry signature used for change detection.

| Parameter  | Type                 | Description |
| ---------- | -------------------- | ----------- |
| `registry` | `protocols.Registry` |             |
| _Returns_  | `tuple`              |             |

### `get_all_frame_point_ids()`

```python
get_all_frame_point_ids(registry: protocols.Registry) -> list[int]
```

The ids of all frame points, including text box frames.

| Parameter  | Type                 | Description |
| ---------- | -------------------- | ----------- |
| `registry` | `protocols.Registry` |             |
| _Returns_  | `list[int]`          |             |

### `get_drag_anchor_points()`

```python
get_drag_anchor_points(point_id: int) -> list[int]
```

The ids of the entity's drag anchor points.

| Parameter  | Type        | Description |
| ---------- | ----------- | ----------- |
| `point_id` | `int`       |             |
| _Returns_  | `list[int]` |             |

### `get_endpoint_ids()`

```python
get_endpoint_ids() -> list[int]
```

The ids of the entity's endpoints.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[int]` |             |

### `get_helper_ids()`

```python
get_helper_ids() -> list[int]
```

The ids of the entity's helper points.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[int]` |             |

### `get_ignorable_unconstrained_points()`

```python
get_ignorable_unconstrained_points() -> list[int]
```

The ids of points that may stay unconstrained.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[int]` |             |

### `get_junction_point_ids()`

```python
get_junction_point_ids() -> list[int]
```

The ids of the entity's junction points.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[int]` |             |

### `get_point_ids()`

```python
get_point_ids() -> list[int]
```

The ids of all control points used by this entity.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[int]` |             |

### `get_rigidly_connected_points()`

```python
get_rigidly_connected_points(point_id: int) -> list[int]
```

The ids of points rigidly connected to the given point.

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

Whether the entity alone forms a closed loop.

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

Whether the entity is fully contained by the given rect.

| Parameter  | Type                                | Description |
| ---------- | ----------------------------------- | ----------- |
| `rect`     | `tuple[float, float, float, float]` |             |
| `registry` | `protocols.Registry`                |             |
| _Returns_  | `bool`                              |             |

### `is_edge_entity()`

```python
is_edge_entity() -> bool
```

Whether the entity is an edge entity.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `is_radius_entity()`

```python
is_radius_entity() -> bool
```

Whether the entity has a radius.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `mirror()`

```python
mirror(axis: Any) -> None
```

Mirrors the entity across the reference axis object.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `axis`    | `Any`  |             |
| _Returns_ | `None` |             |

### `plan_offset()`

```python
plan_offset(
    registry: protocols.Registry,
    offset: float,
    _allocate_id: Any,
) -> Optional[OffsetPlan]
```

Returns a pure description of the offset changes; the reference default is None (no analytic
offset).

| Parameter      | Type                   | Description |
| -------------- | ---------------------- | ----------- |
| `registry`     | `protocols.Registry`   |             |
| `offset`       | `float`                |             |
| `_allocate_id` | `Any`                  |             |
| _Returns_      | `Optional[OffsetPlan]` |             |

### `preview_polylines()`

```python
preview_polylines(
    registry: protocols.Registry,
    offset: float,
) -> list[list[tuple[float, float]]]
```

Coarse polylines of the entity's offset for live preview; the reference default is no preview.

| Parameter  | Type                              | Description |
| ---------- | --------------------------------- | ----------- |
| `registry` | `protocols.Registry`              |             |
| `offset`   | `float`                           |             |
| _Returns_  | `list[list[tuple[float, float]]]` |             |

### `remap_references()`

```python
remap_references(id_map: Mapping[int, int]) -> None
```

Rewrites every point/entity id reference through *id_map*.

| Parameter | Type                | Description |
| --------- | ------------------- | ----------- |
| `id_map`  | `Mapping[int, int]` |             |
| _Returns_ | `None`              |             |

### `rewrite_offsets_from()`

```python
rewrite_offsets_from(template: Any, placement: Any) -> None
```

Re-derives copy state from a template.

| Parameter   | Type   | Description |
| ----------- | ------ | ----------- |
| `template`  | `Any`  |             |
| `placement` | `Any`  |             |
| _Returns_   | `None` |             |

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

Whether the entity supports fills.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `supports_point_on_curve()`

```python
supports_point_on_curve() -> bool
```

Whether the entity supports point-on-curve constraints.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `tangent_at()`

```python
tangent_at(registry: protocols.Registry, point_id: int) -> tuple[float, float]
```

Tangent direction at the point nearest the model-space point; raises `NotImplementedError` when the
entity has no tangent.

| Parameter  | Type                  | Description |
| ---------- | --------------------- | ----------- |
| `registry` | `protocols.Registry`  |             |
| `point_id` | `int`                 |             |
| _Returns_  | `tuple[float, float]` |             |

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

Returns the entity's toolpath geometry in model coordinates.

| Parameter  | Type                 | Description |
| ---------- | -------------------- | ----------- |
| `registry` | `protocols.Registry` |             |
| _Returns_  | `geo.Geometry`       |             |

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

### `to_polylines()`

```python
to_polylines(
    registry: protocols.Registry,
    tolerance: float = 0.1,
) -> list[list[tuple[float, float]]]
```

Samples the entity into one or more polylines.

| Parameter   | Type                              | Description |
| ----------- | --------------------------------- | ----------- |
| `registry`  | `protocols.Registry`              |             |
| `tolerance` | `float = 0.1`                     |             |
| _Returns_   | `list[list[tuple[float, float]]]` |             |

### `transform_offsets()`

```python
transform_offsets(placement: Any) -> None
```

Transforms entity-internal offset state for a placement.

| Parameter   | Type   | Description |
| ----------- | ------ | ----------- |
| `placement` | `Any`  |             |
| _Returns_   | `None` |             |

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

## OffsetPlan

Description of the changes one offset operation makes.

### `entities`

```python
entities: list
```

### `point_moves`

```python
point_moves: dict
```

### `points`

```python
points: list
```

### `removed_entity_ids`

```python
removed_entity_ids: list
```

## PolygonOutline

A sampled outline produced from a connected chain of entities.

### `closed`

```python
closed: bool
```

### `source_ids`

```python
source_ids: list[int]
```

### `vertices`

```python
vertices: list[tuple[float, float]]
```

### `offset()`

```python
offset(
    distance: float,
    tolerance: float,
) -> list[tuple[list[tuple[float, float]], bool]]
```

Offsets the outline into `(vertices, closed)` pairs.

| Parameter   | Type                                           | Description |
| ----------- | ---------------------------------------------- | ----------- |
| `distance`  | `float`                                        |             |
| `tolerance` | `float`                                        |             |
| _Returns_   | `list[tuple[list[tuple[float, float]], bool]]` |             |

### `plan_offset()`

```python
plan_offset(
    _registry: protocols.Registry,
    offset: float,
    allocate_id: Any,
) -> Optional[OffsetPlan]
```

Plans the replacement geometry for this outline.

| Parameter     | Type                   | Description |
| ------------- | ---------------------- | ----------- |
| `_registry`   | `protocols.Registry`   |             |
| `offset`      | `float`                |             |
| `allocate_id` | `Any`                  |             |
| _Returns_     | `Optional[OffsetPlan]` |             |

### `preview_polylines()`

```python
preview_polylines(
    _registry: protocols.Registry,
    offset: float,
) -> list[list[tuple[float, float]]]
```

Coarse polylines of the offset result for live preview.

| Parameter   | Type                              | Description |
| ----------- | --------------------------------- | ----------- |
| `_registry` | `protocols.Registry`              |             |
| `offset`    | `float`                           |             |
| _Returns_   | `list[list[tuple[float, float]]]` |             |

## Functions

### `quantize()`

```python
quantize(value: float) -> float
```

Rounds a coordinate to the model's six-decimal precision.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `value`   | `float` |             |
| _Returns_ | `float` |             |
