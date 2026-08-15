---
title: raygeo.mesh.types
sidebar_label: raygeo.mesh.types
---

## PrismMesh

GPU-ready prism mesh returned by `build_prism_mesh`.

Buffers are per-face-vertex (not shared). All getters return fresh numpy arrays: float32 positions
(N, 3), float32 normals (N, 3), float32 UVs (N, 2) and flat uint32 triangle indices (3T,).

### `indices`

```python
indices: Any
```

Flat triangle vertex indices as a uint32 array of shape (3T,).

### `normals`

```python
normals: Any
```

Flat XYZ vertex normals as a float32 array of shape (N, 3).

### `positions`

```python
positions: Any
```

Flat XYZ vertex positions as a float32 array of shape (N, 3).

### `uvs`

```python
uvs: Any
```

Flat XY UV coordinates as a float32 array of shape (N, 2).

## TriangleMesh

### `adjacency`

```python
adjacency: list[int]
```

### `boundary_tags`

```python
boundary_tags: list[str]
```

### `triangles`

```python
triangles: list[tuple[int, int, int]]
```

### `vertices`

```python
vertices: list[tuple[float, float]]
```
