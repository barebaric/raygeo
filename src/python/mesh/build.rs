use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::types::Point;
use crate::mesh::build;

use super::types::{PrismMesh, TriangleMesh};

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
    let mesh = build::build_triangle_mesh(
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
    let mesh = build::build_uniform_mesh(
        &outer_pts,
        &hole_polys,
        tool_radius,
        target_edge_len,
    )
    .map_err(pyo3::exceptions::PyValueError::new_err)?;
    Ok(TriangleMesh { inner: mesh })
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.mesh.types

    def build_prism_mesh(
        outer: collections.abc.Sequence[tuple[float, float]],
        holes: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = (),
        thickness: float = 18.0,
        uv_scale: float = 300.0,
        z_top: float = 0.0,
    ) -> types.PrismMesh:
        """Build a closed prism mesh by extruding a polygon downward.

        The top face is triangulated with ear clipping (holes carved
        out) and placed at *z_top*; the bottom cap sits at
        ``z_top - thickness``; every boundary ring gets outward-facing
        side walls.  UVs are planar: ``uv = xy / uv_scale``.

        :param outer: Outer boundary polygon vertices as (x, y) tuples.
        :param holes: Sequence of hole/island polygons.
        :param thickness: Extrusion depth below *z_top*.
        :param uv_scale: World units per UV tile.
        :param z_top: Z of the top face.
        :returns: PrismMesh with positions, normals, uvs and indices.
        :complexity: O(n^2) worst case where n = total ring vertices
        """
    "#,
    module = "raygeo.mesh.build"
)]
#[pyfunction(name = "build_prism_mesh")]
#[pyo3(signature = (outer, holes = vec![], thickness = 18.0, uv_scale = 300.0, z_top = 0.0))]
fn build_prism_mesh_py(
    outer: Vec<(f64, f64)>,
    holes: Vec<Vec<(f64, f64)>>,
    thickness: f64,
    uv_scale: f64,
    z_top: f64,
) -> PyResult<PrismMesh> {
    let outer_pts: Vec<Point> =
        outer.iter().map(|p| Point::new(p.0, p.1)).collect();
    let hole_polys: Vec<Vec<Point>> = holes
        .iter()
        .map(|h| h.iter().map(|p| Point::new(p.0, p.1)).collect())
        .collect();
    let mesh = build::build_prism_mesh(
        &outer_pts,
        &hole_polys,
        thickness,
        uv_scale,
        z_top,
    )
    .map_err(pyo3::exceptions::PyValueError::new_err)?;
    Ok(PrismMesh { inner: mesh })
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "build")?;
    m.setattr("__doc__", "Constrained Delaunay triangulation.")?;
    m.add_function(pyo3::wrap_pyfunction!(build_triangle_mesh_py, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(build_uniform_mesh_py, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(build_prism_mesh_py, m.clone())?)?;
    parent.add_submodule(&m)?;
    let sys_modules = parent.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.mesh.build", &m)?;
    Ok(())
}
