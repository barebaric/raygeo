---
title: raygeo.sketch.document
sidebar_label: raygeo.sketch.document
---

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
