use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::convert::{
    gcode::GcodeSpec as CoreGcodeSpec, texture::TextureSpec as CoreTextureSpec,
    vertex_arrays::VertexSpec as CoreVertexSpec, Encoder,
};

pub(crate) mod dict;
pub(crate) mod gcode_spec;
pub(crate) mod numpy;

pub(crate) use gcode_spec::PyGcodeDialectSpec;

/// Try to extract an encoder spec from a Python object.
///
/// Returns `PyTypeError` if the object is not one of the known spec
/// pyclasses. The returned `Box<dyn Encoder>` is consumed by
/// callers that drive the `Encoder` trait.
pub fn extract_encoder(
    py: Python<'_>,
    ob: &Bound<'_, PyAny>,
) -> PyResult<Box<dyn Encoder>> {
    if let Ok(s) = ob.extract::<PyGcodeSpec>() {
        return Ok(Box::new(s.into_core(py)));
    }
    if let Ok(s) = ob.extract::<PyVertexSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<PyTextureSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    let type_name = ob
        .get_type()
        .qualname()
        .map(|s| s.to_string())
        .unwrap_or_default();
    Err(PyErr::new::<PyTypeError, _>(format!(
        "Unknown encoder spec type: {type_name}"
    )))
}

/// Python-visible wrapper around an encoder spec.
///
/// Construct as ``Encoder(spec)`` where `spec` is an instance of one
/// of the encoder spec classes under `raygeo.ops.convert` (e.g.
/// :class:`~raygeo.ops.convert.GcodeSpec`). Callers that drive the
/// `Encoder` trait hold an `Encoder` instance.
#[gen_stub_pyclass(module = "raygeo.ops.convert")]
#[pyclass(name = "Encoder", module = "raygeo.ops.convert", skip_from_py_object)]
#[derive(Debug)]
pub struct PyEncoder {
    /// The wrapped Python-side spec object. Type-erased here;
    /// dispatched to a concrete `Box<dyn Encoder>` by
    /// [`PyEncoder::into_core`].
    #[pyo3(get)]
    pub spec: Py<PyAny>,
}

impl PyEncoder {
    /// Convert into the core-layer `Box<dyn Encoder>` by dispatching
    /// on the runtime type of `self.spec`.
    #[allow(clippy::wrong_self_convention)]
    pub fn into_core(&self, py: Python<'_>) -> PyResult<Box<dyn Encoder>> {
        extract_encoder(py, self.spec.bind(py))
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEncoder {
    /// Construct an `Encoder` wrapping a spec object.
    ///
    /// :param spec: An encoder spec instance (e.g.
    ///     :class:`~raygeo.ops.convert.GcodeSpec`).
    #[new]
    fn new(spec: Py<PyAny>) -> Self {
        PyEncoder { spec }
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let name = self
            .spec
            .bind(py)
            .get_type()
            .qualname()
            .map(|s| s.to_string())
            .unwrap_or_else(|_| "<unknown>".to_string());
        format!("Encoder({name})")
    }
}

/// Parameters for the G-code encoder.
#[gen_stub_pyclass(module = "raygeo.ops.convert")]
#[pyclass(
    name = "GcodeSpec",
    module = "raygeo.ops.convert",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyGcodeSpec {
    #[pyo3(get)]
    pub dialect: Py<PyGcodeDialectSpec>,
    #[pyo3(get)]
    pub context_json: String,
}

impl PyGcodeSpec {
    pub fn into_core(self, py: Python<'_>) -> CoreGcodeSpec {
        let dialect = self.dialect.borrow(py).0.clone();
        CoreGcodeSpec {
            dialect,
            context_json: self.context_json,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyGcodeSpec {
    #[new]
    fn new(dialect: Py<PyGcodeDialectSpec>, context_json: String) -> Self {
        PyGcodeSpec {
            dialect,
            context_json,
        }
    }
}

/// Parameters for the vertex-array encoder.
#[gen_stub_pyclass(module = "raygeo.ops.convert")]
#[pyclass(
    name = "VertexSpec",
    module = "raygeo.ops.convert",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PyVertexSpec {
    #[pyo3(get)]
    _tag: bool,
}

impl PyVertexSpec {
    pub fn into_core(self) -> CoreVertexSpec {
        CoreVertexSpec
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyVertexSpec {
    #[new]
    fn new() -> Self {
        PyVertexSpec { _tag: true }
    }
}

/// Parameters for the texture encoder.
#[gen_stub_pyclass(module = "raygeo.ops.convert")]
#[pyclass(
    name = "TextureSpec",
    module = "raygeo.ops.convert",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq, Debug)]
pub struct PyTextureSpec {
    #[pyo3(get)]
    pub width_px: u32,
    #[pyo3(get)]
    pub height_px: u32,
    #[pyo3(get)]
    pub px_per_mm: (f64, f64),
    #[pyo3(get)]
    pub origin_mm: (f64, f64),
}

impl PyTextureSpec {
    pub fn into_core(self) -> CoreTextureSpec {
        CoreTextureSpec {
            width_px: self.width_px,
            height_px: self.height_px,
            px_per_mm: self.px_per_mm,
            origin_mm: self.origin_mm,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyTextureSpec {
    #[new]
    #[allow(clippy::too_many_arguments)]
    fn new(
        width_px: u32,
        height_px: u32,
        px_per_mm: (f64, f64),
        origin_mm: (f64, f64),
    ) -> Self {
        PyTextureSpec {
            width_px,
            height_px,
            px_per_mm,
            origin_mm,
        }
    }
}

pub(crate) fn register(ops_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let convert_mod = PyModule::new(ops_mod.py(), "convert")?;
    convert_mod.add_class::<PyGcodeDialectSpec>()?;
    convert_mod.add_class::<PyEncoder>()?;
    convert_mod.add_class::<PyGcodeSpec>()?;
    convert_mod.add_class::<PyVertexSpec>()?;
    convert_mod.add_class::<PyTextureSpec>()?;
    ops_mod.add_submodule(&convert_mod)?;

    let sys_modules = ops_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.convert", &convert_mod)?;

    Ok(())
}
