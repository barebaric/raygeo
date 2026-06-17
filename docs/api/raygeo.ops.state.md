---
title: raygeo.ops.state
sidebar_label: raygeo.ops.state
sidebar_position: 41
---

Machine state tracking for laser cutting and CNC milling.

Tracks the current or intended machine state at any point in a command sequence, including power
level (0.0–1.0), coolant mode, cut speed and travel speed, active laser source UID, pulse frequency,
and pulse width. State objects are used by Ops to associate machine parameters with moving commands
and to detect rapid (non-power) state changes.

## CoolantMode

Coolant mode for CNC milling operations.

Controls the coolant state: `Off`, `Flood`, `Mist`, or `Air`.

### `name`

```python
name: str
```

### `value`

```python
value: int
```

## State

The current state of a laser cutting or CNC milling machine.

Tracks power level, coolant mode, cut/travel speeds, active laser UID, frequency, pulse width,
spindle speed, and coolant mode.

### `active_laser_uid`

```python
active_laser_uid: Optional[str]
```

UID of the active laser source (if set).

### `coolant`

```python
coolant: Optional[CoolantMode]
```

Coolant mode (if set).

### `cut_speed`

```python
cut_speed: Optional[int]
```

Cutting speed in mm/s (if set).

### `dwell_ms`

```python
dwell_ms: Optional[float]
```

Dwell time in milliseconds (if set).

### `frequency`

```python
frequency: Optional[int]
```

Laser pulse frequency in Hz (if set).

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

### `spindle_speed`

```python
spindle_speed: Optional[int]
```

Spindle speed in RPM (if set).

### `travel_speed`

```python
travel_speed: Optional[int]
```

Travel (rapid) speed in mm/s (if set).

### `allow_rapid_change()`

```python
allow_rapid_change(target: State) -> bool
```

Check whether the machine can transition from the current state to the _target_ state without a
`SetPower` command.

| Parameter | Type    | Description                                       |
| --------- | ------- | ------------------------------------------------- |
| `target`  | `State` | The target state to compare against.              |
| _Returns_ | `bool`  | True if the change is a rapid (non-power) change. |
