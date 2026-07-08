---
title: raygeo.ops.assembly.slot
sidebar_label: raygeo.ops.assembly.slot
---

## Functions

### `generate_slot()`

```python
generate_slot(
    carrier: Sequence[tuple[float, float]],
    tool_radius: float,
    target_z: float,
    state: ops.state.State | None = None,
) -> ops.assembly.result.AssemblyResult
```

Generate a back-and-forth slot clearing path along a carrier.

Produces a forward pass then a backward pass along the carrier, both at constant *target_z*. The
cleared polygon is the carrier swept by *tool_radius* (Minkowski sum).

| Parameter     | Type                                 | Description                                      |
| ------------- | ------------------------------------ | ------------------------------------------------ |
| `carrier`     | `Sequence[tuple[float, float]]`      | `(x, y)` waypoints (currently 2-point segment).  |
| `tool_radius` | `float`                              | Tool radius in mm.                               |
| `target_z`    | `float`                              | Cutting Z height.                                |
| `state`       | `ops.state.State &#124; None = None` | Optional machine state to apply before the path. |
| _Returns_     | `ops.assembly.result.AssemblyResult` | An **AssemblyResult** with the slot path.        |

![3D back-and-forth slot path through a 40×7 mm slot. The carrier is derived by  (Step 9a) from the slot polygon and the bottom entry edge;  then emits a forward pass (entry side → far side, blue) immediately followed by a backward pass (far side → entry side, red) at constant target_z=-3. The dashed black line is the carrier; the solid black outline is the slot polygon; the navy rings are the tool-radius envelopes at the carrier endpoints. No trochoid — slotting is a linear constant-Z operation.](images/ops-assembly-slot-slot-3d.png)

*3D back-and-forth slot path through a 40×7 mm slot. The carrier is derived by `find_slot_path`
(Step 9a) from the slot polygon and the bottom entry edge; `generate_slot` then emits a forward pass
(entry side → far side, blue) immediately followed by a backward pass (far side → entry side, red)
at constant target_z=-3. The dashed black line is the carrier; the solid black outline is the slot
polygon; the navy rings are the tool-radius envelopes at the carrier endpoints. No trochoid —
slotting is a linear constant-Z operation.*
