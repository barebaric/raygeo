use std::collections::HashMap;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_complex_enum, gen_stub_pymethods,
};

use crate::ops::convert::{
    gcode::GcodeSpec, texture::TextureSpec, vertex_arrays::VertexSpec,
    EncodeOutput, Encoder,
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
    pub fn into_core(self, py: Python<'_>) -> GcodeSpec {
        let dialect = self.dialect.borrow(py).0.clone();
        GcodeSpec {
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
    pub fn into_core(self) -> VertexSpec {
        VertexSpec
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
    pub fn into_core(self) -> TextureSpec {
        TextureSpec {
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

/// Non-Ops artifact produced by an Encode stage.
#[gen_stub_pyclass_complex_enum]
#[pyclass(
    name = "EncodeOutput",
    module = "raygeo.ops.convert",
    skip_from_py_object
)]
#[derive(Clone)]
pub enum PyEncodeOutput {
    MachineCode {
        text: String,
        op_to_machine_code: HashMap<usize, Vec<usize>>,
        machine_code_to_op: HashMap<usize, usize>,
    },
    VertexArrays {
        repr: String,
    },
    Texture {
        power_texture: Vec<u8>,
        width_px: u32,
        height_px: u32,
    },
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEncodeOutput {
    /// Variant name as a string: ``"MachineCode"``, ``"VertexArrays"``,
    /// or ``"Texture"``.
    fn __repr__(&self) -> String {
        match self {
            PyEncodeOutput::MachineCode { .. } => {
                "EncodeOutput.MachineCode".to_string()
            }
            PyEncodeOutput::VertexArrays { .. } => {
                "EncodeOutput.VertexArrays".to_string()
            }
            PyEncodeOutput::Texture {
                width_px,
                height_px,
                ..
            } => {
                format!("EncodeOutput.Texture({}x{})", width_px, height_px)
            }
        }
    }

    /// The variant's name: ``"MachineCode"``, ``"VertexArrays"``, or
    /// ``"Texture"``.
    #[getter]
    fn variant(&self) -> &'static str {
        match self {
            PyEncodeOutput::MachineCode { .. } => "MachineCode",
            PyEncodeOutput::VertexArrays { .. } => "VertexArrays",
            PyEncodeOutput::Texture { .. } => "Texture",
        }
    }

    /// The G-code text. Returns ``None`` unless this is the
    /// ``MachineCode`` variant.
    #[getter]
    fn text(&self) -> Option<String> {
        match self {
            PyEncodeOutput::MachineCode { text, .. } => Some(text.clone()),
            _ => None,
        }
    }

    /// Mapping ``op_index -> list of machine-code line indices``.
    /// Returns ``None`` unless this is the ``MachineCode`` variant.
    #[getter]
    fn op_to_machine_code(&self) -> Option<HashMap<usize, Vec<usize>>> {
        match self {
            PyEncodeOutput::MachineCode {
                op_to_machine_code, ..
            } => Some(op_to_machine_code.clone()),
            _ => None,
        }
    }

    /// Mapping ``machine-code line index -> op_index``.
    /// Returns ``None`` unless this is the ``MachineCode`` variant.
    #[getter]
    fn machine_code_to_op(&self) -> Option<HashMap<usize, usize>> {
        match self {
            PyEncodeOutput::MachineCode {
                machine_code_to_op, ..
            } => Some(machine_code_to_op.clone()),
            _ => None,
        }
    }

    /// The vertex-array debug repr. Returns ``None`` unless this is
    /// the ``VertexArrays`` variant.
    #[getter]
    fn repr(&self) -> Option<String> {
        match self {
            PyEncodeOutput::VertexArrays { repr } => Some(repr.clone()),
            _ => None,
        }
    }

    /// Raw texture bytes (row-major uint8 power map). Returns
    /// ``None`` unless this is the ``Texture`` variant.
    #[getter]
    fn power_texture(&self) -> Option<Vec<u8>> {
        match self {
            PyEncodeOutput::Texture { power_texture, .. } => {
                Some(power_texture.clone())
            }
            _ => None,
        }
    }

    /// Texture width in pixels. Returns ``None`` unless this is the
    /// ``Texture`` variant.
    #[getter]
    fn width_px(&self) -> Option<u32> {
        match self {
            PyEncodeOutput::Texture { width_px, .. } => Some(*width_px),
            _ => None,
        }
    }

    /// Texture height in pixels. Returns ``None`` unless this is the
    /// ``Texture`` variant.
    #[getter]
    fn height_px(&self) -> Option<u32> {
        match self {
            PyEncodeOutput::Texture { height_px, .. } => Some(*height_px),
            _ => None,
        }
    }
}

impl From<EncodeOutput> for PyEncodeOutput {
    fn from(eo: EncodeOutput) -> Self {
        match eo {
            EncodeOutput::MachineCode {
                text,
                op_to_machine_code,
                machine_code_to_op,
            } => PyEncodeOutput::MachineCode {
                text,
                op_to_machine_code,
                machine_code_to_op,
            },
            EncodeOutput::VertexArrays(_va) => PyEncodeOutput::VertexArrays {
                repr: "<VertexArrays>".to_string(),
            },
            EncodeOutput::Texture {
                power_texture,
                width_px,
                height_px,
            } => PyEncodeOutput::Texture {
                power_texture,
                width_px,
                height_px,
            },
        }
    }
}

pub(crate) fn register(ops_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let convert_mod = PyModule::new(ops_mod.py(), "convert")?;
    convert_mod.add_class::<PyEncodeOutput>()?;
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
