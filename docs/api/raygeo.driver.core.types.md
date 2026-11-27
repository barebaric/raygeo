---
title: raygeo.driver.core.types
sidebar_label: raygeo.driver.core.types
---

Shared device value types: DeviceState, DeviceStatus and DeviceError. These mirror the
identically-named types in Rayforge so the Python shell can convert losslessly.

## DeviceError

An error or alarm with code, title and description.

### `code`

```python
code: int
```

### `description`

```python
description: str
```

### `title`

```python
title: str
```

## DeviceState

The complete state of a device at a moment in time.

### `buffer_available`

```python
buffer_available: Optional[int]
```

### `buffer_rx_available`

```python
buffer_rx_available: Optional[int]
```

### `error`

```python
error: Optional[DeviceError]
```

### `feed_rate`

```python
feed_rate: Optional[int]
```

### `machine_pos`

```python
machine_pos: list[Optional[float]]
```

Machine position in mm; entries may be `None`.

### `spindle_speed`

```python
spindle_speed: Optional[int]
```

### `status`

```python
status: DeviceStatus
```

### `wco`

```python
wco: list[Optional[float]]
```

Work coordinate offset in mm; entries may be `None`.

### `work_pos`

```python
work_pos: list[Optional[float]]
```

Work position in mm; entries may be `None`.

## DeviceStatus

Machine state as reported in Grbl status reports.

**Values:**

- `ALARM`
- `CHECK`
- `CYCLE`
- `DOOR`
- `HOLD`
- `HOME`
- `IDLE`
- `JOG`
- `LOCK`
- `QUEUE`
- `RUN`
- `SLEEP`
- `TEST`
- `TOOL`
- `UNKNOWN`
- `UNLOCK`

### `name`

```python
name: str
```
