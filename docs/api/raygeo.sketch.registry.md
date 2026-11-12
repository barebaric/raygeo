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

### `get_point()`

```python
get_point(id: int) -> Optional[tuple[float, float]]
```

Coordinates `(x, y)` of a point, or `None`.

| Parameter | Type                            | Description |
| --------- | ------------------------------- | ----------- |
| `id`      | `int`                           |             |
| _Returns_ | `Optional[tuple[float, float]]` |             |

### `is_point_used()`

```python
is_point_used(id: int) -> bool
```

Whether the point is referenced by any entity.

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

### `to_dict()`

```python
to_dict() -> dict
```

Serializes the model to a reference registry dictionary.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |
