---
title: raygeo.ops.cut
sidebar_label: raygeo.ops.cut
---

Cleared-area tracker for material removal.

Maintains a union of swept-disk polygons and provides a spatial-indexed windowed query for efficient
engagement computation.

## Part

Unified workpiece description for motion assembly.

Carries geometry, physical metadata, and a `ClearedArea` tracking what has already been cut.
Assemblers mutate the cleared area as they work.

### `cleared`

```python
cleared: cleared_area.ClearedArea
```

Accumulated cleared-area state — what has been cut so far.

Read-only snapshot. Assemblers mutate this internally; use it after an assembler returns to inspect
remaining material, fragments, etc.

### `geometry`

```python
geometry: Optional[geo.Geometry]
```

Vector geometry (the outline(s) of the part), if any.

Returns `None` if no geometry was provided at construction time.

### `pixels_per_mm`

```python
pixels_per_mm: Optional[tuple[float, float]]
```

Pixel density `(x, y)` in px/mm, if set.

### `size_mm`

```python
size_mm: tuple[float, float]
```

Physical size `(width, height)` in millimetres.

### `from_polygons()`

```python
from_polygons(
    boundary: Sequence[tuple[float, float]],
    islands: Optional[Sequence[Sequence[tuple[float, float]]]] = None,
    size_mm: tuple[float, float] = (0.0, 0.0),
    initial: Optional[Sequence[Sequence[tuple[float, float]]]] = None,
) -> Part
```

Build a Part from a boundary polygon and optional islands.

| Parameter  | Type                                                       | Description                                                                                                                                                                 |
| ---------- | ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `boundary` | `Sequence[tuple[float, float]]`                            | Outer boundary as `[(x, y), ...]`.                                                                                                                                          |
| `islands`  | `Optional[Sequence[Sequence[tuple[float, float]]]] = None` | List of island polygons, each `[(x, y), ...]` (default `[]`).                                                                                                               |
| `size_mm`  | `tuple[float, float] = (0.0, 0.0)`                         | Physical size `(width, height)` in mm (default `(0, 0)`).                                                                                                                   |
| `initial`  | `Optional[Sequence[Sequence[tuple[float, float]]]] = None` | Optional pre-seeded cleared polygons (e.g. a seed circle for adaptive clearing). When provided, the part's cleared area starts with these fragments instead of being empty. |
| _Returns_  | `Part`                                                     | A new `Part` with the geometry constructed from the given polygons.                                                                                                         |

### `has_geometry()`

```python
has_geometry() -> bool
```

True if this Part has geometry.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |
