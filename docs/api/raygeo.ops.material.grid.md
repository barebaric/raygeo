---
title: raygeo.ops.material.grid
sidebar_label: raygeo.ops.material.grid
---

Burn-grid helpers: per-vertex power UVs for a stock mesh.

## Functions

### `compute_power_uvs()`

```python
compute_power_uvs(
    positions: numpy.NDArray[numpy.float32],
    origin_mm: tuple[float, float],
    px_per_mm: tuple[float, float],
    size_px: tuple[int, int],
) -> numpy.NDArray[numpy.float32]
```

Map vertex positions onto a burn power grid as UVs.

| Parameter   | Type                           | Description                                              |
| ----------- | ------------------------------ | -------------------------------------------------------- |
| `positions` | `numpy.NDArray[numpy.float32]` | Flat mesh vertex positions, shape `(N, 3)`.              |
| `origin_mm` | `tuple[float, float]`          | World-mm coordinate of the grid's `(0, 0)` pixel corner. |
| `px_per_mm` | `tuple[float, float]`          | Grid density in pixels per millimetre.                   |
| `size_px`   | `tuple[int, int]`              | Grid size in pixels `(width, height)`.                   |
| _Returns_   | `numpy.NDArray[numpy.float32]` | `(N, 2)` power UVs index-aligned with *positions*.       |
