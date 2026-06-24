---
title: raygeo.ops.axis
sidebar_label: raygeo.ops.axis
sidebar_position: 57
---

Axis bitflag for multi-axis machines.

Represents a single axis or combination of axes (X, Y, Z, A, B, C, U). Axis values can be combined
using bitwise operators (|, &, ^, ~) to represent multiple axes at once, useful when specifying
which axes participate in a coordinated move or transformation.

## Axis

Represents a single axis or a combination of axes (X, Y, Z, A, B, C, U).

Axis values can be combined using bitwise operators (`|`, `&`, `^`, `~`) to represent multiple axes
at once.

### `label`

```python
label: str
```

The uppercase label of the axis (e.g. `"X"`, `"Y"`, `"Z"`).

### `name`

```python
name: str
```

The uppercase name of the axis (e.g. `"X"`, `"Y"`, `"Z"`).

Legacy alias for **label** to match Python `IntFlag.name`.

### `value`

```python
value: int
```

The raw bit value of the axis.

### `assert_single_axis()`

```python
assert_single_axis() -> None
```

Assert that this Axis represents exactly one axis (not a combination).

**Raises:** `ValueError` — If the axis mask contains multiple or zero bits set.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `from_name()`

```python
@classmethod from_name(name: str) -> Axis
```

Look up an Axis by its uppercase name.

**Raises:** `ValueError` — If the name is unknown.

| Parameter | Type   | Description                                |
| --------- | ------ | ------------------------------------------ |
| `name`    | `str`  | The uppercase letter (`"X"`, `"Y"`, etc.). |
| _Returns_ | `Axis` | The corresponding Axis constant.           |
