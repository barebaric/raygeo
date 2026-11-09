---
title: raygeo.ops.feature.hole
sidebar_label: raygeo.ops.feature.hole
---

## Functions

### `find_holes()`

```python
find_holes(
    islands: Sequence[Sequence[tuple[float, float]]],
    tolerance: float = 0.1,
) -> list[tuple[float, float, float]]
```

Find circular holes among island contours.

An island counts as a hole when every densified sample of its outline lies on a common circle within
`tolerance` (mm). Non-circular islands (bosses, slots, arbitrary pockets) are rejected.

| Parameter   | Type                                      | Description                                                                           |
| ----------- | ----------------------------------------- | ------------------------------------------------------------------------------------- |
| `islands`   | `Sequence[Sequence[tuple[float, float]]]` | Island contours as `[[(x, y), ...], ...]`.                                            |
| `tolerance` | `float = 0.1`                             | Maximum distance (mm) between any contour sample and the fitted circle (default 0.1). |
| _Returns_   | `list[tuple[float, float, float]]`        | `[(x, y, diameter), ...]` in the order the islands were passed.                       |
