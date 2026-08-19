---
title: raygeo.ops.material.state
sidebar_label: raygeo.ops.material.state
---

The folded state of one stock: an immutable snapshot.

## MaterialState

The folded state of one stock: an immutable snapshot.

### `depth_field`

```python
depth_field: Optional[compressed_array.CompressedArray]
```

Removal-depth heightmap in mm, or `None` until depth folding lands.

### `escalation`

```python
escalation: Optional[str]
```

First invariant violation encountered (`"top_open_violation"` or `"solid_profile_required"`), or
`None`.

### `grid`

```python
grid: Optional[spec.GridSpec]
```

Grid shared by the raster outputs.

### `profile`

```python
profile: str
```

Which profile produced this state (`"prismatic"`).

### `provenance`

```python
provenance: list[str]
```

Sorted unique source keys whose effects were applied.

### `surface_map`

```python
surface_map: Optional[compressed_array.CompressedArray]
```

Per-pixel maximum laser power (R8), or `None` when no raster effects contributed.

### `void_polygons`

```python
void_polygons: list[list[tuple[float, float]]]
```

Regions removed through the full stock thickness (world mm).
