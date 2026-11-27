---
title: raygeo.driver.grbl.probe
sidebar_label: raygeo.driver.grbl.probe
---

## GrblDeviceInfo

The structured result of probing a GRBL device: firmware identity, compile flags, flavor
classification and the semantic values applications need.

### `acceleration`

```python
acceleration: Optional[int]
```

### `arc_tolerance`

```python
arc_tolerance: Optional[float]
```

### `compile_flags`

```python
compile_flags: str
```

### `firmware_version`

```python
firmware_version: Optional[str]
```

### `flavor`

```python
flavor: GrblFlavor
```

### `homing_enabled`

```python
homing_enabled: Optional[bool]
```

### `laser_mode`

```python
laser_mode: Optional[bool]
```

### `max_speed`

```python
max_speed: Optional[int]
```

### `max_spindle`

```python
max_spindle: Optional[int]
```

### `max_travel`

```python
max_travel: Optional[tuple[float, float]]
```

### `name`

```python
name: str
```

### `report_inches`

```python
report_inches: Optional[bool]
```

### `rx_buffer_size`

```python
rx_buffer_size: Optional[int]
```

### `settings`

```python
settings: dict[str, float]
```

### `single_axis_homing`

```python
single_axis_homing: bool
```

### `to_dict()`

```python
to_dict() -> dict
```

The mapping form of the census (the daemon wire form).

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## GrblFlavor

The firmware family a device identifies itself as.

**Values:**

- `FLUIDNC`
- `GRBLHAL`
- `STOCK`
- `UNKNOWN`

## Functions

### `describe()`

```python
describe(
    build_info: Sequence[str],
    settings_lines: Sequence[str],
) -> GrblDeviceInfo
```

Builds the census from the raw Grbl `$I` and `$$` response lines.

Pure data transformation with no I/O; the caller is responsible for communicating with the device
and passing the collected response lines.

| Parameter        | Type             | Description |
| ---------------- | ---------------- | ----------- |
| `build_info`     | `Sequence[str]`  |             |
| `settings_lines` | `Sequence[str]`  |             |
| _Returns_        | `GrblDeviceInfo` |             |

### `probe_device()`

```python
probe_device(session: Any) -> Awaitable[GrblDeviceInfo]
```

Runs the device census over *session* and resolves to \[`GrblDeviceInfo`\].

| Parameter | Type                        | Description |
| --------- | --------------------------- | ----------- |
| `session` | `Any`                       |             |
| _Returns_ | `Awaitable[GrblDeviceInfo]` |             |
