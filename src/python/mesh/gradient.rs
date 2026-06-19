use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::mesh::gradient as rust_gradient;

use super::types::TriangleMesh;

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.mesh.types

    def compute_gradient_field(
        mesh: types.TriangleMesh,
        u_field: collections.abc.Sequence[float],
    ) -> collections.abc.Sequence[tuple[float, float]]:
        """Compute the gradient of the scalar field on each triangle.

        Given the solution u to the Laplace equation, computes ∇u = (∂u/∂x, ∂u/∂y)
        in the interior of each triangle of the mesh (piecewise constant).

        :param mesh: TriangleMesh with the same vertex count as u_field.
        :param u_field: Scalar field values, one per vertex.
        :returns: List of (gx, gy) pairs, one per triangle in mesh order.
        """
"#,
    module = "raygeo.mesh.gradient"
)]
#[pyfunction(name = "compute_gradient_field")]
#[pyo3(signature = (mesh, u_field))]
fn compute_gradient_field_py(
    mesh: &TriangleMesh,
    u_field: Vec<f64>,
) -> PyResult<Vec<(f64, f64)>> {
    let grad = rust_gradient::compute_gradient_field(&mesh.inner, &u_field)
        .map_err(pyo3::exceptions::PyValueError::new_err)?;
    Ok(grad.into_iter().map(|g| (g[0], g[1])).collect())
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "gradient")?;
    m.setattr("__doc__", "Gradient field computation on triangle meshes.")?;
    m.add_function(pyo3::wrap_pyfunction!(
        compute_gradient_field_py,
        m.clone()
    )?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
