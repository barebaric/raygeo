---
title: raygeo.mesh.solid
sidebar_label: raygeo.mesh.solid
---

Plain-data closed-manifold triangle meshes for solid interchange.

## SolidMesh

A closed-manifold triangle mesh in millimetres.

The interchange format for solid geometry: f64 positions plus triangle indices and nothing else.

### `positions`

```python
positions: list[tuple[float, float, float]]
```

Vertex positions (world mm).

### `triangles`

```python
triangles: list[tuple[int, int, int]]
```

Triangles as indices into `positions`.
