---
title: raygeo.ops.assembly.polyline
sidebar_label: raygeo.ops.assembly.polyline
sidebar_position: 57
---

## Functions

### `polyline_to_ops()`

```python
polyline_to_ops(
    points: list[tuple[float, float, float]],
    move_first: bool = True,
) -> ops.Ops
```

Convert a 3-D polyline into an Ops command sequence.

When _move_first_ is `True` the first point is emitted as a MoveTo and subsequent points as LineTo.
When _move_first_ is `False` every point is emitted as a LineTo (useful for appending a polyline to
an in-progress cut).

| Parameter    | Type                               | Description                                  |
| ------------ | ---------------------------------- | -------------------------------------------- |
| `points`     | `list[tuple[float, float, float]]` | List of `(x, y, z)` tuples.                  |
| `move_first` | `bool = True`                      | Whether to emit the first point as a MoveTo. |
| _Returns_    | `ops.Ops`                          | An \*\*~raygeo.ops.Ops\*\* container.        |
| _Complexity_ |                                    | O(n) where n = number of points              |

![polyline_to_ops with move_first=True vs move_first=False](images/ops-assembly-polyline-to-ops.png)

_polyline_to_ops with move_first=True vs move_first=False_
