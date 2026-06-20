use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::mesh::laplace as rust_laplace;

use super::types::TriangleMesh;

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.mesh.types

    def solve_laplace(
        mesh: types.TriangleMesh,
        max_iter: int = 1000,
        tolerance: float = 1e-8,
    ) -> collections.abc.Sequence[float]:
        """Solve the Laplace equation Δu=0 on a triangle mesh.

        Returns a scalar field with one value per vertex. Outer boundary
        vertices are fixed to u=1.0 and inner boundary vertices to u=0.0.

        :param mesh: TriangleMesh with boundary tags.
        :param max_iter: Maximum conjugate gradient iterations.
        :param tolerance: Convergence tolerance for CG residual.
        :returns: List of scalar u values, one per vertex.
        :complexity: O(i * n) where i = CG iterations, n = mesh vertices
        """
"#,
    module = "raygeo.mesh.laplace"
)]
#[pyfunction(name = "solve_laplace")]
#[pyo3(signature = (mesh, max_iter = 1000, tolerance = 1e-8))]
fn solve_laplace_py(
    mesh: &TriangleMesh,
    max_iter: usize,
    tolerance: f64,
) -> PyResult<Vec<f64>> {
    rust_laplace::solve_laplace(&mesh.inner, Some(max_iter), Some(tolerance))
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.mesh.types

    def solve_laplace_with_history(
        mesh: types.TriangleMesh,
        max_iter: int = 1000,
        tolerance: float = 1e-8,
    ) -> tuple[collections.abc.Sequence[float], collections.abc.Sequence[float]]:
        """Solve the Laplace equation and return convergence history.

        Identical to solve_laplace() but also returns the residual norm after
        each conjugate gradient iteration for convergence analysis.

        :param mesh: TriangleMesh with boundary tags.
        :param max_iter: Maximum conjugate gradient iterations.
        :param tolerance: Convergence tolerance for CG residual.
        :returns: Tuple of (solution, residuals).
        :complexity: O(i * n) where i = CG iterations, n = mesh vertices
        """
"#,
    module = "raygeo.mesh.laplace"
)]
#[pyfunction(name = "solve_laplace_with_history")]
#[pyo3(signature = (mesh, max_iter = 1000, tolerance = 1e-8))]
fn solve_laplace_with_history_py(
    mesh: &TriangleMesh,
    max_iter: usize,
    tolerance: f64,
) -> PyResult<(Vec<f64>, Vec<f64>)> {
    rust_laplace::solve_laplace_with_history(
        &mesh.inner,
        Some(max_iter),
        Some(tolerance),
    )
    .map_err(pyo3::exceptions::PyRuntimeError::new_err)
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "laplace")?;
    m.setattr("__doc__", "FEM Laplace equation solver on triangle meshes.")?;
    m.add_function(pyo3::wrap_pyfunction!(solve_laplace_py, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        solve_laplace_with_history_py,
        m.clone()
    )?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
