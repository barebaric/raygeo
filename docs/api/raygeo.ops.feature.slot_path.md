---
title: raygeo.ops.feature.slot_path
sidebar_label: raygeo.ops.feature.slot_path
---

## Functions

### `find_slot_path()`

```python
find_slot_path(
    slot_polygon: Sequence[tuple[float, float]],
    entry_edges: Sequence[int],
    entry_point: tuple[float, float],
    tool_radius: float = 3,
) -> list[tuple[float, float]] | None
```

Find the 2D carrier axis for a slot clearing operation.

Returns a `[(x1, y1), (x2, y2)]` list representing the longitudinal axis of the slot, with the first
point on the entry side. Both points are valid tool centres that fit within the eroded slot.

| Parameter      | Type                                    | Description                                                 |
| -------------- | --------------------------------------- | ----------------------------------------------------------- |
| `slot_polygon` | `Sequence[tuple[float, float]]`         | Slot polygon as `[(x, y), ...]`.                            |
| `entry_edges`  | `Sequence[int]`                         | Indices of entry edges into the slot polygon.               |
| `entry_point`  | `tuple[float, float]`                   | Entry point `(x, y)` (used only for side determination).    |
| `tool_radius`  | `float = 3`                             | Tool radius in mm (default 3.0).                            |
| _Returns_      | `list[tuple[float, float]] &#124; None` | `[(x1, y1), (x2, y2)]` or `None` if the slot is too narrow. |

![The slot carrier returned by  on four scenarios (2x2 layout). Top-left: a horizontal 40x7 mm slot, with the navy carrier centred on the eroded region along the slot's long axis. Top-right: a vertical 7x40 mm slot where the long axis flips to y. Bottom-left: a too-narrow 30x5 mm slot with tool_radius=3 returns None (the eroded region is empty). Bottom-right: a sinusoidal S-slot (6 mm corridor, r=2) where the disk-probe snake walk produces a carrier that follows the S-curve smoothly from bottom to top without crossing empty space or leaving the eroded region. The red cross marks the requested entry_point and the red bar highlights the slot's entry edge.](images/ops-feature-slot-path-slot-path-scenarios.png)

*The slot carrier returned by `find_slot_path` on four scenarios (2x2 layout). Top-left: a
horizontal 40x7 mm slot, with the navy carrier centred on the eroded region along the slot's long
axis. Top-right: a vertical 7x40 mm slot where the long axis flips to y. Bottom-left: a too-narrow
30x5 mm slot with tool_radius=3 returns None (the eroded region is empty). Bottom-right: a
sinusoidal S-slot (6 mm corridor, r=2) where the disk-probe snake walk produces a carrier that
follows the S-curve smoothly from bottom to top without crossing empty space or leaving the eroded
region. The red cross marks the requested entry_point and the red bar highlights the slot's entry
edge.*
