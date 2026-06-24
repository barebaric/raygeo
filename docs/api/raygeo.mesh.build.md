---
title: raygeo.mesh.build
sidebar_label: raygeo.mesh.build
---

![Uniform mesh (top) and Laplace gradient field (bottom) from build_uniform_mesh.](images/mesh-build-uniform.png)

*Uniform mesh (top) and Laplace gradient field (bottom) from build_uniform_mesh.*

## Functions

### `build_triangle_mesh()`

```python
build_triangle_mesh(
    outer: Sequence[tuple[float, float]],
    holes: Sequence[Sequence[tuple[float, float]]] = (),
    tool_radius: float = 0,
    min_angle: float = 20,
) -> types.TriangleMesh
```

Build a constrained Delaunay triangle mesh from polygon boundaries.

| Parameter     | Type                                           | Description                                            |
| ------------- | ---------------------------------------------- | ------------------------------------------------------ |
| `outer`       | `Sequence[tuple[float, float]]`                | Outer boundary polygon vertices as (x, y) tuples.      |
| `holes`       | `Sequence[Sequence[tuple[float, float]]] = ()` | Sequence of hole/island polygons.                      |
| `tool_radius` | `float = 0`                                    | Tool radius for offsetting the outer boundary inwards. |
| `min_angle`   | `float = 20`                                   | Minimum triangle angle for Steiner point refinement.   |
| _Returns_     | `types.TriangleMesh`                           | TriangleMesh with boundary tags.                       |
| _Complexity_  |                                                | O(n log n) where n = number of Steiner points          |

![CDT triangulation of a square pocket with centred hole](images/mesh-build-triangulation.png)

*CDT triangulation of a square pocket with centred hole*

![CDT triangulation of an L-shaped pocket](images/mesh-build-l-shape.png)

*CDT triangulation of an L-shaped pocket*

![CDT triangulation of a square pocket with multiple islands](images/mesh-build-multi-island.png)

*CDT triangulation of a square pocket with multiple islands*

### `build_uniform_mesh()`

```python
build_uniform_mesh(
    outer: Sequence[tuple[float, float]],
    holes: Sequence[Sequence[tuple[float, float]]] = (),
    tool_radius: float = 0,
    target_edge_len: float = 1,
) -> types.TriangleMesh
```

Build a triangle mesh with approximately uniform edge length.

Computes the Steiner point density needed to achieve *target_edge_len* and delegates to
`build_triangle_mesh`.

| Parameter         | Type                                           | Description                                   |
| ----------------- | ---------------------------------------------- | --------------------------------------------- |
| `outer`           | `Sequence[tuple[float, float]]`                | Outer boundary polygon.                       |
| `holes`           | `Sequence[Sequence[tuple[float, float]]] = ()` | List of hole/island polygons.                 |
| `tool_radius`     | `float = 0`                                    | Offsets outer boundary inward.                |
| `target_edge_len` | `float = 1`                                    | Desired edge length.                          |
| _Returns_         | `types.TriangleMesh`                           | TriangleMesh with uniform-sized elements.     |
| _Complexity_      |                                                | O(n log n) where n = number of Steiner points |
