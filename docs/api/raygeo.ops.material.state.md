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

### `max_power_watts`

```python
max_power_watts: float
```

Optical output power in watts at full power of the laser that produced the surface-map fluence.

### `profile`

```python
profile: str
```

Which profile produced this state (`"prismatic"` or `"cylindrical"`).

### `provenance`

```python
provenance: list[str]
```

Sorted unique source keys whose effects were applied.

### `surface_map`

```python
surface_map: Optional[compressed_array.CompressedArray]
```

Per-pixel maximum laser fluence (F32, J/cm²), or `None` when no raster effects contributed.

### `void_polygons`

```python
void_polygons: list[list[tuple[float, float]]]
```

Regions removed through the full stock thickness (world mm).

### `wavelength_nm`

```python
wavelength_nm: float
```

Emission wavelength in nm of the laser that produced the surface-map fluence. 0 means unconfigured;
the renderer falls back to full absorption.
