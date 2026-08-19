//! Python bindings for `raygeo.ops.material.state`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::material::state::MaterialState;
use crate::ops::material::FoldProfile;
use crate::python::compressed_array::PyCompressedArray;

use super::polygons_to_tuples;
use super::spec::PyGridSpec;

pyo3_stub_gen::module_doc!("raygeo.ops.material.state", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
The folded state of one stock: an immutable snapshot.
";

/// The folded state of one stock: an immutable snapshot.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops.material.state", name = "MaterialState")]
pub struct PyMaterialState {
    pub(crate) inner: MaterialState,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyMaterialState {
    /// Which profile produced this state (``"prismatic"``).
    #[getter]
    fn profile(&self) -> &'static str {
        match self.inner.profile {
            FoldProfile::Prismatic => "prismatic",
            FoldProfile::Solid => "solid",
        }
    }

    /// Regions removed through the full stock thickness (world mm).
    #[getter]
    fn void_polygons(&self) -> Vec<Vec<(f64, f64)>> {
        polygons_to_tuples(&self.inner.void_polygons)
    }

    /// Removal-depth heightmap in mm, or ``None`` until depth
    /// folding lands.
    #[getter]
    fn depth_field(&self) -> Option<PyCompressedArray> {
        self.inner
            .depth_field
            .as_ref()
            .map(|d| PyCompressedArray::from_inner(d.clone()))
    }

    /// Per-pixel maximum laser power (R8), or ``None`` when no
    /// raster effects contributed.
    #[getter]
    fn surface_map(&self) -> Option<PyCompressedArray> {
        self.inner
            .surface_map
            .as_ref()
            .map(|s| PyCompressedArray::from_inner(s.clone()))
    }

    /// Grid shared by the raster outputs.
    #[getter]
    fn grid(&self) -> Option<PyGridSpec> {
        self.inner.grid.as_ref().map(|g| PyGridSpec {
            origin_mm: g.origin_mm,
            px_per_mm: g.px_per_mm,
            size_px: g.size_px,
        })
    }

    /// Sorted unique source keys whose effects were applied.
    #[getter]
    fn provenance(&self) -> Vec<String> {
        self.inner.provenance.clone()
    }

    /// First invariant violation encountered (``"top_open_violation"``
    /// or ``"solid_profile_required"``), or ``None``.
    #[getter]
    fn escalation(&self) -> Option<String> {
        self.inner.escalation.as_ref().map(|e| e.kind().to_string())
    }

    fn __repr__(&self) -> String {
        format!(
            "MaterialState(profile={:?}, voids={}, provenance={}, escalation={:?})",
            self.profile(),
            self.inner.void_polygons.len(),
            self.inner.provenance.len(),
            self.inner.escalation.as_ref().map(|e| e.kind()),
        )
    }
}

pub(crate) fn register(mat_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = mat_mod.py();
    let m = PyModule::new(py, "state")?;
    m.setattr("__doc__", MODULE_DOC)?;
    m.add_class::<PyMaterialState>()?;

    mat_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.material.state", &m)?;

    Ok(())
}
