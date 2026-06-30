---
title: raygeo.ops.assembly.adaptive.tool
sidebar_label: raygeo.ops.assembly.adaptive.tool
---

## Tool

Cutting-tool state for adaptive clearing.

Holds the tool centre position, heading, and the steering predictor / gyroscope buffers used to
smooth the walking path. Construct with `Tool(pos, heading, radius)` and feed direction vectors via
`push_gyro` between solver steps.

### `heading`

```python
heading: float
```

Current heading angle in radians.

### `pos`

```python
pos: tuple[float, float]
```

Tool centre position `(x, y)`.

### `radius`

```python
radius: float
```

Tool radius in mm.

### `predicted_angle()`

```python
predicted_angle(max_deflection: float) -> float
```

Predictor seed for the engagement solver, clamped to a fraction of *max_deflection*.

| Parameter        | Type    | Description |
| ---------------- | ------- | ----------- |
| `max_deflection` | `float` |             |
| _Returns_        | `float` |             |

### `push_gyro()`

```python
push_gyro(dir: tuple[float, float]) -> None
```

Push a direction vector `(dx, dy)` into the gyroscope buffer.

| Parameter | Type                  | Description |
| --------- | --------------------- | ----------- |
| `dir`     | `tuple[float, float]` |             |
| _Returns_ | `None`                |             |

### `raw_predictor()`

```python
raw_predictor() -> float
```

Raw (un-clamped) predictor value.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| _Returns_ | `float` |             |

### `reset_gyro()`

```python
reset_gyro() -> None
```

Reset the gyroscope and predictor history to the current heading.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `smoothed_heading()`

```python
smoothed_heading() -> float
```

Gyroscope-smoothed heading (radians), averaged over recent direction vectors.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| _Returns_ | `float` |             |

### `update_predictor()`

```python
update_predictor(delta: float) -> None
```

Update the decayed steering predictor with a converged deflection.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `delta`   | `float` |             |
| _Returns_ | `None`  |             |
