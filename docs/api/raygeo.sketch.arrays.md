---
title: raygeo.sketch.arrays
sidebar_label: raygeo.sketch.arrays
---

## Array

Persistent definition of a sketch array.

### `align_to_tangent`

```python
align_to_tangent: bool
```

Whether curve-along copies follow the tangent.

### `count`

```python
count: int
```

Number of members.

### `guide_circle_id`

```python
guide_circle_id: types.EntityID
```

Guide circle or guide path entity id.

### `guide_entity_id`

```python
guide_entity_id: types.EntityID
```

Guide entity id (circle or path).

### `members`

```python
members: list[tuple[int, list[types.EntityID]]]
```

Array members as `(slot, [entity_id, ...])` pairs.

### `mode`

```python
mode: str
```

Array mode: `circular` or `curve_along`.

### `offset_to_start`

```python
offset_to_start: float
```

Curve-along start offset.

### `path_entity_id`

```python
path_entity_id: int
```

Curve-along guide path entity id.

### `rotate_copies`

```python
rotate_copies: bool
```

Whether copies are rotated with the guide.

### `spacing`

```python
spacing: float
```

Curve-along spacing.

### `standalone_pids`

```python
standalone_pids: dict
```

Standalone member points as `{slot: [point_id, ...]}`.

### `template_anchor`

```python
template_anchor: Optional[tuple[tuple[float, float], float]]
```

Curve-along template anchor.

### `total_angle_deg`

```python
total_angle_deg: float
```

Circular-array sweep angle.

### `uid`

```python
uid: str
```

Unique array id.

### `circular()`

```python
circular(
    uid: str,
    guide_circle_id: types.EntityID,
    members: Optional[Sequence[tuple[int, Sequence[types.EntityID]]]] = None,
    count: int = 6,
    total_angle_deg: float = 360.0,
    rotate_copies: bool = True,
) -> Array
```

| Parameter         | Type                                                              | Description |
| ----------------- | ----------------------------------------------------------------- | ----------- |
| `uid`             | `str`                                                             |             |
| `guide_circle_id` | `types.EntityID`                                                  |             |
| `members`         | `Optional[Sequence[tuple[int, Sequence[types.EntityID]]]] = None` |             |
| `count`           | `int = 6`                                                         |             |
| `total_angle_deg` | `float = 360.0`                                                   |             |
| `rotate_copies`   | `bool = True`                                                     |             |
| _Returns_         | `Array`                                                           |             |

### `commit()`

```python
commit(strategy: Any) -> None
```

Writes a strategy's parameters onto the definition.

| Parameter  | Type   | Description |
| ---------- | ------ | ----------- |
| `strategy` | `Any`  |             |
| _Returns_  | `None` |             |

### `copy()`

```python
copy() -> Array
```

Creates a copy of the definition.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| _Returns_ | `Array` |             |

### `curve_along()`

```python
curve_along(
    uid: str,
    guide_circle_id: types.EntityID,
    members: Optional[Sequence[tuple[int, Sequence[types.EntityID]]]] = None,
    count: int = 6,
    rotate_copies: bool = True,
    path_entity_id: int = -1,
    align_to_tangent: bool = True,
    offset_to_start: float = 0.0,
    spacing: float = 0.0,
    template_anchor: Optional[tuple[tuple[float, float], float]] = None,
) -> Array
```

| Parameter          | Type                                                              | Description |
| ------------------ | ----------------------------------------------------------------- | ----------- |
| `uid`              | `str`                                                             |             |
| `guide_circle_id`  | `types.EntityID`                                                  |             |
| `members`          | `Optional[Sequence[tuple[int, Sequence[types.EntityID]]]] = None` |             |
| `count`            | `int = 6`                                                         |             |
| `rotate_copies`    | `bool = True`                                                     |             |
| `path_entity_id`   | `int = -1`                                                        |             |
| `align_to_tangent` | `bool = True`                                                     |             |
| `offset_to_start`  | `float = 0.0`                                                     |             |
| `spacing`          | `float = 0.0`                                                     |             |
| `template_anchor`  | `Optional[tuple[tuple[float, float], float]] = None`              |             |
| _Returns_          | `Array`                                                           |             |

### `from_dict()`

```python
from_dict(dict: dict) -> Any
```

Deserializes a definition.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `dict`    | `dict` |             |
| _Returns_ | `Any`  |             |

### `from_strategy()`

```python
from_strategy(
    strategy: Any,
    uid: str,
    guide_circle_id: types.EntityID,
    members: Sequence[tuple[int, Sequence[types.EntityID]]],
    count: int,
    template_anchor: Optional[tuple[tuple[float, float], float]],
) -> Any
```

Creates an array from a strategy.

| Parameter         | Type                                             | Description |
| ----------------- | ------------------------------------------------ | ----------- |
| `strategy`        | `Any`                                            |             |
| `uid`             | `str`                                            |             |
| `guide_circle_id` | `types.EntityID`                                 |             |
| `members`         | `Sequence[tuple[int, Sequence[types.EntityID]]]` |             |
| `count`           | `int`                                            |             |
| `template_anchor` | `Optional[tuple[tuple[float, float], float]]`    |             |
| _Returns_         | `Any`                                            |             |

### `guide_signature()`

```python
guide_signature(registry: registry.EntityRegistry) -> Any
```

Quantized guide geometry signature.

| Parameter  | Type                      | Description |
| ---------- | ------------------------- | ----------- |
| `registry` | `registry.EntityRegistry` |             |
| _Returns_  | `Any`                     |             |

### `is_guide_radius_point()`

```python
is_guide_radius_point(
    registry: registry.EntityRegistry,
    pid: types.EntityID,
) -> bool
```

Whether the point is this array's guide radius point.

| Parameter  | Type                      | Description |
| ---------- | ------------------------- | ----------- |
| `registry` | `registry.EntityRegistry` |             |
| `pid`      | `types.EntityID`          |             |
| _Returns_  | `bool`                    |             |

### `living_entity_ids()`

```python
living_entity_ids(registry: registry.EntityRegistry) -> list[types.EntityID]
```

Flat list of surviving member entity ids.

| Parameter  | Type                      | Description |
| ---------- | ------------------------- | ----------- |
| `registry` | `registry.EntityRegistry` |             |
| _Returns_  | `list[types.EntityID]`    |             |

### `living_members()`

```python
living_members(
    registry: registry.EntityRegistry,
) -> list[tuple[int, list[types.EntityID]]]
```

Living members, pruned to surviving entities.

| Parameter  | Type                                     | Description |
| ---------- | ---------------------------------------- | ----------- |
| `registry` | `registry.EntityRegistry`                |             |
| _Returns_  | `list[tuple[int, list[types.EntityID]]]` |             |

### `make_strategy()`

```python
make_strategy(registry: registry.EntityRegistry) -> Any
```

Builds the strategy for this definition.

| Parameter  | Type                      | Description |
| ---------- | ------------------------- | ----------- |
| `registry` | `registry.EntityRegistry` |             |
| _Returns_  | `Any`                     |             |

### `occupied_slots()`

```python
occupied_slots(registry: registry.EntityRegistry) -> list[int]
```

Slot numbers of surviving members.

| Parameter  | Type                      | Description |
| ---------- | ------------------------- | ----------- |
| `registry` | `registry.EntityRegistry` |             |
| _Returns_  | `list[int]`               |             |

### `params_changed()`

```python
params_changed(strategy: Any) -> bool
```

Whether a strategy requires full re-distribution.

| Parameter  | Type   | Description |
| ---------- | ------ | ----------- |
| `strategy` | `Any`  |             |
| _Returns_  | `bool` |             |

### `prune()`

```python
prune(registry: registry.EntityRegistry) -> bool
```

Drops deleted entities from member groups.

| Parameter  | Type                      | Description |
| ---------- | ------------------------- | ----------- |
| `registry` | `registry.EntityRegistry` |             |
| _Returns_  | `bool`                    |             |

### `reanchor_template()`

```python
reanchor_template(
    strategy: Any,
    registry: registry.EntityRegistry,
    template_eids: Sequence[types.EntityID],
    standalone_pids: Sequence[types.EntityID],
) -> None
```

Re-positions the template onto guide position 0.

| Parameter         | Type                       | Description |
| ----------------- | -------------------------- | ----------- |
| `strategy`        | `Any`                      |             |
| `registry`        | `registry.EntityRegistry`  |             |
| `template_eids`   | `Sequence[types.EntityID]` |             |
| `standalone_pids` | `Sequence[types.EntityID]` |             |
| _Returns_         | `None`                     |             |

### `refresh_caches()`

```python
refresh_caches(registry: registry.EntityRegistry, strategy: Any) -> None
```

Recomputes the sync caches from live geometry.

| Parameter  | Type                      | Description |
| ---------- | ------------------------- | ----------- |
| `registry` | `registry.EntityRegistry` |             |
| `strategy` | `Any`                     |             |
| _Returns_  | `None`                    |             |

### `restore()`

```python
restore(state: Any) -> None
```

Restores editable state from a snapshot.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `state`   | `Any`  |             |
| _Returns_ | `None` |             |

### `signatures_changed()`

```python
signatures_changed(guide_sig: Any, template_sig: Any) -> bool
```

True if stored sync caches disagree with the given signatures.

| Parameter      | Type   | Description |
| -------------- | ------ | ----------- |
| `guide_sig`    | `Any`  |             |
| `template_sig` | `Any`  |             |
| _Returns_      | `bool` |             |

### `snapshot()`

```python
snapshot() -> Any
```

Captures editable state for undo.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `Any` |             |

### `template_signature()`

```python
template_signature(registry: registry.EntityRegistry) -> Any
```

Quantized template geometry signature.

| Parameter  | Type                      | Description |
| ---------- | ------------------------- | ----------- |
| `registry` | `registry.EntityRegistry` |             |
| _Returns_  | `Any`                     |             |

### `to_dict()`

```python
to_dict() -> dict
```

Serializes the definition.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

### `update_caches()`

```python
update_caches(guide_sig: Any, template_sig: Any) -> None
```

Stores the sync signatures after a cycle.

| Parameter      | Type   | Description |
| -------------- | ------ | ----------- |
| `guide_sig`    | `Any`  |             |
| `template_sig` | `Any`  |             |
| _Returns_      | `None` |             |

## ArrayStrategy

Shared base class for array strategies.

### `count`

```python
count: int
```

### `needs_center_point`

```python
needs_center_point: bool
```

### `rotate_copies`

```python
rotate_copies: bool
```

### `uses_template_anchor`

```python
uses_template_anchor: bool
```

### `apply_frame()`

```python
apply_frame(
    registry: registry.EntityRegistry,
    array: Array,
    old_frame: tuple[tuple[float, float], float],
) -> None
```

| Parameter   | Type                                | Description |
| ----------- | ----------------------------------- | ----------- |
| `registry`  | `registry.EntityRegistry`           |             |
| `array`     | `Array`                             |             |
| `old_frame` | `tuple[tuple[float, float], float]` |             |
| _Returns_   | `None`                              |             |

### `capture_master_frame()`

```python
capture_master_frame(
    registry: registry.EntityRegistry,
    array: Array,
) -> Optional[tuple[tuple[float, float], float]]
```

| Parameter  | Type                                          | Description |
| ---------- | --------------------------------------------- | ----------- |
| `registry` | `registry.EntityRegistry`                     |             |
| `array`    | `Array`                                       |             |
| _Returns_  | `Optional[tuple[tuple[float, float], float]]` |             |

### `create_master_geometry()`

```python
create_master_geometry(
    center_pid: Optional[types.EntityID],
    radius_pt_pid: Optional[types.EntityID],
) -> tuple[list[Any], list[Any], list[Any]]
```

| Parameter       | Type                                     | Description |
| --------------- | ---------------------------------------- | ----------- |
| `center_pid`    | `Optional[types.EntityID]`               |             |
| `radius_pt_pid` | `Optional[types.EntityID]`               |             |
| _Returns_       | `tuple[list[Any], list[Any], list[Any]]` |             |

### `existing_master_id()`

```python
existing_master_id() -> Optional[int]
```

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| _Returns_ | `Optional[int]` |             |

### `member_placements()`

```python
member_placements(
    template_center: tuple[float, float],
    registry: Optional[registry.EntityRegistry] = None,
) -> list[InstancePlacement]
```

| Parameter         | Type                                       | Description |
| ----------------- | ------------------------------------------ | ----------- |
| `template_center` | `tuple[float, float]`                      |             |
| `registry`        | `Optional[registry.EntityRegistry] = None` |             |
| _Returns_         | `list[InstancePlacement]`                  |             |

### `template_placement()`

```python
template_placement(
    template_center: tuple[float, float],
    registry: Optional[registry.EntityRegistry] = None,
) -> InstancePlacement
```

| Parameter         | Type                                       | Description |
| ----------------- | ------------------------------------------ | ----------- |
| `template_center` | `tuple[float, float]`                      |             |
| `registry`        | `Optional[registry.EntityRegistry] = None` |             |
| _Returns_         | `InstancePlacement`                        |             |

## CircularArray

Concrete circular array.

## CircularArrayStrategy

Circular-array placement strategy.

### `center`

```python
center: tuple[float, float]
```

### `radius`

```python
radius: float
```

### `total_angle_deg`

```python
total_angle_deg: float
```

## CurveAlongArray

Concrete curve-along array.

## CurveAlongArrayStrategy

Curve-along placement strategy.

### `align_to_tangent`

```python
align_to_tangent: bool
```

### `offset_to_start`

```python
offset_to_start: float
```

### `path_entity_id`

```python
path_entity_id: int
```

### `spacing`

```python
spacing: float
```

## InstancePlacement

A rigid placement of one array instance.

### `angle`

```python
angle: float
```

### `center`

```python
center: tuple[float, float]
```

### `delta`

```python
delta: tuple[float, float]
```

### `kind`

```python
kind: str
```

Placement kind: `rotation`, `translation` or `curve_aligned`.

### `target_center`

```python
target_center: tuple[float, float]
```

### `transform_offset()`

```python
transform_offset(dx: float, dy: float) -> tuple[float, float]
```

Transforms a relative offset through the placement.

| Parameter | Type                  | Description |
| --------- | --------------------- | ----------- |
| `dx`      | `float`               |             |
| `dy`      | `float`               |             |
| _Returns_ | `tuple[float, float]` |             |

### `transform_point()`

```python
transform_point(x: float, y: float) -> tuple[float, float]
```

Transforms an absolute point through the placement.

| Parameter | Type                  | Description |
| --------- | --------------------- | ----------- |
| `x`       | `float`               |             |
| `y`       | `float`               |             |
| _Returns_ | `tuple[float, float]` |             |

## PlacementKind

Placement kind constants.

## Functions

### `_cumulative_lengths()`

```python
_cumulative_lengths(polyline: Sequence[tuple[float, float]]) -> list[float]
```

Cumulative arc lengths of a polyline.

| Parameter  | Type                            | Description |
| ---------- | ------------------------------- | ----------- |
| `polyline` | `Sequence[tuple[float, float]]` |             |
| _Returns_  | `list[float]`                   |             |

### `_point_at_arclength()`

```python
_point_at_arclength(
    polyline: Sequence[tuple[float, float]],
    cum: Sequence[float],
    s: float,
) -> tuple[tuple[float, float], tuple[float, float]]
```

Point and unit tangent at an arc length.

| Parameter  | Type                                              | Description |
| ---------- | ------------------------------------------------- | ----------- |
| `polyline` | `Sequence[tuple[float, float]]`                   |             |
| `cum`      | `Sequence[float]`                                 |             |
| `s`        | `float`                                           |             |
| _Returns_  | `tuple[tuple[float, float], tuple[float, float]]` |             |

### `find_array_for_entity()`

```python
find_array_for_entity(
    arrays: Sequence[Array],
    entity_id: types.EntityID,
) -> Optional[Array]
```

Returns the array whose master entity is the given one.

| Parameter   | Type              | Description |
| ----------- | ----------------- | ----------- |
| `arrays`    | `Sequence[Array]` |             |
| `entity_id` | `types.EntityID`  |             |
| _Returns_   | `Optional[Array]` |             |

### `path_length()`

```python
path_length(
    registry: registry.EntityRegistry,
    path_entity_id: types.EntityID,
) -> float
```

Total arc length of a guide path.

| Parameter        | Type                      | Description |
| ---------------- | ------------------------- | ----------- |
| `registry`       | `registry.EntityRegistry` |             |
| `path_entity_id` | `types.EntityID`          |             |
| _Returns_        | `float`                   |             |

### `resolve_template_center()`

```python
resolve_template_center(
    registry: registry.EntityRegistry,
    template_entity_ids: Sequence[types.EntityID],
    _template_points: Any,
) -> tuple[float, float]
```

Returns the logical center of the template entities.

| Parameter             | Type                       | Description |
| --------------------- | -------------------------- | ----------- |
| `registry`            | `registry.EntityRegistry`  |             |
| `template_entity_ids` | `Sequence[types.EntityID]` |             |
| `_template_points`    | `Any`                      |             |
| _Returns_             | `tuple[float, float]`      |             |

### `sample_path()`

```python
sample_path(
    registry: registry.EntityRegistry,
    path_entity_id: types.EntityID,
    count: int,
    offset_to_start: float = 0.0,
) -> list[tuple[tuple[float, float], float]]
```

Samples a guide path at equal arc-length intervals.

| Parameter         | Type                                      | Description |
| ----------------- | ----------------------------------------- | ----------- |
| `registry`        | `registry.EntityRegistry`                 |             |
| `path_entity_id`  | `types.EntityID`                          |             |
| `count`           | `int`                                     |             |
| `offset_to_start` | `float = 0.0`                             |             |
| _Returns_         | `list[tuple[tuple[float, float], float]]` |             |
