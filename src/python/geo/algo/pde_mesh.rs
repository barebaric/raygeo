pyo3_stub_gen::module_doc!(
    "raygeo.geo.algo.pde_mesh",
    "{}",
    MODULE_DOC_PDE_MESH
);

pub(crate) const MODULE_DOC_PDE_MESH: &str = "\
PDE mesh generation and Laplace solving for HSM toolpath planning.

Provides a TriangleMesh class for constrained Delaunay triangulation of
2D polygon domains, and a solve_laplace function that solves the Laplace
equation Δu=0 using linear finite elements.

Typical usage:
    from raygeo.geo.algo.pde_mesh import build_triangle_mesh, solve_laplace
    mesh = build_triangle_mesh(outer, holes, tool_radius=0.0, min_angle=20.0)
    u = solve_laplace(mesh)
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::geo::algo::pde_mesh::{
    build_triangle_mesh as rust_build_triangle_mesh,
    solve_laplace as rust_solve_laplace, BoundaryTag, TriangleMesh as RustMesh,
};
use crate::types::Point;

#[gen_stub_pyclass]
#[pyclass(module = "raygeo.geo.algo.pde_mesh", name = "TriangleMesh")]
pub struct TriangleMesh {
    inner: RustMesh,
}

#[gen_stub_pymethods]
#[pymethods]
impl TriangleMesh {
    #[new]
    fn new() -> Self {
        TriangleMesh {
            inner: RustMesh {
                vertices: Vec::new(),
                triangles: Vec::new(),
                adjacency: Vec::new(),
                boundary_tags: Vec::new(),
            },
        }
    }

    #[getter]
    fn vertices(&self) -> Vec<(f64, f64)> {
        self.inner.vertices.iter().map(|p| (p.x, p.y)).collect()
    }

    #[getter]
    fn triangles(&self) -> Vec<(usize, usize, usize)> {
        self.inner
            .triangles
            .iter()
            .map(|t| (t[0], t[1], t[2]))
            .collect()
    }

    #[getter]
    fn adjacency(&self) -> Vec<isize> {
        self.inner.adjacency.clone()
    }

    #[getter]
    fn boundary_tags(&self) -> Vec<String> {
        self.inner
            .boundary_tags
            .iter()
            .map(|t| match t {
                BoundaryTag::Outer => "outer".to_string(),
                BoundaryTag::Inner => "inner".to_string(),
                BoundaryTag::None => "free".to_string(),
            })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "TriangleMesh(vertices={}, triangles={})",
            self.inner.vertices.len(),
            self.inner.triangles.len()
        )
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import raygeo.geo.types

    def build_triangle_mesh(
        outer: collections.abc.Sequence[types.Point],
        holes: collections.abc.Sequence[collections.abc.Sequence[types.Point]] = (),
        tool_radius: float = 0.0,
        min_angle: float = 20.0,
    ) -> TriangleMesh:
        """Build a constrained Delaunay triangle mesh from polygon boundaries.

        :param outer: Outer boundary polygon vertices as (x, y) tuples.
        :param holes: Sequence of hole/island polygons.
        :param tool_radius: Tool radius for offsetting the outer boundary inwards.
        :param min_angle: Minimum triangle angle for Steiner point refinement.
        :returns: TriangleMesh with boundary tags.
        :complexity: O(n log n) time, O(n) space
        """
"#,
    module = "raygeo.geo.algo.pde_mesh"
)]
#[pyfunction(name = "build_triangle_mesh")]
#[pyo3(signature = (outer, holes = vec![], tool_radius = 0.0, min_angle = 20.0))]
fn build_triangle_mesh_py(
    outer: Vec<(f64, f64)>,
    holes: Vec<Vec<(f64, f64)>>,
    tool_radius: f64,
    min_angle: f64,
) -> PyResult<TriangleMesh> {
    let outer_pts: Vec<Point> =
        outer.iter().map(|p| Point::new(p.0, p.1)).collect();
    let hole_polys: Vec<Vec<Point>> = holes
        .iter()
        .map(|h| h.iter().map(|p| Point::new(p.0, p.1)).collect())
        .collect();
    let mesh = rust_build_triangle_mesh(
        &outer_pts,
        &hole_polys,
        tool_radius,
        min_angle,
    )
    .map_err(pyo3::exceptions::PyValueError::new_err)?;
    Ok(TriangleMesh { inner: mesh })
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def solve_laplace(
        mesh: TriangleMesh,
        max_iter: int = 1000,
        tolerance: float = 1e-8,
    ) -> list[float]:
        """Solve the Laplace equation Δu=0 on a triangle mesh.

        Returns a scalar field with one value per vertex. Outer boundary
        vertices are fixed to u=1.0 and inner boundary vertices to u=0.0.

        :param mesh: TriangleMesh with boundary tags.
        :param max_iter: Maximum conjugate gradient iterations.
        :param tolerance: Convergence tolerance for CG residual.
        :returns: List of scalar u values, one per vertex.
        :complexity: O(k * n) time where k is the number of CG iterations
        """
"#,
    module = "raygeo.geo.algo.pde_mesh"
)]
#[pyfunction(name = "solve_laplace")]
#[pyo3(signature = (mesh, max_iter = 1000, tolerance = 1e-8))]
fn solve_laplace_py(
    mesh: &TriangleMesh,
    max_iter: usize,
    tolerance: f64,
) -> PyResult<Vec<f64>> {
    rust_solve_laplace(&mesh.inner, Some(max_iter), Some(tolerance))
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)
}

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "pde_mesh")?;
    m.setattr("__doc__", MODULE_DOC_PDE_MESH)?;

    m.add_class::<TriangleMesh>()?;
    register_functions!(m, build_triangle_mesh_py, solve_laplace_py);

    algo_mod.add_submodule(&m)?;
    Ok(())
}
