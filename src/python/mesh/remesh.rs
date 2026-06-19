use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::mesh::remesh::remesh as rust_remesh;

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.mesh.types

    def remesh(
        mesh: types.TriangleMesh,
        outer: collections.abc.Sequence[tuple[float, float]],
        max_edge_len: float = 1.0,
    ) -> types.TriangleMesh:
        """Refine a triangle mesh so no interior edge exceeds
        *max_edge_len*.

        Boundary edges are preserved; only edges with at least one
        free (non-boundary) vertex are subdivided.

        :param mesh: Input TriangleMesh to refine.
        :param outer: Outer boundary polygon (for retriangulation).
        :param max_edge_len: Maximum allowed edge length (default 1.0).
        :returns: A refined TriangleMesh.
        :raises RuntimeError: If retriangulation fails.
        """
"#,
    module = "raygeo.mesh.remesh"
)]
#[pyfunction(name = "remesh")]
#[pyo3(signature = (mesh, outer, max_edge_len = 1.0))]
fn remesh_py(
    mesh: &crate::python::mesh::types::TriangleMesh,
    outer: Vec<(f64, f64)>,
    max_edge_len: f64,
) -> PyResult<crate::python::mesh::types::TriangleMesh> {
    let boundary: Vec<crate::types::Point> = outer
        .into_iter()
        .map(|(x, y)| crate::types::Point::new(x, y))
        .collect();

    let result = rust_remesh(&mesh.inner, &boundary, max_edge_len)
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;

    Ok(crate::python::mesh::types::TriangleMesh { inner: result })
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let m = PyModule::new(py, "remesh")?;
    m.setattr("__doc__", "Uniform mesh refinement.")?;
    register_functions!(m, remesh_py);
    parent.add_submodule(&m)?;
    Ok(())
}
