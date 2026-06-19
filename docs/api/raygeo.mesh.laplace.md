---
title: raygeo.mesh.laplace
sidebar_label: raygeo.mesh.laplace
sidebar_position: 45
---

## Functions

### `solve_laplace()`

```python
solve_laplace(
    mesh: types.TriangleMesh,
    max_iter: int = 1000,
    tolerance: float = 1e-08,
) -> Sequence[float]
```

Solve the Laplace equation Δu=0 on a triangle mesh.

Returns a scalar field with one value per vertex. Outer boundary vertices are fixed to u=1.0 and
inner boundary vertices to u=0.0.

| Parameter   | Type                 | Description                              |
| ----------- | -------------------- | ---------------------------------------- |
| `mesh`      | `types.TriangleMesh` | TriangleMesh with boundary tags.         |
| `max_iter`  | `int = 1000`         | Maximum conjugate gradient iterations.   |
| `tolerance` | `float = 1e-08`      | Convergence tolerance for CG residual.   |
| _Returns_   | `Sequence[float]`    | List of scalar u values, one per vertex. |

![Stiffness matrix edge weights on the mesh — line thickness ∝ |Kᵢⱼ|](images/mesh-laplace-stiffness-spy.png)

_Stiffness matrix edge weights on the mesh — line thickness ∝ |Kᵢⱼ|_

![Laplace solution on a multi-island domain — contour lines morph smoothly between four inner islands and the outer boundary](images/mesh-laplace-multi-island.png)

_Laplace solution on a multi-island domain — contour lines morph smoothly between four inner islands
and the outer boundary_

![Laplace solution — contours morph smoothly from hole to boundary](images/mesh-laplace-overview.png)

_Laplace solution — contours morph smoothly from hole to boundary_

![Laplace solution on an L-shaped domain](images/mesh-laplace-l-shape-solution.png)

_Laplace solution on an L-shaped domain_

### `solve_laplace_with_history()`

```python
solve_laplace_with_history(
    mesh: types.TriangleMesh,
    max_iter: int = 1000,
    tolerance: float = 1e-08,
) -> tuple[Sequence[float], Sequence[float]]
```

Solve the Laplace equation and return convergence history.

Identical to solve_laplace() but also returns the residual norm after each conjugate gradient
iteration for convergence analysis.

| Parameter   | Type                                      | Description                            |
| ----------- | ----------------------------------------- | -------------------------------------- |
| `mesh`      | `types.TriangleMesh`                      | TriangleMesh with boundary tags.       |
| `max_iter`  | `int = 1000`                              | Maximum conjugate gradient iterations. |
| `tolerance` | `float = 1e-08`                           | Convergence tolerance for CG residual. |
| _Returns_   | `tuple[Sequence[float], Sequence[float]]` | Tuple of (solution, residuals).        |

![Conjugate gradient convergence — residual norm per iteration](images/mesh-laplace-convergence.png)

_Conjugate gradient convergence — residual norm per iteration_
