---
title: raygeo.ops.transform.mesh_correction
sidebar_label: raygeo.ops.transform.mesh_correction
---

## MeshCorrectionSpec

A probed bed height map for **raygeo.ops.Ops.mesh_correction**.

Construct with `MeshCorrectionSpec(x0, y0, dx, dy, heights, z_offset)`; invalid grids raise
`ValueError` at construction.

### `dx`

```python
dx: float
```

Spacing between grid columns along X.

### `dy`

```python
dy: float
```

Spacing between grid rows along Y.

### `heights`

```python
heights: numpy.NDArray[numpy.float64]
```

The probed heights as a `(ny, nx)` float64 array.

### `x0`

```python
x0: float
```

X coordinate of the first grid column.

### `y0`

```python
y0: float
```

Y coordinate of the first grid row.

### `z_offset`

```python
z_offset: float
```

Constant Z added on top of every sampled height.
