pyo3_stub_gen::module_doc!("raygeo.ops.assembly", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Motion-path assembly: turning raw geometry primitives into Ops.

Functions in this module compose geo-layer primitives (polylines, arcs,
polygons) into complete motion sequences represented as Ops objects.
They decide traversal order, linking strategy, lead-in/out, overscan,
and tab insertion — concerns that belong to motion assembly rather
than pure geometry.

Each assembler exposes a spec class (e.g.
:class:`~raygeo.ops.assembly.contour.ContourSpec`) implementing the
Rust ``Assembler`` trait; :class:`Assembler` wraps any spec so callers
can drive it through the trait.
";

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods,
};

use crate::ops::assembly::{
    Assembler, AssemblyWarning, AssemblyWarningKind, ProgressEvent,
};

pub(crate) mod adaptive;
pub(crate) mod contour;
pub(crate) mod frame;
pub(crate) mod helix;
pub(crate) mod material_test_grid;
pub(crate) mod profile;
pub(crate) mod ramp;
pub(crate) mod raster;
pub(crate) mod result;
pub(crate) mod shrinkwrap;
pub(crate) mod slot;
pub(crate) mod spiral;
pub(crate) mod toroid;
pub(crate) mod wavefront;

use crate::ops::assembly::AssemblyOutput;
use crate::python::geo::flex_point::{
    polygons_from_tuples, polygons_to_tuples,
};
use crate::python::ops::assembly::adaptive::PyAdaptiveClearingSpec;
use crate::python::ops::assembly::contour::PyContourSpec;
use crate::python::ops::assembly::frame::PyFrameSpec;
use crate::python::ops::assembly::helix::PyHelixSpec;
use crate::python::ops::assembly::material_test_grid::PyMaterialTestGridSpec;
use crate::python::ops::assembly::profile::PyProfileSpec;
use crate::python::ops::assembly::ramp::PyRampSpec;
use crate::python::ops::assembly::raster::PyRasterSpec;
use crate::python::ops::assembly::shrinkwrap::PyShrinkwrapSpec;
use crate::python::ops::assembly::slot::PySlotSpec;
use crate::python::ops::assembly::spiral::PySpiralSpec;
use crate::python::ops::assembly::toroid::{PyToroidSpec, PyToroidalClearSpec};
use crate::python::ops::assembly::wavefront::PyAdaptiveWavefrontSpec;
use crate::python::ops::container::PyOps;

/// Try to extract an assembler spec from a Python object.
///
/// Returns `PyTypeError` if the object is not one of the known spec
/// pyclasses. The returned `Box<dyn Assembler>` is consumed by
/// callers that drive the `Assembler` trait.
pub fn extract_assembler(
    ob: &Bound<'_, PyAny>,
) -> PyResult<Box<dyn Assembler>> {
    if let Ok(s) = ob.extract::<PyContourSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<PyAdaptiveClearingSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<PySpiralSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<PySlotSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<PyToroidSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<PyToroidalClearSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<PyHelixSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<PyRampSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<PyAdaptiveWavefrontSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<PyMaterialTestGridSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<PyProfileSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<PyFrameSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<PyRasterSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<PyShrinkwrapSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    let type_name = ob
        .get_type()
        .qualname()
        .map(|s| s.to_string())
        .unwrap_or_default();
    Err(PyErr::new::<PyTypeError, _>(format!(
        "Unknown assembler spec type: {type_name}"
    )))
}

/// Machine-readable category for a non-fatal :class:`AssemblyWarning`.
///
/// Mirrors the Rust :class:`~raygeo.ops.assembly.AssemblyWarningKind`; the
/// consumer maps each variant to a translatable message template.
#[gen_stub_pyclass_enum]
#[pyclass(
    module = "raygeo.ops.assembly",
    name = "AssemblyWarningKind",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PyAssemblyWarningKind {
    /// A whole face's assembly failed; processing continued.
    #[pyo3(name = "FACE_FAILED")]
    FaceFailed,
    /// A single region within a face failed; other regions cleared.
    #[pyo3(name = "REGION_FAILED")]
    RegionFailed,
}

#[pymethods]
impl PyAssemblyWarningKind {
    fn __repr__(&self) -> String {
        match self {
            PyAssemblyWarningKind::FaceFailed => {
                "AssemblyWarningKind.FACE_FAILED".into()
            }
            PyAssemblyWarningKind::RegionFailed => {
                "AssemblyWarningKind.REGION_FAILED".into()
            }
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    #[getter]
    fn value(&self) -> &str {
        match self {
            PyAssemblyWarningKind::FaceFailed => "face_failed",
            PyAssemblyWarningKind::RegionFailed => "region_failed",
        }
    }
}

impl From<AssemblyWarningKind> for PyAssemblyWarningKind {
    fn from(k: AssemblyWarningKind) -> Self {
        match k {
            AssemblyWarningKind::FaceFailed => {
                PyAssemblyWarningKind::FaceFailed
            }
            AssemblyWarningKind::RegionFailed => {
                PyAssemblyWarningKind::RegionFailed
            }
        }
    }
}

/// A non-fatal warning emitted during assembly.
///
/// Assemblers push these instead of aborting when a single face or region
/// fails; the failed face/region is skipped and the rest of the part is
/// still machined. Use :attr:`kind` to pick a translation template and
/// :attr:`detail` for the raw, non-translatable diagnostic.
#[gen_stub_pyclass(module = "raygeo.ops.assembly")]
#[pyclass(
    name = "AssemblyWarning",
    module = "raygeo.ops.assembly",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyAssemblyWarning {
    pub inner: AssemblyWarning,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyAssemblyWarning {
    /// What failed — determines the translation template.
    #[getter]
    fn kind(&self) -> PyAssemblyWarningKind {
        PyAssemblyWarningKind::from(self.inner.kind.clone())
    }

    /// Face id; ``""`` is the default face, ``"1"``, ``"2"``, ... others.
    #[getter]
    fn face_id(&self) -> &str {
        &self.inner.face_id
    }

    /// Region index within the face; ``None`` for whole-face failures.
    #[getter]
    fn region(&self) -> Option<usize> {
        self.inner.region
    }

    /// Raw, non-translatable diagnostic (the assembler's error string).
    #[getter]
    fn detail(&self) -> &str {
        &self.inner.detail
    }

    fn __repr__(&self) -> String {
        let kind = PyAssemblyWarningKind::from(self.inner.kind.clone());
        format!(
            "AssemblyWarning(kind={}, face_id={:?}, region={:?})",
            kind.__repr__(),
            self.inner.face_id,
            self.inner.region,
        )
    }
}

/// The output of an assembler, packaged for caching.
///
/// Produced by
/// :meth:`Assembler.store_cache() <raygeo.ops.assembly.Assembler.store_cache>`
/// and consumed by
/// :meth:`Assembler.restore_cache() <raygeo.ops.assembly.Assembler.restore_cache>`.
///
/// Carries the assembled ``Ops``, metadata, and optional post-assembly
/// cleared fragments for face-state restoration on cache hit.
#[gen_stub_pyclass(module = "raygeo.ops.assembly")]
#[pyclass(
    name = "AssemblyOutput",
    module = "raygeo.ops.assembly",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyAssemblyOutput {
    pub inner: AssemblyOutput,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyAssemblyOutput {
    #[new]
    #[pyo3(signature = (ops, is_scalable = false, source_dimensions = None, cleared_fragments = None))]
    fn new(
        ops: &PyOps,
        is_scalable: bool,
        source_dimensions: Option<(f64, f64)>,
        cleared_fragments: Option<Vec<Vec<(f64, f64)>>>,
    ) -> Self {
        use crate::ops::types::ToolPose;
        use crate::types::Point3D;
        let frags = cleared_fragments.map(polygons_from_tuples);
        PyAssemblyOutput {
            inner: AssemblyOutput {
                ops: ops.inner.clone(),
                is_scalable,
                source_dimensions,
                cleared_fragments: frags,
                meta: crate::ops::assembly::AssemblyMeta {
                    start: ToolPose {
                        pos: Point3D::ZERO,
                        heading: 0.0,
                    },
                    end: ToolPose {
                        pos: Point3D::ZERO,
                        heading: 0.0,
                    },
                },
                warnings: Vec::new(),
            },
        }
    }

    /// The assembled Ops.
    #[getter]
    fn ops(&self) -> PyOps {
        PyOps {
            inner: self.inner.ops.clone(),
        }
    }

    /// Whether the Ops may be uniformly scaled during aggregation.
    #[getter]
    fn is_scalable(&self) -> bool {
        self.inner.is_scalable
    }

    /// Source ``(width_mm, height_mm)`` of the part that produced the Ops.
    #[getter]
    fn source_dimensions(&self) -> Option<(f64, f64)> {
        self.inner.source_dimensions
    }

    /// Post-assembly cleared fragments (``list[list[(x, y)]]``), or
    /// ``None`` for assemblers that don't touch ``FaceState.cleared``.
    #[getter]
    fn cleared_fragments(&self) -> Option<Vec<Vec<(f64, f64)>>> {
        self.inner
            .cleared_fragments
            .as_ref()
            .map(|frags| polygons_to_tuples(frags.clone()))
    }

    /// Non-fatal warnings emitted during assembly (``list[AssemblyWarning]``).
    /// Empty when assembly completed without per-face/region failures.
    #[getter]
    fn warnings(&self) -> Vec<PyAssemblyWarning> {
        self.inner
            .warnings
            .iter()
            .map(|w| PyAssemblyWarning { inner: w.clone() })
            .collect()
    }

    fn __repr__(&self) -> String {
        let n_frags = self
            .inner
            .cleared_fragments
            .as_ref()
            .map(|f| f.len())
            .unwrap_or(0);
        format!(
            "AssemblyOutput(ops_len={}, is_scalable={}, source_dimensions={:?}, n_fragments={}, n_warnings={})",
            self.inner.ops.len(),
            self.inner.is_scalable,
            self.inner.source_dimensions,
            n_frags,
            self.inner.warnings.len(),
        )
    }
}

/// Python-visible wrapper around an assembler spec.
///
/// Construct as ``Assembler(spec)`` where `spec` is an instance of
/// one of the assembler spec classes under `raygeo.ops.assembly.*`
/// (e.g. :class:`~raygeo.ops.assembly.contour.ContourSpec`). Callers
/// that drive the `Assembler` trait hold an `Assembler` instance.
#[gen_stub_pyclass(module = "raygeo.ops.assembly")]
#[pyclass(
    name = "Assembler",
    module = "raygeo.ops.assembly",
    skip_from_py_object
)]
#[derive(Debug)]
pub struct PyAssembler {
    /// The wrapped Python-side spec object. Type-erased here;
    /// dispatched to a concrete `Box<dyn Assembler>` by
    /// [`PyAssembler::into_core`].
    #[pyo3(get)]
    pub spec: Py<PyAny>,
}

impl PyAssembler {
    /// Convert into the core-layer `Box<dyn Assembler>` by
    /// dispatching on the runtime type of `self.spec`.
    #[allow(clippy::wrong_self_convention)]
    pub fn into_core(&self, py: Python<'_>) -> PyResult<Box<dyn Assembler>> {
        extract_assembler(self.spec.bind(py))
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyAssembler {
    /// Construct an `Assembler` wrapping a spec object.
    ///
    /// :param spec: An assembler spec instance (e.g.
    ///     :class:`~raygeo.ops.assembly.contour.ContourSpec`).
    #[new]
    fn new(spec: Py<PyAny>) -> Self {
        PyAssembler { spec }
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let name = self
            .spec
            .bind(py)
            .get_type()
            .qualname()
            .map(|s| s.to_string())
            .unwrap_or_else(|_| "<unknown>".to_string());
        format!("Assembler({name})")
    }
}

/// Convert a Rust ProgressEvent into a Python dict.
pub(crate) fn progress_event_to_py(
    py: Python<'_>,
    event: ProgressEvent,
) -> Py<PyAny> {
    match event {
        ProgressEvent::StepStart { step_index, label } => {
            let d = PyDict::new(py);
            d.set_item("kind", "step_start").unwrap();
            d.set_item("step_index", step_index).unwrap();
            d.set_item("label", label).unwrap();
            d.into_any().unbind()
        }
        ProgressEvent::Ops {
            commands,
            ops_total,
        } => {
            let d = PyDict::new(py);
            d.set_item("kind", "ops").unwrap();
            d.set_item("ops_count", commands.len()).unwrap();
            d.set_item("ops_total", ops_total).unwrap();
            d.into_any().unbind()
        }
        ProgressEvent::StepEnd { step_index } => {
            let d = PyDict::new(py);
            d.set_item("kind", "step_end").unwrap();
            d.set_item("step_index", step_index).unwrap();
            d.into_any().unbind()
        }
    }
}

pub(crate) fn register(ops_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = ops_mod.py();
    let assembly_mod = PyModule::new(py, "assembly")?;
    assembly_mod.setattr("__doc__", MODULE_DOC)?;
    assembly_mod.add_class::<PyAssembler>()?;
    assembly_mod.add_class::<PyAssemblyOutput>()?;
    assembly_mod.add_class::<PyAssemblyWarningKind>()?;
    assembly_mod.add_class::<PyAssemblyWarning>()?;

    adaptive::register(&assembly_mod)?;
    contour::register(&assembly_mod)?;
    frame::register(&assembly_mod)?;
    helix::register(&assembly_mod)?;
    material_test_grid::register(&assembly_mod)?;
    raster::register(&assembly_mod)?;
    shrinkwrap::register(&assembly_mod)?;
    profile::register(&assembly_mod)?;
    ramp::register(&assembly_mod)?;
    result::register(&assembly_mod)?;
    slot::register(&assembly_mod)?;
    spiral::register(&assembly_mod)?;
    toroid::register(&assembly_mod)?;
    wavefront::register(&assembly_mod)?;

    ops_mod.add_submodule(&assembly_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly", &assembly_mod)?;

    Ok(())
}
