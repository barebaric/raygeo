use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::ops::assembly::profile::{self, ProfileKind, ProfileSpec};
use crate::ops::assembly::Tracelet;
use crate::ops::state::State;
use crate::ops::types::CutDirection;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::types::Point3D;

fn check_cancel() -> bool {
    let rc = unsafe { pyo3::ffi::PyErr_CheckSignals() };
    if rc == -1 {
        unsafe { pyo3::ffi::PyErr_Clear() };
        true
    } else {
        false
    }
}

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "profile")?;
    m.add_function(pyo3::wrap_pyfunction!(profile_outer_py, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(profile_inner_py, m.clone())?)?;
    m.add_class::<PyProfileSpec>()?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.profile", &m)?;

    Ok(())
}

/// Parameters for the adaptive-profile assembler.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.assembly.profile",
    name = "ProfileSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyProfileSpec {
    /// ``"inner"`` or ``"outer"``.
    #[pyo3(get)]
    pub kind: String,
    #[pyo3(get)]
    pub tool_radius: f64,
    #[pyo3(get)]
    pub step_over: f64,
    #[pyo3(get)]
    pub step_length: f64,
    #[pyo3(get)]
    pub target_z: f64,
    #[pyo3(get)]
    pub safe_z: f64,
    #[pyo3(get)]
    pub wall_margin: f64,
    #[pyo3(get)]
    pub stock_to_leave: f64,
    /// ``"cw"`` or ``"ccw"``.
    #[pyo3(get)]
    pub cut_direction: String,
    /// Optional override start ``(x, y)``.
    #[pyo3(get)]
    pub start_pos: Option<(f64, f64)>,
    #[pyo3(get)]
    pub tolerance: f64,
    #[pyo3(get)]
    pub expansion_batch_size: usize,
    #[pyo3(get)]
    pub engagement_area_threshold: f64,
    #[pyo3(get)]
    pub engagement_angle_threshold: f64,
    /// Factor (0–1) by which feed is reduced on over-engagement.
    #[pyo3(get)]
    pub feed_reduction_factor: f64,
    /// Optional path to write a binary trace file.
    #[pyo3(get)]
    pub trace_path: Option<String>,
}

impl PyProfileSpec {
    pub fn into_core(self) -> ProfileSpec {
        let kind = match self.kind.as_str() {
            "outer" => ProfileKind::Outer,
            _ => ProfileKind::Inner,
        };
        let cd = match self.cut_direction.as_str() {
            "cw" => CutDirection::Cw,
            _ => CutDirection::Ccw,
        };
        ProfileSpec {
            kind,
            tool_radius: self.tool_radius,
            step_over: self.step_over,
            step_length: self.step_length,
            target_z: self.target_z,
            safe_z: self.safe_z,
            wall_margin: self.wall_margin,
            stock_to_leave: self.stock_to_leave,
            cut_direction: cd,
            start_pos: self
                .start_pos
                .map(|(x, y)| Point3D::new(x, y, self.target_z)),
            tolerance: self.tolerance,
            expansion_batch_size: self.expansion_batch_size,
            cancel_check: None,
            engagement_area_threshold: self.engagement_area_threshold,
            engagement_angle_threshold: self.engagement_angle_threshold,
            feed_reduction_factor: self.feed_reduction_factor,
            trace_path: self.trace_path.map(std::path::PathBuf::from),
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyProfileSpec {
    #[new]
    #[pyo3(signature = (
        kind = "inner",
        tool_radius = 3.0,
        step_over = 1.5,
        step_length = 0.6,
        target_z = -5.0,
        safe_z = 2.0,
        wall_margin = 0.0,
        stock_to_leave = 0.0,
        cut_direction = "ccw",
        start_pos = None,
        tolerance = 0.1,
        expansion_batch_size = 20,
        engagement_area_threshold = 0.0,
        engagement_angle_threshold = std::f64::consts::PI,
        feed_reduction_factor = 0.5,
        trace_path = None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        kind: &str,
        tool_radius: f64,
        step_over: f64,
        step_length: f64,
        target_z: f64,
        safe_z: f64,
        wall_margin: f64,
        stock_to_leave: f64,
        cut_direction: &str,
        start_pos: Option<(f64, f64)>,
        tolerance: f64,
        expansion_batch_size: usize,
        engagement_area_threshold: f64,
        engagement_angle_threshold: f64,
        feed_reduction_factor: f64,
        trace_path: Option<String>,
    ) -> Self {
        PyProfileSpec {
            kind: kind.to_string(),
            tool_radius,
            step_over,
            step_length,
            target_z,
            safe_z,
            wall_margin,
            stock_to_leave,
            cut_direction: cut_direction.to_string(),
            start_pos,
            tolerance,
            expansion_batch_size,
            engagement_area_threshold,
            engagement_angle_threshold,
            feed_reduction_factor,
            trace_path,
        }
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def profile_outer(
        part: raygeo.ops.part.Part,
        tool_radius: float,
        step_over: float,
        step_length: float,
        target_z: float,
        safe_z: float,
        wall_margin: float,
        cut_feed_rate: int,
        cut_power: float,
        start_pos: tuple[float, float] | None = None,
        cut_direction: str = "ccw",
        stock_to_leave: float = 0.0,
        engagement_area_threshold: float = 0.0,
        engagement_angle_threshold: float = 3.141592653589793,
        trace_path: str | None = None,
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Profile the outer boundary of a pocket.

        Walks a tool around the grown boundary (offset outward by tool
        radius).  The path stays approximately one tool radius outside
        the original boundary, removing any excess stock on the outer
        side.  Returns an :class:`AssemblyResult` with the profiling
        move sequence.

        :param part: The part whose ``cleared`` field tracks accumulated
                     workpiece state and whose geometry defines the
                     pocket boundary.
        :param tool_radius: Tool radius in mm.
        :param step_over: Radial step-over between passes (mm).
        :param step_length: Forward step length in mm.
        :param target_z: Cutting depth (Z).
        :param safe_z: Safe (rapid) Z height.
        :param wall_margin: Extra distance to keep from the wall (mm).
        :param cut_feed_rate: Feed rate in mm/min.
        :param cut_power: Spindle power (0.0–1.0).
        :param start_pos: Optional override start ``(x, y)`` (default: first boundary vertex).
        :param cut_direction: ``"cw"`` or ``"ccw"`` (default ``"ccw"``).
        :param stock_to_leave: Stock left on wall for rough pass (mm, default 0.0).
        :param engagement_area_threshold: Overengagement area threshold (mm², 0 = auto).
        :param engagement_angle_threshold: Overengagement angle threshold (rad, default π).
        :param trace_path: Optional path to write a binary trace file (default None).
        :returns: An :class:`AssemblyResult` with the profiling path.
        """
    "#,
    module = "raygeo.ops.assembly.profile"
)]
#[pyfunction(name = "profile_outer")]
#[pyo3(signature = (
    part,
    tool_radius,
    step_over,
    step_length,
    target_z,
    safe_z,
    wall_margin,
    cut_feed_rate,
    cut_power,
    start_pos = None,
    cut_direction = "ccw",
    stock_to_leave = 0.0,
    engagement_area_threshold = 0.0,
    engagement_angle_threshold = std::f64::consts::PI,
    trace_path = None,
))]
#[allow(clippy::too_many_arguments)]
fn profile_outer_py(
    part: &mut crate::python::ops::part::part::PyPart,
    tool_radius: f64,
    step_over: f64,
    step_length: f64,
    target_z: f64,
    safe_z: f64,
    wall_margin: f64,
    cut_feed_rate: i32,
    cut_power: f64,
    start_pos: Option<(f64, f64)>,
    cut_direction: &str,
    stock_to_leave: f64,
    engagement_area_threshold: f64,
    engagement_angle_threshold: f64,
    trace_path: Option<String>,
) -> PyResult<PyAssemblyResult> {
    use std::path::PathBuf;

    let cd = match cut_direction.to_ascii_lowercase().as_str() {
        "cw" => CutDirection::Cw,
        _ => CutDirection::Ccw,
    };

    let opts = ProfileSpec {
        kind: ProfileKind::Outer,
        tool_radius,
        step_over,
        step_length,
        target_z,
        safe_z,
        wall_margin,
        stock_to_leave,
        cut_direction: cd,
        start_pos: start_pos.map(|(x, y)| Point3D::new(x, y, target_z)),
        tolerance: 0.1,
        expansion_batch_size: 20,
        cancel_check: Some(check_cancel),
        engagement_area_threshold,
        engagement_angle_threshold,
        feed_reduction_factor: 0.5,
        trace_path: trace_path.map(PathBuf::from),
    };

    let cut_state = State {
        power: cut_power,
        feed_rate: Some(cut_feed_rate),
        ..Default::default()
    };

    let mut trace = Tracelet::new();
    let face = part.inner.face_mut("");
    let meta = profile::profile_outer(face, &mut trace, &opts, &cut_state)?;
    let events = trace.drain();
    let attrs = trace.attrs().cloned();
    let ops = trace.into_ops();
    Ok(PyAssemblyResult::from_parts(ops, meta, attrs, events))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def profile_inner(
        part: raygeo.ops.part.Part,
        tool_radius: float = 3.0,
        step_over: float = 1.5,
        step_length: float = 0.6,
        target_z: float = -5.0,
        safe_z: float = 2.0,
        wall_margin: float = 0.0,
        stock_to_leave: float = 0.0,
        cut_feed_rate: int = 1000,
        cut_power: float = 0.0,
        start_pos: tuple[float, float] | None = None,
        cut_direction: str = "ccw",
        engagement_area_threshold: float = 0.0,
        engagement_angle_threshold: float = 3.141592653589793,
        trace_path: str | None = None,
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Profile the inner boundary of a pocket, around islands.

        Walks a tool around the inset boundary (offset inward by tool
        radius) and around accessible islands, so that the tool clears
        the material along the pocket walls and around each island.
        Returns an :class:`AssemblyResult` with the profiling move
        sequence.

        :param part: The part whose ``cleared`` field tracks accumulated
                     workpiece state and whose geometry defines the
                     pocket boundary and islands.
        :param tool_radius: Tool radius in mm.
        :param step_over: Radial step-over between passes (mm).
        :param step_length: Forward step length in mm.
        :param target_z: Cutting depth (Z).
        :param safe_z: Safe (rapid) Z height.
        :param wall_margin: Extra distance to keep from the wall (mm).
        :param stock_to_leave: Stock left on wall for rough pass (mm, default 0.0).
        :param cut_feed_rate: Feed rate in mm/min.
        :param cut_power: Spindle power (0.0–1.0).
        :param start_pos: Optional override start ``(x, y)`` (default: first boundary vertex).
        :param cut_direction: ``"cw"`` or ``"ccw"`` (default ``"ccw"``).
        :param engagement_area_threshold: Overengagement area threshold (mm², 0 = auto).
        :param engagement_angle_threshold: Overengagement angle threshold (rad, default π).
        :param trace_path: Optional path to write a binary trace file (default None).
        :returns: An :class:`AssemblyResult` with the profiling path.
        """
    "#,
    module = "raygeo.ops.assembly.profile"
)]
#[pyfunction(name = "profile_inner")]
#[pyo3(signature = (
    part,
    tool_radius = 3.0,
    step_over = 1.5,
    step_length = 0.6,
    target_z = -5.0,
    safe_z = 2.0,
    wall_margin = 0.0,
    stock_to_leave = 0.0,
    cut_feed_rate = 1000,
    cut_power = 0.0,
    start_pos = None,
    cut_direction = "ccw",
    engagement_area_threshold = 0.0,
    engagement_angle_threshold = std::f64::consts::PI,
    trace_path = None,
))]
#[allow(clippy::too_many_arguments)]
fn profile_inner_py(
    part: &mut crate::python::ops::part::part::PyPart,
    tool_radius: f64,
    step_over: f64,
    step_length: f64,
    target_z: f64,
    safe_z: f64,
    wall_margin: f64,
    stock_to_leave: f64,
    cut_feed_rate: i32,
    cut_power: f64,
    start_pos: Option<(f64, f64)>,
    cut_direction: &str,
    engagement_area_threshold: f64,
    engagement_angle_threshold: f64,
    trace_path: Option<String>,
) -> PyResult<PyAssemblyResult> {
    use std::path::PathBuf;

    let cd = match cut_direction.to_ascii_lowercase().as_str() {
        "cw" => CutDirection::Cw,
        _ => CutDirection::Ccw,
    };

    let opts = ProfileSpec {
        kind: ProfileKind::Inner,
        tool_radius,
        step_over,
        step_length,
        target_z,
        safe_z,
        wall_margin,
        stock_to_leave,
        cut_direction: cd,
        start_pos: start_pos.map(|(x, y)| Point3D::new(x, y, target_z)),
        tolerance: 0.1,
        expansion_batch_size: 20,
        cancel_check: Some(check_cancel),
        engagement_area_threshold,
        engagement_angle_threshold,
        feed_reduction_factor: 0.5,
        trace_path: trace_path.map(PathBuf::from),
    };

    let cut_state = State {
        power: cut_power,
        feed_rate: Some(cut_feed_rate),
        ..Default::default()
    };

    let mut trace = Tracelet::new();
    let face = part.inner.face_mut("");
    let meta = profile::profile_inner(face, &mut trace, &opts, &cut_state)?;
    let events = trace.drain();
    let attrs = trace.attrs().cloned();
    let ops = trace.into_ops();
    Ok(PyAssemblyResult::from_parts(ops, meta, attrs, events))
}
