---
title: raygeo.ops.assembly.frame
sidebar_label: raygeo.ops.assembly.frame
---

## FrameSpec

Parameters for the `frame` assembler.

Construct with `FrameSpec(offset_mm, cut_side, corner_radius)`. Wrap in an
**~raygeo.ops.assembly.Assembler** instance to drive the `Assembler` trait.

### `corner_radius`

```python
corner_radius: float
```

Corner rounding radius in mm (0 = sharp corners).

### `cut_side`

```python
cut_side: str
```

`"centerline"`, `"outside"`, or `"inside"`.

### `offset_mm`

```python
offset_mm: float
```

Total path offset distance in mm.

## Functions

### `frame()`

```python
frame(
    part: ops.part.Part,
    offset_mm: float = 0,
    cut_side: str = 'centerline',
    corner_radius: float = 0,
) -> ops.assembly.AssemblyResult
```

Generate a rectangular frame around the part boundary.

Creates a rectangle matching `part.size_mm`, computes the total offset from offset / cut-side,
applies it, and returns the frame as an **AssemblyResult**. When `corner_radius` is positive the
corners are rounded to that radius (clamped to what fits the frame).

**Raises:** `ValueError` — If the part has no size information.

| Parameter       | Type                          | Description                                                          |
| --------------- | ----------------------------- | -------------------------------------------------------------------- |
| `part`          | `ops.part.Part`               | The part whose size defines the frame.                               |
| `offset_mm`     | `float = 0`                   | Total path offset distance in mm (default 0.0).                      |
| `cut_side`      | `str = 'centerline'`          | `"centerline"`, `"outside"`, or `"inside"` (default `"centerline"`). |
| `corner_radius` | `float = 0`                   | Corner rounding radius in mm (default 0.0 = sharp corners).          |
| _Returns_       | `ops.assembly.AssemblyResult` | An **AssemblyResult** with the frame path.                           |
