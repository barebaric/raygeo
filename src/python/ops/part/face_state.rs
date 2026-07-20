use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::part::FaceState;
use crate::python::geo::geometry::Geometry as PyGeometry;
use crate::python::ops::part::cleared_area::PyClearedArea;
use crate::python::ops::part::stock_region::PyStockRegion;

/// Geometry, stock region, and cleared area for one face of a
/// multi-face part.
///
/// Read-only snapshot. Assemblers mutate face state internally;
/// use the getters to inspect the state after assembly.
#[gen_stub_pyclass(module = "raygeo.ops.part")]
#[pyclass(name = "FaceState", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyFaceState {
    pub(crate) inner: FaceState,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyFaceState {
    /// Vector geometry for this face, if any.
    #[getter]
    fn geometry(&self) -> Option<PyGeometry> {
        self.inner
            .geometry
            .as_ref()
            .map(|g| PyGeometry { inner: g.clone() })
    }

    /// Boundary and islands of this face — the geometric input to
    /// clearing operations.
    #[getter]
    fn stock_region(&self) -> PyStockRegion {
        PyStockRegion {
            inner: self.inner.stock_region.clone(),
        }
    }

    /// Accumulated cleared-area state for this face — what has been
    /// cut so far.
    #[getter]
    fn cleared(&self) -> PyClearedArea {
        PyClearedArea {
            inner: self.inner.cleared.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "FaceState(geometry={}, stock_region=..., cleared=...)",
            if self.inner.geometry.is_some() {
                "Some"
            } else {
                "None"
            },
        )
    }
}

pub fn register(cut_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    cut_mod.add_class::<PyFaceState>()?;
    Ok(())
}
