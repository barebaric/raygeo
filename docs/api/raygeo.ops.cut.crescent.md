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

![Disk increment (red): stepping C1 to C2; left full, right shows reduction with a cleared fragment](images/ops-cut-crescent-disk-increment.png)

*Disk increment (red): stepping C1 to C2; left full, right shows reduction with a cleared fragment*

![Crescent area vs step  compared to analytical formula; right shows left-side portion](images/ops-cut-crescent-crescent-area-vs-distance.png)

*Crescent area vs step `d` compared to analytical formula; right shows left-side portion*

![2D heatmap of  as c2 orbits c1; zero at coincidence, maximal at mid distances](images/ops-cut-crescent-crescent-heatmap-2d.png)

*2D heatmap of `cut_area` as c2 orbits c1; zero at coincidence, maximal at mid distances*

![Vertical-wall fragment sweeping crescent; left shows geometry, right plots area vs wall position](images/ops-cut-crescent-crescent-fragment-sweep.png)

*Vertical-wall fragment sweeping crescent; left shows geometry, right plots area vs wall position*

![Crescent clipped to : no clip, left-half window, tip window; faint gray = unclipped](images/ops-cut-crescent-crescent-valid-area-clip.png)

*Crescent clipped to `valid_area`: no clip, left-half window, tip window; faint gray = unclipped*
