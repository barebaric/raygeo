use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::mesh::build as rust_build;
use crate::types::Point;

use super::types::TriangleMesh;

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.mesh.types

    def build_triangle_mesh(
        outer: collections.abc.Sequence[tuple[float, float]],
        holes: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = (),
        tool_radius: float = 0.0,
        min_angle: float = 20.0,
    ) -> types.TriangleMesh:
        """Build a constrained Delaunay triangle mesh from polygon boundaries.

        :param outer: Outer boundary polygon vertices as (x, y) tuples.
        :param holes: Sequence of hole/island polygons.
        :param tool_radius: Tool radius for offsetting the outer boundary inwards.
        :param min_angle: Minimum triangle angle for Steiner point refinement.
        :returns: TriangleMesh with boundary tags.
        :complexity: O(n log n) where n = number of Steiner points
        """
"#,
    module = "raygeo.mesh.build"
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
    let mesh = rust_build::build_triangle_mesh(
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

    def build_uniform_mesh(
        outer: collections.abc.Sequence[tuple[float, float]],
        holes: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = (),
        tool_radius: float = 0.0,
        target_edge_len: float = 1.0,
    ) -> types.TriangleMesh:
        """Build a triangle mesh with approximately uniform edge length.

        Computes the Steiner point density needed to achieve
        *target_edge_len* and delegates to ``build_triangle_mesh``.

        :param outer: Outer boundary polygon.
        :param holes: List of hole/island polygons.
        :param tool_radius: Offsets outer boundary inward.
        :param target_edge_len: Desired edge length.
        :returns: TriangleMesh with uniform-sized elements.
        :complexity: O(n log n) where n = number of Steiner points
        """
"#,
    module = "raygeo.mesh.build"
)]
#[pyfunction(name = "build_uniform_mesh")]
#[pyo3(signature = (outer, holes = vec![], tool_radius = 0.0, target_edge_len = 1.0))]
fn build_uniform_mesh_py(
    outer: Vec<(f64, f64)>,
    holes: Vec<Vec<(f64, f64)>>,
    tool_radius: f64,
    target_edge_len: f64,
) -> PyResult<TriangleMesh> {
    let outer_pts: Vec<Point> =
        outer.iter().map(|p| Point::new(p.0, p.1)).collect();
    let hole_polys: Vec<Vec<Point>> = holes
        .iter()
        .map(|h| h.iter().map(|p| Point::new(p.0, p.1)).collect())
        .collect();
    let mesh = rust_build::build_uniform_mesh(
        &outer_pts,
        &hole_polys,
        tool_radius,
        target_edge_len,
    )
    .map_err(pyo3::exceptions::PyValueError::new_err)?;
    Ok(TriangleMesh { inner: mesh })
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "build")?;
    m.setattr("__doc__", "Constrained Delaunay triangulation.")?;
    m.add_function(pyo3::wrap_pyfunction!(build_triangle_mesh_py, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(build_uniform_mesh_py, m.clone())?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
