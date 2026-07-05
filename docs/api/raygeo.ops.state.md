---
title: raygeo.ops.state
sidebar_label: raygeo.ops.state
---

Machine state tracking for CNC milling.

Tracks the current or intended machine state at any point in a command sequence, including power
level (0.0–1.0), coolant mode, air assist, head coolant, feed rate and rapid rate, active head UID,
pulse frequency, and pulse width. State objects are used by Ops to associate machine parameters with
moving commands and to detect rapid (non-power) state changes.

## AirAssistMode

Air assist mode for laser or CNC operations.

Controls the air assist state: `Off` or `On`.

### `name`

```python
name: str
```

### `value`

```python
value: int
```

## CoolantMode

Coolant mode for CNC milling operations.

Controls the coolant state: `Off`, `Flood`, or `Mist`.

### `name`

```python
name: str
```

### `value`

```python
value: int
```

## HeadCoolantMode

Head coolant mode for laser or CNC operations.

Controls the head coolant state: `Off` or `On`.

### `name`

```python
name: str
```

### `value`

```python
value: int
```

## State

The current state of a CNC machine.

Tracks power level, coolant mode, air assist, head coolant, feed/rapid rates, active head UID,
frequency, pulse width, spindle RPM, and coolant mode.

### `active_head_uid`

```python
active_head_uid: Optional[str]
```

UID of the active head (if set).

### `air_assist`

```python
air_assist: Optional[AirAssistMode]
```

Air assist mode (if set).

### `coolant`

```python
coolant: Optional[CoolantMode]
```

Coolant mode (if set).

### `dwell_ms`

```python
dwell_ms: Optional[float]
```

Dwell time in milliseconds (if set).

### `feed_rate`

```python
feed_rate: Optional[int]
```

Cutting feed rate in mm/s (if set).

### `frequency`

```python
frequency: Optional[int]
```

Laser pulse frequency in Hz (if set).

### `head_coolant`

```python
head_coolant: Optional[HeadCoolantMode]
```

Head coolant mode (if set).

### `power`

```python
power: float
```

Laser power level (0.0 – 1.0 typically).

### `pulse_width`

```python
pulse_width: Optional[float]
```

Laser pulse width in microseconds (if set).

### `rapid_rate`

```python
rapid_rate: Optional[int]
```

Rapid (traverse) rate in mm/s (if set).

### `spindle_rpm`

```python
spindle_rpm: Optional[int]
```

Spindle RPM (if set).

### `allow_rapid_change()`

```python
allow_rapid_change(target: State) -> bool
```

Check whether the machine can transition from the current state to the *target* state without a
`SetPower` command.

| Parameter    | Type    | Description                                       |
| ------------ | ------- | ------------------------------------------------- |
| `target`     | `State` | The target state to compare against.              |
| _Returns_    | `bool`  | True if the change is a rapid (non-power) change. |
| _Complexity_ |         | O(1)                                              |
