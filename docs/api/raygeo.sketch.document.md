---
title: raygeo.sketch.document
sidebar_label: raygeo.sketch.document
---

## DocumentMeta

Locating metadata of a sketch document.

### `format_version`

```python
format_version: int
```

Sketch file format version.

### `hidden`

```python
hidden: bool
```

Whether the sketch is hidden.

### `name`

```python
name: str
```

Display name.

### `origin_id`

```python
origin_id: types.EntityID
```

Id of the fixed origin point.

### `uid`

```python
uid: str
```

Asset uid.

### `copy()`

```python
copy() -> DocumentMeta
```

Creates a copy of the metadata.

| Parameter | Type           | Description |
| --------- | -------------- | ----------- |
| _Returns_ | `DocumentMeta` |             |

### `from_dict()`

```python
from_dict(dict: dict) -> DocumentMeta
```

Deserializes metadata from a sketch dictionary.

| Parameter | Type           | Description |
| --------- | -------------- | ----------- |
| `dict`    | `dict`         |             |
| _Returns_ | `DocumentMeta` |             |

### `to_dict()`

```python
to_dict() -> dict
```

Serializes the metadata to dictionary fields.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## Fill

A filled area bounded by sketch entities.

### `boundary`

```python
boundary: list[tuple[types.EntityID, bool]]
```

Loop boundary as `(entity_id, forward)` pairs.

### `color`

```python
color: tuple[float, float, float, float]
```

Fill RGBA color.

### `gradient_angle`

```python
gradient_angle: float
```

Gradient angle in radians.

### `gradient_stops`

```python
gradient_stops: list[tuple[float, tuple[float, float, float, float]]]
```

Gradient stops as `(position, rgba)` pairs.

### `style`

```python
style: FillStyle
```

Fill rendering style.

### `uid`

```python
uid: str
```

Unique fill id.

### `from_dict()`

```python
from_dict(dict: dict) -> Fill
```

Deserializes a fill from the reference dictionary shape.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `dict`    | `dict` |             |
| _Returns_ | `Fill` |             |

### `to_dict()`

```python
to_dict() -> dict
```

Serializes the fill to the reference dictionary shape.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## FillStyle

Fill rendering style.

**Values:**

- `LINEAR_GRADIENT`
- `RADIAL_GRADIENT`
- `SOLID`
