pyo3_stub_gen::module_doc!(
    "raygeo.geo.algo.pde_spiral",
    "{}",
    MODULE_DOC_PDE_SPIRAL
);

pub(crate) const MODULE_DOC_PDE_SPIRAL: &str = "\
PDE-based spiral path tracing.

Given a triangle mesh with a Laplace solution (scalar field u),
traces a spiral path from the inner boundary (u=0) outward to the
outer boundary (u\u{2248}1) by following the piecewise-constant
vector field \u{2207}u\u{22a5} + \u{03b1}\u{2207}u.

Typical usage:
    from raygeo.geo.algo.pde_mesh import build_triangle_mesh, solve_laplace
    from raygeo.geo.algo.pde_spiral import trace_spiral
    mesh = build_triangle_mesh(outer, [hole])
    u = solve_laplace(mesh)
    path = trace_spiral(mesh, u, step_over=1.0)
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::algo::pde_spiral;

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.algo.pde_mesh

    def trace_spiral(
        mesh: pde_mesh.TriangleMesh,
        u_field: collections.abc.Sequence[float],
        step_over: float,
    ) -> list[tuple[float, float, float]]:
        """Trace a spiral toolpath from inner to outer boundary.

        Uses the piecewise-constant gradient of the scalar field u to
        trace a smooth spiral that morphs from the inner boundary (u=0)
        to the outer boundary (u=1) without self-intersections.

        :param mesh: TriangleMesh with boundary tags from build_triangle_mesh.
        :param u_field: Scalar field from solve_laplace, one value per vertex.
        :param step_over: Desired radial step-over distance between spiral
            turns. Larger values produce fewer, wider loops.
        :returns: List of (x, y, z) points forming the spiral polyline.
        :complexity: O(t * k) where t is the number of traversed triangles
        """
"#,
    module = "raygeo.geo.algo.pde_spiral"
)]
#[pyfunction(name = "trace_spiral")]
#[pyo3(signature = (mesh, u_field, step_over))]
fn trace_spiral_py(
    mesh: &super::pde_mesh::TriangleMesh,
    u_field: Vec<f64>,
    step_over: f64,
) -> PyResult<Vec<(f64, f64, f64)>> {
    let path = pde_spiral::trace_spiral(&mesh.inner, &u_field, step_over)
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
    Ok(path.into_iter().map(|p| (p.x, p.y, p.z)).collect())
}

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "pde_spiral")?;
    m.setattr("__doc__", MODULE_DOC_PDE_SPIRAL)?;

    register_functions!(m, trace_spiral_py);

    algo_mod.add_submodule(&m)?;
    Ok(())
}
