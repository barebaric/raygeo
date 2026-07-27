use std::collections::HashMap;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_complex_enum, gen_stub_pymethods,
};

use crate::ops::convert::{
    gcode::GcodeSpec, texture::TextureSpec, vertex_arrays::VertexSpec,
    view::ViewSpec, EncodeCtx, EncodeOutput, Encoder,
};

pub(crate) mod dict;
pub(crate) mod gcode_spec;
pub(crate) mod numpy;
pub(crate) mod view;

pub(crate) use self::view::lut_to_array;

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
    if let Ok(s) = ob.extract::<PyViewSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(bound) = ob.cast::<PyPythonEncoder>() {
        return Ok(Box::new(bound.borrow().to_core()));
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

/// Python-side constructor for [`PythonEncoder`].
///
/// Wraps a Python callable ``(ops: Ops) -> EncodeOutput`` so it can
/// be driven through the Rust ``EncoderCompute`` stage. The callable
/// runs under the GIL on a rayon worker thread. Use this to route
/// encoders that remain in Python through the same pipeline as
/// native Rust encoders.
#[gen_stub_pyclass(module = "raygeo.ops.convert")]
#[pyclass(
    name = "PythonEncoder",
    module = "raygeo.ops.convert",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyPythonEncoder {
    #[pyo3(get)]
    pub callable: Py<PyAny>,
    #[pyo3(get)]
    pub name: String,
}

impl PyPythonEncoder {
    /// Convert into the core-layer [`PythonEncoder`].
    pub fn to_core(&self) -> PythonEncoder {
        PythonEncoder::new(self.callable.clone(), self.name.clone())
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPythonEncoder {
    /// Construct a Python-callable encoder.
    ///
    /// :param callable: A Python callable ``(ops: Ops) -> EncodeOutput``.
    /// :param name: Human-readable name for progress messages.
    #[new]
    fn new(callable: Py<PyAny>, name: String) -> Self {
        PyPythonEncoder { callable, name }
    }

    fn __repr__(&self) -> String {
        format!("PythonEncoder({:?})", self.name)
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

/// Parameters for the view encoder.
#[gen_stub_pyclass(module = "raygeo.ops.convert")]
#[pyclass(
    name = "ViewSpec",
    module = "raygeo.ops.convert",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyViewSpec {
    #[pyo3(get)]
    pub pixels_per_mm: (f64, f64),
    #[pyo3(get)]
    pub show_travel_moves: bool,
    #[pyo3(get)]
    pub render_bbox: (f64, f64, f64, f64),
    #[pyo3(get)]
    pub max_dimension_px: u32,
    #[pyo3(get)]
    pub max_total_pixels: u64,
    #[pyo3(get)]
    pub cut_color: [u8; 4],
    #[pyo3(get)]
    pub travel_color: [u8; 4],
    #[pyo3(get)]
    pub zero_power_color: [u8; 4],
    #[pyo3(get)]
    pub cut_lut: Vec<[u8; 4]>,
    #[pyo3(get)]
    pub engrave_lut: Vec<[u8; 4]>,
}

impl PyViewSpec {
    #[allow(clippy::wrong_self_convention)]
    pub fn into_core(self) -> ViewSpec {
        let cut_lut = lut_to_array(self.cut_lut)
            .expect("ViewSpec cut_lut validated at construction");
        let engrave_lut = lut_to_array(self.engrave_lut)
            .expect("ViewSpec engrave_lut validated at construction");
        ViewSpec {
            pixels_per_mm: self.pixels_per_mm,
            show_travel_moves: self.show_travel_moves,
            render_bbox: self.render_bbox,
            max_dimension_px: self.max_dimension_px,
            max_total_pixels: self.max_total_pixels,
            cut_color: self.cut_color,
            travel_color: self.travel_color,
            zero_power_color: self.zero_power_color,
            cut_lut,
            engrave_lut,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyViewSpec {
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        pixels_per_mm,
        render_bbox,
        cut_color,
        travel_color,
        zero_power_color,
        cut_lut,
        engrave_lut,
        show_travel_moves = true,
        max_dimension_px = 8192,
        max_total_pixels = 8192 * 8192,
    ))]
    fn new(
        pixels_per_mm: (f64, f64),
        render_bbox: (f64, f64, f64, f64),
        cut_color: [u8; 4],
        travel_color: [u8; 4],
        zero_power_color: [u8; 4],
        cut_lut: Vec<[u8; 4]>,
        engrave_lut: Vec<[u8; 4]>,
        show_travel_moves: bool,
        max_dimension_px: u32,
        max_total_pixels: u64,
    ) -> PyResult<Self> {
        if cut_lut.len() != 256 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "cut_lut must have exactly 256 entries",
            ));
        }
        if engrave_lut.len() != 256 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "engrave_lut must have exactly 256 entries",
            ));
        }
        Ok(PyViewSpec {
            pixels_per_mm,
            show_travel_moves,
            render_bbox,
            max_dimension_px,
            max_total_pixels,
            cut_color,
            travel_color,
            zero_power_color,
            cut_lut,
            engrave_lut,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "ViewSpec(ppm={:?}, render_bbox={:?}, travel={}, \
             cut={:?}, travel_color={:?}, zero_color={:?})",
            self.pixels_per_mm,
            self.render_bbox,
            self.show_travel_moves,
            self.cut_color,
            self.travel_color,
            self.zero_power_color,
        )
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
    View {
        buffer: Vec<u8>,
        width: usize,
        height: usize,
        bbox_mm: (f64, f64, f64, f64),
        effective_ppm: (f64, f64),
    },
}

impl PyEncodeOutput {
    pub fn from_core(core: crate::ops::convert::EncodeOutput) -> Self {
        match core {
            crate::ops::convert::EncodeOutput::MachineCode {
                text,
                op_to_machine_code,
                machine_code_to_op,
            } => PyEncodeOutput::MachineCode {
                text,
                op_to_machine_code,
                machine_code_to_op,
            },
            crate::ops::convert::EncodeOutput::VertexArrays(va) => {
                PyEncodeOutput::VertexArrays {
                    repr: format!("{:?}", va),
                }
            }
            crate::ops::convert::EncodeOutput::Texture {
                power_texture,
                width_px,
                height_px,
            } => PyEncodeOutput::Texture {
                power_texture,
                width_px,
                height_px,
            },
            crate::ops::convert::EncodeOutput::View {
                buffer,
                width,
                height,
                bbox_mm,
                effective_ppm,
            } => PyEncodeOutput::View {
                buffer,
                width,
                height,
                bbox_mm,
                effective_ppm,
            },
        }
    }

    /// Convert a Python-side ``PyEncodeOutput`` back into the core
    /// ``EncodeOutput``. Used by the Python-callable encoder
    /// ([`PythonEncoder`]) to hand a Python-produced result back to
    /// the Rust pipeline.
    pub fn into_core(self) -> EncodeOutput {
        match self {
            PyEncodeOutput::MachineCode {
                text,
                op_to_machine_code,
                machine_code_to_op,
            } => EncodeOutput::MachineCode {
                text,
                op_to_machine_code,
                machine_code_to_op,
            },
            PyEncodeOutput::VertexArrays { repr } => {
                let _ = repr;
                EncodeOutput::VertexArrays(
                    crate::ops::convert::vertex_arrays::VertexArrays {
                        powered_vertices: Vec::new(),
                        powered_colors: Vec::new(),
                        travel_vertices: Vec::new(),
                        zero_power_vertices: Vec::new(),
                    },
                )
            }
            PyEncodeOutput::Texture {
                power_texture,
                width_px,
                height_px,
            } => EncodeOutput::Texture {
                power_texture,
                width_px,
                height_px,
            },
            PyEncodeOutput::View {
                buffer,
                width,
                height,
                bbox_mm,
                effective_ppm,
            } => EncodeOutput::View {
                buffer,
                width,
                height,
                bbox_mm,
                effective_ppm,
            },
        }
    }
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
            PyEncodeOutput::View { width, height, .. } => {
                format!("EncodeOutput.View({}x{})", width, height)
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
            PyEncodeOutput::View { .. } => "View",
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
            EncodeOutput::View {
                buffer,
                width,
                height,
                bbox_mm,
                effective_ppm,
            } => PyEncodeOutput::View {
                buffer,
                width,
                height,
                bbox_mm,
                effective_ppm,
            },
        }
    }
}

// ── Python-callable encoder ───────────────────────────────────────

/// A Python callable wrapped as an [`Encoder`].
///
/// Holds a ``Py<PyAny>`` callable. On [`Encoder::encode`] the
/// callable is invoked under the GIL on the rayon worker thread
/// with a single positional argument — the ``Ops`` to encode — and
/// is expected to return a ``raygeo.ops.convert.EncodeOutput``
/// instance, which is converted back to the core
/// [`crate::ops::convert::EncodeOutput`].
///
/// This lets callers route Python-side encoders through the same
/// ``EncoderCompute`` stage as native Rust encoders.
pub struct PythonEncoder {
    /// The Python callable ``(ops) -> EncodeOutput``.
    callable: Py<PyAny>,
    /// Human-readable name used in progress messages.
    name: String,
}

impl PythonEncoder {
    /// Construct from a Python callable and a display name.
    pub fn new(callable: Py<PyAny>, name: String) -> Self {
        PythonEncoder { callable, name }
    }
}

impl Encoder for PythonEncoder {
    fn encode(&self, ctx: &mut EncodeCtx<'_>) -> Result<EncodeOutput, String> {
        Python::attach(|py| {
            if ctx.callbacks.is_cancelled() {
                return Err("cancelled".to_string());
            }
            let py_ops = crate::python::ops::container::PyOps {
                inner: ctx.ops.clone(),
            };
            let py_obj = Py::new(py, py_ops).map_err(|e| e.to_string())?;
            let result = self
                .callable
                .call1(py, (py_obj,))
                .map_err(|e| e.to_string())?;
            let bound = result.into_bound(py);
            let py_output = bound.cast::<PyEncodeOutput>().map_err(|_| {
                "Python encoder callable must return an \
                     EncodeOutput instance"
                    .to_string()
            })?;
            let py_output = py_output.borrow();
            Ok(py_output.clone().into_core())
        })
    }

    fn name(&self) -> &str {
        &self.name
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
    convert_mod.add_class::<PyViewSpec>()?;
    convert_mod.add_class::<PyPythonEncoder>()?;
    view::register(&convert_mod)?;
    ops_mod.add_submodule(&convert_mod)?;

    let sys_modules = ops_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.convert", &convert_mod)?;

    Ok(())
}
