---
title: raygeo.ops.types
sidebar_label: raygeo.ops.types
sidebar_position: 36
---

Core enumerations for the Ops command system.

CommandType — identifies each command in a sequence (MoveTo, LineTo, ArcTo, BezierTo, SetPower,
SetSpeed, JobStart, etc.).

CommandCategory — classifies commands as MOVING (changes tool position), STATE (changes machine
parameters), or MARKER (structural job boundaries).

SectionType — distinguishes between VECTOR_OUTLINE and RASTER_FILL sections when an Ops sequence is
split into logical portions.

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

## Functions

### `category()`

```python
category(ct: CommandType) -> CommandCategory
```

Get the category of a command type.

| Parameter    | Type              | Description |
| ------------ | ----------------- | ----------- |
| `ct`         | `CommandType`     |             |
| _Returns_    | `CommandCategory` |             |
| _Complexity_ |                   | O(1)        |
