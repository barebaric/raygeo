use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::ops::assembly::frame::{assemble_frame, FrameSpec as CoreFrameSpec};
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::part::part::PyPart;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "frame")?;
    m.add_function(pyo3::wrap_pyfunction!(frame_py, m.clone())?)?;
    m.add_class::<PyFrameSpec>()?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.frame", &m)?;

    Ok(())
}

/// Parameters for the ``frame`` assembler.
///
/// Construct with ``FrameSpec(kerf_mm, path_offset_mm, cut_side)``.
/// Wrap in an :class:`~raygeo.ops.assembly.Assembler` instance to
/// drive the `Assembler` trait.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.assembly.frame",
    name = "FrameSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyFrameSpec {
    /// Tool kerf width in mm.
    #[pyo3(get)]
    pub kerf_mm: f64,
    /// Additional offset distance in mm.
    #[pyo3(get)]
    pub path_offset_mm: f64,
    /// ``"centerline"``, ``"outside"``, or ``"inside"``.
    #[pyo3(get)]
    pub cut_side: String,
}

impl PyFrameSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreFrameSpec {
        CoreFrameSpec {
            kerf_mm: self.kerf_mm,
            path_offset_mm: self.path_offset_mm,
            cut_side: self.cut_side,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyFrameSpec {
    #[new]
    #[pyo3(signature = (
        kerf_mm = 0.0,
        path_offset_mm = 0.0,
        cut_side = "centerline",
    ))]
    fn new(kerf_mm: f64, path_offset_mm: f64, cut_side: &str) -> Self {
        PyFrameSpec {
            kerf_mm,
            path_offset_mm,
            cut_side: cut_side.to_string(),
        }
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def frame(
        part: raygeo.ops.part.Part,
        kerf_mm: float = 0.0,
        path_offset_mm: float = 0.0,
        cut_side: str = "centerline",
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Generate a rectangular frame around the part boundary.

        Creates a rectangle matching ``part.size_mm``, computes the
        total offset from kerf / path-offset / cut-side, applies it,
        and returns the frame as an :class:`AssemblyResult`.

        :param part: The part whose size defines the frame.
        :param kerf_mm: Tool kerf width in mm (default 0.0).
        :param path_offset_mm: Additional offset distance in mm
            (default 0.0).
        :param cut_side: ``"centerline"``, ``"outside"``, or
            ``"inside"`` (default ``"centerline"``).
        :returns: An :class:`AssemblyResult` with the frame path.
        :raises ValueError: If the part has no size information.
        """
    "#,
    module = "raygeo.ops.assembly.frame"
)]
#[pyfunction(name = "frame")]
#[pyo3(signature = (
    part,
    kerf_mm = 0.0,
    path_offset_mm = 0.0,
    cut_side = "centerline",
))]
fn frame_py(
    part: &PyPart,
    kerf_mm: f64,
    path_offset_mm: f64,
    cut_side: &str,
) -> PyResult<PyAssemblyResult> {
    let (ops, meta) =
        assemble_frame(part.inner.size_mm, kerf_mm, path_offset_mm, cut_side)?;
    Ok(PyAssemblyResult::from_parts(ops, meta, None, vec![]))
}
