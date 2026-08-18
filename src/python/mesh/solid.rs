//! Python bindings for `raygeo.mesh.solid`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::geo::types::Point3D;
use crate::mesh::solid::SolidMesh;

pyo3_stub_gen::module_doc!(
    "raygeo.mesh.solid",
    "{}",
    "Plain-data closed-manifold triangle meshes for solid interchange."
);

/// A closed-manifold triangle mesh in millimetres.
///
/// The interchange format for solid geometry: f64 positions plus
/// triangle indices and nothing else.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.mesh.solid", name = "SolidMesh")]
pub struct PySolidMesh {
    pub(crate) inner: SolidMesh,
}

#[gen_stub_pymethods]
#[pymethods]
impl PySolidMesh {
    #[new]
    fn new(
        positions: Vec<(f64, f64, f64)>,
        triangles: Vec<(u32, u32, u32)>,
    ) -> Self {
        PySolidMesh {
            inner: SolidMesh::new(
                positions
                    .iter()
                    .map(|p| Point3D::new(p.0, p.1, p.2))
                    .collect(),
                triangles.iter().map(|t| [t.0, t.1, t.2]).collect(),
            ),
        }
    }

    /// Vertex positions (world mm).
    #[getter]
    fn positions(&self) -> Vec<(f64, f64, f64)> {
        self.inner
            .positions
            .iter()
            .map(|p| (p.x, p.y, p.z))
            .collect()
    }

    /// Triangles as indices into ``positions``.
    #[getter]
    fn triangles(&self) -> Vec<(u32, u32, u32)> {
        self.inner
            .triangles
            .iter()
            .map(|t| (t[0], t[1], t[2]))
            .collect()
    }

    fn __len__(&self) -> usize {
        self.inner.triangles.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "SolidMesh(vertices={}, triangles={})",
            self.inner.positions.len(),
            self.inner.triangles.len()
        )
    }
}

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "solid")?;
    m.setattr(
        "__doc__",
        "Plain-data closed-manifold triangle meshes for solid interchange.",
    )?;
    m.add_class::<PySolidMesh>()?;
    parent.add_submodule(&m)?;
    let sys_modules = parent.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.mesh.solid", &m)?;
    Ok(())
}
