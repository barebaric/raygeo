---
title: raygeo.driver.ruida
sidebar_label: raygeo.driver.ruida
---

Ruida laser controllers implemented in Rust. RuidaDriver speaks the binary rpascript protocol over
USB, UDP or TCP: it runs complete job scripts, executes live motion and job-control commands, polls
the controller status and reports the machine state through the uniform driver event surface. Job
compilation from Ops lives in the encoder; this session consumes rpascript text.

## RuidaDriver

A direct connection to a Ruida laser controller.

All async methods return asyncio futures; a typed `DriverEvent` is delivered to
`event_callback(event)` on the event loop thread. `run()` and `run_raw()` take rpascript text; the
machine-code encoder produces it from Ops.

### `machine_status`

```python
machine_status: dict
```

The last decoded controller status.

### `protect_enabled`

```python
protect_enabled: bool
```

Whether protect mode blocks `SET_SETTING`.

### `execute()`

```python
execute(line: str) -> Awaitable[None]
```

Send one raw rpascript line.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `line`    | `str`             |             |
| _Returns_ | `Awaitable[None]` |             |

### `focus_z()`

```python
focus_z() -> Awaitable[None]
```

Run the controller's Z auto-focus routine.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `Awaitable[None]` |             |

### `get_setting()`

```python
get_setting(address: int) -> Awaitable[list]
```

Read one controller memory address; returns `(address, data)` pairs.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `address` | `int`             |             |
| _Returns_ | `Awaitable[list]` |             |

### `home_u()`

```python
home_u() -> Awaitable[None]
```

Home the U axis.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `Awaitable[None]` |             |

### `home_xy()`

```python
home_xy() -> Awaitable[None]
```

Move the XY axes to the current origin.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `Awaitable[None]` |             |

### `home_z()`

```python
home_z() -> Awaitable[None]
```

Home the Z axis through the controller's homing routine.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `Awaitable[None]` |             |

### `jog_axis_rel()`

```python
jog_axis_rel(axis: str, delta: float) -> Awaitable[None]
```

Jog one axis relatively in millimetres.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `axis`    | `str`             |             |
| `delta`   | `float`           |             |
| _Returns_ | `Awaitable[None]` |             |

### `jog_u_to()`

```python
jog_u_to(u: float) -> Awaitable[None]
```

Jog to an absolute U position in millimetres.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `u`       | `float`           |             |
| _Returns_ | `Awaitable[None]` |             |

### `jog_x_to()`

```python
jog_x_to(x: float) -> Awaitable[None]
```

Jog to an absolute X position in millimetres.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `x`       | `float`           |             |
| _Returns_ | `Awaitable[None]` |             |

### `jog_xy_rel()`

```python
jog_xy_rel(
    x: Optional[float] = None,
    y: Optional[float] = None,
) -> Awaitable[None]
```

Jog XY relatively in millimetres.

| Parameter | Type                     | Description |
| --------- | ------------------------ | ----------- |
| `x`       | `Optional[float] = None` |             |
| `y`       | `Optional[float] = None` |             |
| _Returns_ | `Awaitable[None]`        |             |

### `jog_xy_to()`

```python
jog_xy_to(x: float, y: float) -> Awaitable[None]
```

Jog to an absolute XY position in millimetres.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `x`       | `float`           |             |
| `y`       | `float`           |             |
| _Returns_ | `Awaitable[None]` |             |

### `jog_y_to()`

```python
jog_y_to(y: float) -> Awaitable[None]
```

Jog to an absolute Y position in millimetres.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `y`       | `float`           |             |
| _Returns_ | `Awaitable[None]` |             |

### `jog_z_to()`

```python
jog_z_to(z: float) -> Awaitable[None]
```

Jog to an absolute Z position in millimetres.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `z`       | `float`           |             |
| _Returns_ | `Awaitable[None]` |             |

### `reset()`

```python
reset() -> Awaitable[None]
```

Stop the job and home XY, like the panel's reset key.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `Awaitable[None]` |             |

### `set_jog_speeds()`

```python
set_jog_speeds(xy: float, z: float, u: float) -> None
```

Set the jog speeds in mm/s.

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `xy`      | `float` |             |
| `z`       | `float` |             |
| `u`       | `float` |             |
| _Returns_ | `None`  |             |

### `set_protect()`

```python
set_protect(enabled: bool) -> None
```

Block or allow `SET_SETTING` commands.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `enabled` | `bool` |             |
| _Returns_ | `None` |             |

### `set_setting()`

```python
set_setting(address: int, a: int, b: int) -> Awaitable[None]
```

Write one controller memory address. Blocked while protect mode is on.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `address` | `int`             |             |
| `a`       | `int`             |             |
| `b`       | `int`             |             |
| _Returns_ | `Awaitable[None]` |             |

## Functions

### `encode_script()`

```python
encode_script(lines: Sequence[str]) -> list[int]
```

Encode rpascript lines into the controller's raw byte stream, including the file checksum.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| `lines`   | `Sequence[str]` |             |
| _Returns_ | `list[int]`     |             |
