use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::mesh::types::{BoundaryTag, TriangleMesh as RustMesh};

#[gen_stub_pyclass]
#[pyclass(module = "raygeo.mesh.types", name = "TriangleMesh")]
pub struct TriangleMesh {
    pub(crate) inner: RustMesh,
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

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "types")?;
    m.setattr("__doc__", "Mesh data types (TriangleMesh).")?;
    m.add_class::<TriangleMesh>()?;
    parent.add_submodule(&m)?;
    let sys_modules = parent.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.mesh.types", &m)?;
    Ok(())
}
