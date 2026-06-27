---
title: raygeo.ops.cut.crescent
sidebar_label: raygeo.ops.cut.crescent
---

## Functions

### `cut_area()`

```python
cut_area(
    c1: tuple[float, float],
    c2: tuple[float, float],
    radius: float,
    fragments: Sequence[Sequence[tuple[float, float]]],
    valid_area: Sequence[Sequence[tuple[float, float]]],
) -> tuple[float, float]
```

Area of `disk(c2) − disk(c1) − fragments`, intersected with *valid_area*.

Returns `(total, left)` where *left* is the portion on the left side of the step vector `c1 → c2`.

| Parameter    | Type                                      | Description                           |
| ------------ | ----------------------------------------- | ------------------------------------- |
| `c1`         | `tuple[float, float]`                     | Previous centre `(x, y)`.             |
| `c2`         | `tuple[float, float]`                     | Next centre `(x, y)`.                 |
| `radius`     | `float`                                   | Disk radius (mm).                     |
| `fragments`  | `Sequence[Sequence[tuple[float, float]]]` | List of polygons (cleared fragments). |
| `valid_area` | `Sequence[Sequence[tuple[float, float]]]` | Valid region polygons (intersection). |
| _Returns_    | `tuple[float, float]`                     | `(total_area, left_area)` (mm²).      |

![Disk increment (in red) produced by stepping a disk from C1 to C2. Left panel shows the full increment; right panel shows the reduction when a cleared fragment (gray) occupies part of the increment.](images/ops-cut-crescent-disk-increment.png)

*Disk increment (in red) produced by stepping a disk from C1 to C2. Left panel shows the full
increment; right panel shows the reduction when a cleared fragment (gray) occupies part of the
increment.*

![Crescent area as a function of step distance  compared with the analytical crescent formula. Right panel shows the left-side portion (half of total for the symmetric case).](images/ops-cut-crescent-crescent-area-vs-distance.png)

*Crescent area as a function of step distance `d` compared with the analytical crescent formula.
Right panel shows the left-side portion (half of total for the symmetric case).*

![2D heatmap of  as the  centre is moved across a grid around . The dashed circle shows ; the area is zero when  coincides with  and maximal at intermediate distances.](images/ops-cut-crescent-crescent-heatmap-2d.png)

*2D heatmap of `cut_area` as the `c2` centre is moved across a grid around `c1`. The dashed circle
shows `Disk(c1)`; the area is zero when `c2` coincides with `c1` and maximal at intermediate
distances.*

![Effect of a vertical-wall fragment sweeping across the crescent. Left panel shows the geometry at one wall position; the right panel plots total and left area vs wall , demonstrating smooth continuity.](images/ops-cut-crescent-crescent-fragment-sweep.png)

*Effect of a vertical-wall fragment sweeping across the crescent. Left panel shows the geometry at
one wall position; the right panel plots total and left area vs wall `x`, demonstrating smooth
continuity.*

![Crescent clipped to different  polygons: no clip (full), a left-half window, and a window around the crescent tip. The faint gray shape is the unclipped crescent reference.](images/ops-cut-crescent-crescent-valid-area-clip.png)

*Crescent clipped to different `valid_area` polygons: no clip (full), a left-half window, and a
window around the crescent tip. The faint gray shape is the unclipped crescent reference.*
