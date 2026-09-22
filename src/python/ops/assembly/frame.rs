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
/// Construct with ``FrameSpec(offset_mm, cut_side, corner_radius,
/// arc_tolerance, allow_arcs)``.
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
    /// Total path offset distance in mm.
    #[pyo3(get)]
    pub offset_mm: f64,
    /// ``"centerline"``, ``"outside"``, or ``"inside"``.
    #[pyo3(get)]
    pub cut_side: String,
    /// Corner rounding radius in mm (0 = sharp corners).
    #[pyo3(get)]
    pub corner_radius: f64,
    /// Maximum chord deviation in mm when arcs are not supported.
    #[pyo3(get)]
    pub arc_tolerance: f64,
    /// Keep rounded corners as arcs; when false they are linearised.
    #[pyo3(get)]
    pub allow_arcs: bool,
}

impl PyFrameSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreFrameSpec {
        CoreFrameSpec {
            offset_mm: self.offset_mm,
            cut_side: self.cut_side,
            corner_radius: self.corner_radius,
            arc_tolerance: self.arc_tolerance,
            allow_arcs: self.allow_arcs,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyFrameSpec {
    #[new]
    #[pyo3(signature = (
        offset_mm = 0.0,
        cut_side = "centerline",
        corner_radius = 0.0,
        arc_tolerance = 0.0,
        allow_arcs = true,
    ))]
    fn new(
        offset_mm: f64,
        cut_side: &str,
        corner_radius: f64,
        arc_tolerance: f64,
        allow_arcs: bool,
    ) -> Self {
        PyFrameSpec {
            offset_mm,
            cut_side: cut_side.to_string(),
            corner_radius,
            arc_tolerance,
            allow_arcs,
        }
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def frame(
        part: raygeo.ops.part.Part,
        offset_mm: float = 0.0,
        cut_side: str = "centerline",
        corner_radius: float = 0.0,
        arc_tolerance: float = 0.0,
        allow_arcs: bool = True,
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Generate a rectangular frame around the part boundary.

        Creates a rectangle matching ``part.size_mm``, computes the
        total offset from offset / cut-side, applies it, and returns
        the frame as an :class:`AssemblyResult`. When
        ``corner_radius`` is positive the corners are rounded to that
        radius (clamped to what fits the frame) as exact circular arcs.

        :param part: The part whose size defines the frame.
        :param offset_mm: Total path offset distance in mm
            (default 0.0).
        :param cut_side: ``"centerline"``, ``"outside"``, or
            ``"inside"`` (default ``"centerline"``).
        :param corner_radius: Corner rounding radius in mm
            (default 0.0 = sharp corners).
        :param arc_tolerance: Maximum chord deviation in mm used only
            when ``allow_arcs`` is false (default 0.0).
        :param allow_arcs: Keep rounded corners as arcs; when false
            they are linearised (default True).
        :returns: An :class:`AssemblyResult` with the frame path.
        :raises ValueError: If the part has no size information.
        """
    "#,
    module = "raygeo.ops.assembly.frame"
)]
#[pyfunction(name = "frame")]
#[pyo3(signature = (
    part,
    offset_mm = 0.0,
    cut_side = "centerline",
    corner_radius = 0.0,
    arc_tolerance = 0.0,
    allow_arcs = true,
))]
fn frame_py(
    part: &PyPart,
    offset_mm: f64,
    cut_side: &str,
    corner_radius: f64,
    arc_tolerance: f64,
    allow_arcs: bool,
) -> PyResult<PyAssemblyResult> {
    let (ops, meta) = assemble_frame(
        part.inner.size_mm,
        offset_mm,
        cut_side,
        corner_radius,
        arc_tolerance,
        allow_arcs,
    )?;
    Ok(PyAssemblyResult::from_parts(ops, meta, None, vec![]))
}
