---
title: raygeo.driver.smoothie.probe
sidebar_label: raygeo.driver.smoothie.probe
---

## SmoothieDeviceInfo

The structured result of probing a Smoothieware device: firmware identity and the motion limits
applications need.

### `acceleration`

```python
acceleration: Optional[float]
```

### `acceleration_steps`

```python
acceleration_steps: Optional[int]
```

### `alpha_max`

```python
alpha_max: Optional[float]
```

### `alpha_rate`

```python
alpha_rate: Optional[float]
```

### `beta_max`

```python
beta_max: Optional[float]
```

### `beta_rate`

```python
beta_rate: Optional[float]
```

### `firmware_version`

```python
firmware_version: Optional[str]
```

### `max_speed`

```python
max_speed: Optional[int]
```

### `max_travel`

```python
max_travel: Optional[tuple[float, float]]
```

### `to_dict()`

```python
to_dict() -> dict
```

The mapping form of the census (the daemon wire form).

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## Functions

### `describe()`

```python
describe(
    version_lines: Sequence[str],
    alpha_max_lines: Sequence[str],
    beta_max_lines: Sequence[str],
    alpha_max_rate_lines: Sequence[str],
    beta_max_rate_lines: Sequence[str],
    acceleration_lines: Sequence[str],
) -> SmoothieDeviceInfo
```

Builds the census from raw Smoothieware census-command responses.

Pure data transformation with no I/O; the caller is responsible for communicating with the device
and passing the collected response lines. `ok` acknowledgements and `<...>` status reports are
dropped, and non-numeric `config-get` replies yield `None` fields.

| Parameter              | Type                 | Description |
| ---------------------- | -------------------- | ----------- |
| `version_lines`        | `Sequence[str]`      |             |
| `alpha_max_lines`      | `Sequence[str]`      |             |
| `beta_max_lines`       | `Sequence[str]`      |             |
| `alpha_max_rate_lines` | `Sequence[str]`      |             |
| `beta_max_rate_lines`  | `Sequence[str]`      |             |
| `acceleration_lines`   | `Sequence[str]`      |             |
| _Returns_              | `SmoothieDeviceInfo` |             |

### `probe_device()`

```python
probe_device(session: Any) -> Awaitable[SmoothieDeviceInfo]
```

Runs the device census over *session* and resolves to \[`SmoothieDeviceInfo`\].

| Parameter | Type                            | Description |
| --------- | ------------------------------- | ----------- |
| `session` | `Any`                           |             |
| _Returns_ | `Awaitable[SmoothieDeviceInfo]` |             |
