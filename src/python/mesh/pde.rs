use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::mesh::pde;
use crate::types::Point;

use super::types::TriangleMesh;

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.mesh.types

    def trace_spiral(
        mesh: types.TriangleMesh,
        u_field: collections.abc.Sequence[float],
        step_over: float,
        start_point: tuple[float, float] | None = None,
    ) -> collections.abc.Sequence[tuple[float, float, float]]:
        """Trace a spiral toolpath from inner to outer boundary.

        Uses the piecewise-constant gradient of the scalar field u to
        trace a smooth spiral that morphs from the inner boundary (u=0)
        to the outer boundary (u=1) without self-intersections.

        :param mesh: TriangleMesh with boundary tags from build_triangle_mesh.
        :param u_field: Scalar field from solve_laplace, one value per vertex.
        :param step_over: Desired radial step-over distance between spiral turns.
        :param start_point: Optional explicit start point (x,y).
        :returns: List of (x, y, z) points forming the spiral polyline.
        :complexity: O(n * s) where n = triangles, s = spiral steps
        """
"#,
    module = "raygeo.mesh.pde"
)]
#[pyfunction(name = "trace_spiral")]
#[pyo3(signature = (mesh, u_field, step_over, start_point = None))]
fn trace_spiral_py(
    mesh: &TriangleMesh,
    u_field: Vec<f64>,
    step_over: f64,
    start_point: Option<(f64, f64)>,
) -> PyResult<Vec<(f64, f64, f64)>> {
    let start = start_point.map(|(x, y)| Point::new(x, y));
    let path = pde::trace_spiral(&mesh.inner, &u_field, step_over, start)
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
    Ok(path.into_iter().map(|p| (p.x, p.y, p.z)).collect())
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "pde")?;
    m.setattr(
        "__doc__",
        "PDE-based spiral toolpath tracing on triangle meshes.",
    )?;
    m.add_function(pyo3::wrap_pyfunction!(trace_spiral_py, m.clone())?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
