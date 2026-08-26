---
title: raygeo.ops.types
sidebar_label: raygeo.ops.types
---

Core enumerations for the Ops command system.

CommandType — identifies each command in a sequence (MoveTo, LineTo, ArcTo, BezierTo, SetPower,
SetFeedRate, JobStart, etc.).

CommandCategory — classifies commands as MOVING (changes tool position), STATE (changes machine
parameters), or MARKER (structural job boundaries).

SectionType — distinguishes between VECTOR_OUTLINE and RASTER_FILL sections when an Ops sequence is
split into logical portions.

RasterMode — identifies the power-application strategy for raster sections: VARIABLE_POWER
(per-pixel 8-bit), CONSTANT_POWER (uniform), DEPTH_MAP (stepped Z levels), or DITHER (binary
dithered mask).

## CommandCategory

Represents the category of a command: `MOVING`, `STATE`, or `MARKER`.

- **MOVING**: Commands that change the tool position (MoveTo, LineTo, ArcTo, etc.)
- **STATE**: Commands that change machine state (SetPower, SetCutSpeed, etc.)
- **MARKER**: Structural markers (JobStart/End, LayerStart/End, etc.)

### `name`

```python
name: str
```

The uppercase name of this category (`"MOVING"`, `"STATE"`, or `"MARKER"`).

### `value`

```python
value: int
```

The raw integer value of this category.

## CommandType

Enumeration of all command types in an Ops sequence.

Each constant represents a specific operation command such as `MOVE_TO`, `LINE_TO`, `ARC_TO`,
`SET_POWER`, etc.

### `name`

```python
name: str
```

The uppercase name of this command type (e.g. `"MOVE_TO"`, `"LINE_TO"`).

### `value`

```python
value: int
```

The raw integer value of this command type.

## RasterMode

Raster engrave mode: how power is applied during raster scanning.

- `VARIABLE_POWER`: per-pixel 8-bit power (from power-modulated image)
- `CONSTANT_POWER`: uniform power (from mask scans)
- `DEPTH_MAP`: stepped Z levels (from multi-pass image)
- `DITHER`: binary dithered mask scan at constant power

### `name`

```python
name: str
```

The uppercase name (e.g. `"VARIABLE_POWER"`, `"DITHER"`).

### `value`

```python
value: int
```

The raw integer value of this raster mode.

## SectionType

The type of an operations section: `VECTOR_OUTLINE` or `RASTER_FILL`.

Sections divide an Ops sequence into vector and raster portions.

### `name`

```python
name: str
```

The uppercase name (`"VECTOR_OUTLINE"` or `"RASTER_FILL"`).

### `value`

```python
value: int
```

The raw integer value of this section type.

## StateBlock

A state block within an Ops section — delimits parameter-regime changes.

Produced by **OpsSection.state_blocks**.

### `content_indices`

```python
content_indices: list[int]
```

Indices of the content commands belonging to this block.

### `marker_indices`

```python
marker_indices: list[int]
```

Indices of the marker commands (start/end) for this block.

### `name`

```python
name: Optional[str]
```

Optional block name.

### `content()`

```python
content(ops: ops.Ops) -> ops.Ops
```

Extract the content commands of this block from an Ops sequence.

| Parameter    | Type      | Description                                          |
| ------------ | --------- | ---------------------------------------------------- |
| `ops`        | `ops.Ops` | The Ops sequence containing this block.              |
| _Returns_    | `ops.Ops` | A new Ops containing only the content of this block. |
| _Complexity_ |           | O(n) time, O(n) space                                |

## Functions

### `category()`

```python
category(ct: CommandType) -> CommandCategory
```

Get the category of a command type.

| Parameter    | Type              | Description                           |
| ------------ | ----------------- | ------------------------------------- |
| `ct`         | `CommandType`     | `CommandType` variant.                |
| _Returns_    | `CommandCategory` | `CommandCategory` for the given type. |
| _Complexity_ |                   | O(1)                                  |
