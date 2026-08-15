use numpy::IntoPyArray;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::mesh::types::{
    BoundaryTag, PrismMesh as RustPrismMesh, TriangleMesh as RustMesh,
};

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

/// GPU-ready prism mesh returned by ``build_prism_mesh``.
///
/// Buffers are per-face-vertex (not shared).  All getters return
/// fresh numpy arrays: float32 positions (N, 3), float32 normals
/// (N, 3), float32 UVs (N, 2) and flat uint32 triangle indices (3T,).
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.mesh.types", name = "PrismMesh")]
pub struct PrismMesh {
    pub(crate) inner: RustPrismMesh,
}

impl PrismMesh {
    /// Move a flat f32 buffer into a 1-D numpy array reshaped to
    /// (rows, cols).  Reshaping a contiguous array is a zero-copy
    /// view, so no extra buffer is allocated.
    fn flat_to_2d<'py>(
        py: Python<'py>,
        data: Vec<f32>,
        cols: usize,
    ) -> PyResult<Bound<'py, PyAny>> {
        let rows = (data.len() / cols) as isize;
        let array = data.into_pyarray(py).into_any();
        array.call_method1("reshape", (rows, cols as isize))
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PrismMesh {
    /// Flat XYZ vertex positions as a float32 array of shape (N, 3).
    #[getter]
    fn positions<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Self::flat_to_2d(py, self.inner.positions.clone(), 3)
    }

    /// Flat XYZ vertex normals as a float32 array of shape (N, 3).
    #[getter]
    fn normals<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Self::flat_to_2d(py, self.inner.normals.clone(), 3)
    }

    /// Flat XY UV coordinates as a float32 array of shape (N, 2).
    #[getter]
    fn uvs<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Self::flat_to_2d(py, self.inner.uvs.clone(), 2)
    }

    /// Flat triangle vertex indices as a uint32 array of shape (3T,).
    #[getter]
    fn indices<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let array = self.inner.indices.clone().into_pyarray(py);
        Ok(array.into_any())
    }

    fn __repr__(&self) -> String {
        format!(
            "PrismMesh(vertices={}, triangles={})",
            self.inner.positions.len() / 3,
            self.inner.indices.len() / 3
        )
    }
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "types")?;
    m.setattr("__doc__", "Mesh data types (TriangleMesh, PrismMesh).")?;
    m.add_class::<TriangleMesh>()?;
    m.add_class::<PrismMesh>()?;
    parent.add_submodule(&m)?;
    let sys_modules = parent.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.mesh.types", &m)?;
    Ok(())
}
