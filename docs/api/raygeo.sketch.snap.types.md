---
title: raygeo.sketch.snap.types
sidebar_label: raygeo.sketch.snap.types
---

## DragContext

### `dragged_entity_ids`

```python
dragged_entity_ids: set
```

### `dragged_point_ids`

```python
dragged_point_ids: set
```

### `initial_positions`

```python
initial_positions: dict[int, tuple[float, float]]
```

### `preview_entity_ids`

```python
preview_entity_ids: set
```

### `suppress_coincident_points`

```python
suppress_coincident_points: bool
```

### `coincides_with_dragged()`

```python
coincides_with_dragged(
    x: float,
    y: float,
    registry: Any,
    epsilon: float = 1e-06,
) -> bool
```

| Parameter  | Type            | Description |
| ---------- | --------------- | ----------- |
| `x`        | `float`         |             |
| `y`        | `float`         |             |
| `registry` | `Any`           |             |
| `epsilon`  | `float = 1e-06` |             |
| _Returns_  | `bool`          |             |

### `is_entity_dragged()`

```python
is_entity_dragged(entity_id: int) -> bool
```

| Parameter   | Type   | Description |
| ----------- | ------ | ----------- |
| `entity_id` | `int`  |             |
| _Returns_   | `bool` |             |

### `is_point_dragged()`

```python
is_point_dragged(point_id: int) -> bool
```

| Parameter  | Type   | Description |
| ---------- | ------ | ----------- |
| `point_id` | `int`  |             |
| _Returns_  | `bool` |             |

### `is_preview_entity()`

```python
is_preview_entity(entity_id: int) -> bool
```

| Parameter   | Type   | Description |
| ----------- | ------ | ----------- |
| `entity_id` | `int`  |             |
| _Returns_   | `bool` |             |

## SnapLine

### `coordinate`

```python
coordinate: float
```

### `is_horizontal`

```python
is_horizontal: bool
```

### `line_type`

```python
line_type: SnapLineType
```

### `source`

```python
source: Optional[Any]
```

### `style`

```python
style: SnapLineStyle
```

### `distance_to()`

```python
distance_to(x: float, y: float) -> float
```

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `x`       | `float` |             |
| `y`       | `float` |             |
| _Returns_ | `float` |             |

### `get_snap_position()`

```python
get_snap_position(x: float, y: float) -> tuple[float, float]
```

| Parameter | Type                  | Description |
| --------- | --------------------- | ----------- |
| `x`       | `float`               |             |
| `y`       | `float`               |             |
| _Returns_ | `tuple[float, float]` |             |

## SnapLineStyle

### `color`

```python
color: tuple[float, float, float, float]
```

### `dash`

```python
dash: Optional[tuple]
```

### `line_width`

```python
line_width: float
```

## SnapLineType

### `name`

```python
name: str
```

### `priority`

```python
priority: int
```

### `value`

```python
value: int
```

## SnapPoint

### `axis_coord`

```python
axis_coord: Optional[float]
```

### `is_horizontal`

```python
is_horizontal: bool
```

### `line_type`

```python
line_type: SnapLineType
```

### `pattern_coords`

```python
pattern_coords: Optional[tuple]
```

### `pos`

```python
pos: tuple[float, float]
```

### `source`

```python
source: Optional[Any]
```

### `spacing`

```python
spacing: Optional[float]
```

### `x`

```python
x: float
```

### `y`

```python
y: float
```

## SnapResult

### `distance`

```python
distance: float
```

### `position`

```python
position: tuple[float, float]
```

### `primary_snap_line`

```python
primary_snap_line: Optional[SnapLine]
```

### `primary_snap_point`

```python
primary_snap_point: Optional[SnapPoint]
```

### `snap_lines`

```python
snap_lines: list[SnapLine]
```

### `snap_points`

```python
snap_points: list[SnapPoint]
```

### `snapped`

```python
snapped: bool
```

### `from_snap_line()`

```python
@classmethod
from_snap_line(
    snap_line: SnapLine,
    original_pos: tuple[float, float],
    distance: float,
) -> SnapResult
```

| Parameter      | Type                  | Description |
| -------------- | --------------------- | ----------- |
| `snap_line`    | `SnapLine`            |             |
| `original_pos` | `tuple[float, float]` |             |
| `distance`     | `float`               |             |
| _Returns_      | `SnapResult`          |             |

### `from_snap_point()`

```python
@classmethod
from_snap_point(
    snap_point: SnapPoint,
    distance: float,
    snap_lines: Optional[Sequence[SnapLine]] = None,
) -> SnapResult
```

| Parameter    | Type                                  | Description |
| ------------ | ------------------------------------- | ----------- |
| `snap_point` | `SnapPoint`                           |             |
| `distance`   | `float`                               |             |
| `snap_lines` | `Optional[Sequence[SnapLine]] = None` |             |
| _Returns_    | `SnapResult`                          |             |

### `no_snap()`

```python
@classmethod no_snap(position: tuple[float, float]) -> SnapResult
```

| Parameter  | Type                  | Description |
| ---------- | --------------------- | ----------- |
| `position` | `tuple[float, float]` |             |
| _Returns_  | `SnapResult`          |             |
