---
title: raygeo.driver.marlin.probe
sidebar_label: raygeo.driver.marlin.probe
---

## MarlinDeviceInfo

The structured result of probing a Marlin device: firmware identity, software-endstop travel and
motion limits.

### `acceleration`

```python
acceleration: Optional[int]
```

### `firmware_version`

```python
firmware_version: Optional[str]
```

### `machine_type`

```python
machine_type: Optional[str]
```

### `max_speed`

```python
max_speed: Optional[int]
```

### `max_travel`

```python
max_travel: Optional[tuple[float, float]]
```

### `name`

```python
name: str
```

### `units`

```python
units: Optional[str]
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
    m115_lines: Sequence[str],
    m211_lines: Sequence[str],
    m503_lines: Sequence[str],
    m149_lines: Sequence[str],
    boot_lines: Optional[Sequence[str]] = None,
) -> MarlinDeviceInfo
```

Builds the census from the raw Marlin census-command responses.

Pure data transformation with no I/O; the caller is responsible for communicating with the device
and passing the collected response lines. `boot_lines` are startup messages observed before the
handshake; a version found there is more specific than the `FIRMWARE_NAME` token M115 reports.

| Parameter    | Type                             | Description |
| ------------ | -------------------------------- | ----------- |
| `m115_lines` | `Sequence[str]`                  |             |
| `m211_lines` | `Sequence[str]`                  |             |
| `m503_lines` | `Sequence[str]`                  |             |
| `m149_lines` | `Sequence[str]`                  |             |
| `boot_lines` | `Optional[Sequence[str]] = None` |             |
| _Returns_    | `MarlinDeviceInfo`               |             |

### `probe_device()`

```python
probe_device(
    session: Any,
    boot_lines: Optional[Sequence[str]] = None,
) -> Awaitable[MarlinDeviceInfo]
```

Runs the device census over *session* and resolves to \[`MarlinDeviceInfo`\]. *boot_lines* are the
startup lines the caller observed.

| Parameter    | Type                             | Description |
| ------------ | -------------------------------- | ----------- |
| `session`    | `Any`                            |             |
| `boot_lines` | `Optional[Sequence[str]] = None` |             |
| _Returns_    | `Awaitable[MarlinDeviceInfo]`    |             |
