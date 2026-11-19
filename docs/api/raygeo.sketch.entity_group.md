---
title: raygeo.sketch.entity_group
sidebar_label: raygeo.sketch.entity_group
---

## EntityGroup

An ephemeral view over `(registry, entity_ids)`: the surviving entities plus their unique defining
points, manipulable as one piece of geometry.

### `entity_ids`

```python
entity_ids: list[types.EntityID]
```

The group's entity ids, in group order.

### `apply_placement()`

```python
apply_placement(placement: Any) -> None
```

Rigidly moves the group's defining points and internal offsets by the placement transform.

| Parameter   | Type   | Description |
| ----------- | ------ | ----------- |
| `placement` | `Any`  |             |
| _Returns_   | `None` |             |

### `apply_rigid_motion()`

```python
apply_rigid_motion(motion: Any) -> None
```

Applies a shape-preserving placement transform.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `motion`  | `Any`  |             |
| _Returns_ | `None` |             |

### `center()`

```python
center() -> tuple[float, float]
```

Returns the logical center of the group's entities.

| Parameter | Type                  | Description |
| --------- | --------------------- | ----------- |
| _Returns_ | `tuple[float, float]` |             |

### `entities()`

```python
entities() -> list
```

Returns the group's surviving entities, in group order.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `list` |             |

### `helper_ids()`

```python
helper_ids() -> list[types.EntityID]
```

Returns helper geometry belonging to the group's entities.

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| _Returns_ | `list[types.EntityID]` |             |

### `point_ids()`

```python
point_ids() -> list[types.EntityID]
```

Returns the group's unique point ids in stable order.

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| _Returns_ | `list[types.EntityID]` |             |

### `points()`

```python
points() -> list
```

Returns the group's unique defining points.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `list` |             |

### `polylines()`

```python
polylines() -> list[list[tuple[float, float]]]
```

Samples every entity of the group into polylines.

| Parameter | Type                              | Description |
| --------- | --------------------------------- | ----------- |
| _Returns_ | `list[list[tuple[float, float]]]` |             |

### `radial_project()`

```python
radial_project(
    center: tuple[float, float],
    radius: float,
    pin: Optional[tuple[float, float]] = None,
) -> None
```

Translates the group radially so its center sits on a circle.

| Parameter | Type                                   | Description |
| --------- | -------------------------------------- | ----------- |
| `center`  | `tuple[float, float]`                  |             |
| `radius`  | `float`                                |             |
| `pin`     | `Optional[tuple[float, float]] = None` |             |
| _Returns_ | `None`                                 |             |

### `remap_point_refs()`

```python
remap_point_refs(pid_map: Mapping[int, int]) -> None
```

Rewrites the group's entities' point ID references.

| Parameter | Type                | Description |
| --------- | ------------------- | ----------- |
| `pid_map` | `Mapping[int, int]` |             |
| _Returns_ | `None`              |             |

### `restore_positions()`

```python
restore_positions(snapshot: Any) -> None
```

Restores point positions captured by `snapshot_positions`.

| Parameter  | Type   | Description |
| ---------- | ------ | ----------- |
| `snapshot` | `Any`  |             |
| _Returns_  | `None` |             |

### `rewrite_copy_from()`

```python
rewrite_copy_from(
    copy: EntityGroup,
    placement: Any,
    extra_point_pairs: Optional[Sequence[tuple[types.EntityID, types.EntityID]]] = None,
) -> list
```

Rewrites an existing copy group from this template group.

| Parameter           | Type                                                               | Description |
| ------------------- | ------------------------------------------------------------------ | ----------- |
| `copy`              | `EntityGroup`                                                      |             |
| `placement`         | `Any`                                                              |             |
| `extra_point_pairs` | `Optional[Sequence[tuple[types.EntityID, types.EntityID]]] = None` |             |
| _Returns_           | `list`                                                             |             |

### `snapshot_positions()`

```python
snapshot_positions() -> list
```

Captures the current position of every point of the group.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `list` |             |

### `translate()`

```python
translate(dx: float, dy: float) -> None
```

Moves every point of the group by the given delta.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `dx`      | `float` |             |
| `dy`      | `float` |             |
| _Returns_ | `None`  |             |

### `with_helpers()`

```python
with_helpers() -> EntityGroup
```

Returns a view extended with this group's helper entities.

| Parameter | Type          | Description |
| --------- | ------------- | ----------- |
| _Returns_ | `EntityGroup` |             |

## Functions

### `points_bbox_center()`

```python
points_bbox_center(points: Any) -> tuple[float, float]
```

Returns the bbox center of a list of points.

| Parameter | Type                  | Description |
| --------- | --------------------- | ----------- |
| `points`  | `Any`                 |             |
| _Returns_ | `tuple[float, float]` |             |

### `remap_point_refs()`

```python
remap_point_refs(obj: Any, pid_map: Mapping[int, int]) -> None
```

Rewrites point ID references on an entity or constraint.

| Parameter | Type                | Description |
| --------- | ------------------- | ----------- |
| `obj`     | `Any`               |             |
| `pid_map` | `Mapping[int, int]` |             |
| _Returns_ | `None`              |             |
