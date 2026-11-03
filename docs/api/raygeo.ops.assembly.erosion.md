---
title: raygeo.ops.assembly.erosion
sidebar_label: raygeo.ops.assembly.erosion
---

## ErosionSpec

Parameters for the `erosion` assembler.

### `area_tolerance`

```python
area_tolerance: float
```

Leave the pocket alone when the actionable residual totals less than this area (mm²).

### `safe_z`

```python
safe_z: float
```

Travel height for the retract between carriers.

### `target_z`

```python
target_z: float
```

### `tool_radius`

```python
tool_radius: float
```

## Functions

### `generate_erosion()`

```python
generate_erosion(
    part: ops.part.Part,
    tool_radius: float = 3,
    target_z: float = -5,
    safe_z: float = 2,
    area_tolerance: float = 1,
    state: ops.state.State | None = None,
) -> ops.assembly.AssemblyResult
```

Erode the residual clumps the clearing pass left behind.

Finds the uncut material inside the tool-centre envelope (a phantom-scale inset drops the
sweep-approximation film), slots each clump along a spine carrier at *target_z*, and records the cut
footprint in the cleared area. Emits no ops when the residual totals less than *area_tolerance*.

| Parameter        | Type                                 | Description                                                                                                                   |
| ---------------- | ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------- |
| `part`           | `ops.part.Part`                      | The part whose `cleared` field tracks accumulated workpiece state and whose geometry defines the pocket boundary and islands. |
| `tool_radius`    | `float = 3`                          | Tool radius in mm (default 3.0).                                                                                              |
| `target_z`       | `float = -5`                         | Cutting Z height (default -5.0).                                                                                              |
| `safe_z`         | `float = 2`                          | Travel height for the retract between carriers (default 2.0).                                                                 |
| `area_tolerance` | `float = 1`                          | Skip when the residual is below this area in mm² (default 1.0).                                                               |
| `state`          | `ops.state.State &#124; None = None` | Optional machine state to apply before the path.                                                                              |
| _Returns_        | `ops.assembly.AssemblyResult`        | An **AssemblyResult** with the erosion path (empty when there is nothing to erode).                                           |
