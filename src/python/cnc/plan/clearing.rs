pyo3_stub_gen::module_doc!("raygeo.cnc.plan.clearing", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
CNC clearing plan builder.

Produces a :class:`~raygeo.cnc.plan.Plan` organised as a BFS traversal
of the pocket's region/passage graph.
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::cnc::plan::clearing::{self, ClearingWorkplanOptions};
use crate::python::ops::part::part::PyPart;

pub(crate) fn register(plan_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = plan_mod.py();
    let m = PyModule::new(py, "clearing")?;
    register_functions!(m, plan_clearing_py,);
    m.add_class::<PyClearingWorkplanOptions>()?;
    plan_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.plan.clearing", &m)?;

    Ok(())
}

/// The tool and machining parameters :func:`plan_clearing` plans from.
///
/// The pocket boundary and islands are read from the ``Part`` — this
/// class carries only the tool and machining parameters.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.cnc.plan.clearing",
    name = "ClearingWorkplanOptions",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyClearingWorkplanOptions {
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
    pub safe_margin: f64,
    #[pyo3(get)]
    pub stock_to_leave: f64,
    #[pyo3(get)]
    pub plunge_pitch: f64,
    #[pyo3(get)]
    pub angular_step: f64,
    #[pyo3(get)]
    pub area_tolerance: f64,
    #[pyo3(get)]
    pub max_deflection_deg: f64,
    #[pyo3(get)]
    pub finishing: bool,
    /// Steepest entry descent in degrees; ``<= 0`` keeps the planner's
    /// historical entry behaviour.
    #[pyo3(get)]
    pub max_entry_angle_deg: f64,
    /// Cut feed rate (mm/min) the plan's moves carry; ``None`` leaves
    /// the cut state unset, for embedders that map feeds themselves.
    #[pyo3(get)]
    pub feed_rate: Option<f64>,
    /// Vertical plunge feed (mm/min) for descents into material;
    /// ``None`` falls back to the cut feed.
    #[pyo3(get)]
    pub plunge_feed: Option<f64>,
}

impl PyClearingWorkplanOptions {
    pub fn into_core(self) -> ClearingWorkplanOptions {
        ClearingWorkplanOptions {
            tool_radius: self.tool_radius,
            step_over: self.step_over,
            step_length: self.step_length,
            target_z: self.target_z,
            safe_z: self.safe_z,
            wall_margin: self.wall_margin,
            safe_margin: self.safe_margin,
            stock_to_leave: self.stock_to_leave,
            plunge_pitch: self.plunge_pitch,
            angular_step: self.angular_step,
            area_tolerance: self.area_tolerance,
            max_deflection_deg: self.max_deflection_deg,
            finishing: self.finishing,
            max_entry_angle_deg: self.max_entry_angle_deg,
            feed_rate: self.feed_rate,
            plunge_feed: self.plunge_feed,
        }
    }

    pub fn from_core(opts: &ClearingWorkplanOptions) -> Self {
        PyClearingWorkplanOptions {
            tool_radius: opts.tool_radius,
            step_over: opts.step_over,
            step_length: opts.step_length,
            target_z: opts.target_z,
            safe_z: opts.safe_z,
            wall_margin: opts.wall_margin,
            safe_margin: opts.safe_margin,
            stock_to_leave: opts.stock_to_leave,
            plunge_pitch: opts.plunge_pitch,
            angular_step: opts.angular_step,
            area_tolerance: opts.area_tolerance,
            max_deflection_deg: opts.max_deflection_deg,
            finishing: opts.finishing,
            max_entry_angle_deg: opts.max_entry_angle_deg,
            feed_rate: opts.feed_rate,
            plunge_feed: opts.plunge_feed,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyClearingWorkplanOptions {
    /// :param tool_radius: Tool radius in mm.
    /// :param step_over: Radial step-over between passes (mm).
    /// :param step_length: Forward step length in mm.
    /// :param target_z: Cutting depth (Z).
    /// :param safe_z: Safe (rapid) Z height.
    /// :param wall_margin: Extra distance to keep from the walls (mm).
    /// :param safe_margin: Safety margin around entries (mm).
    /// :param stock_to_leave: Stock left on the walls (mm).
    /// :param plunge_pitch: Drop per turn on helical entries (mm).
    /// :param angular_step: Step per helix turn (radians).
    /// :param area_tolerance: Smallest area worth clearing (mm²).
    /// :param max_deflection_deg: Steepest deflection from the cut
    ///                             direction (degrees).
    /// :param finishing: Add the finishing wall profile.
    /// :param max_entry_angle_deg: Steepest entry descent (degrees).
    /// :param feed_rate: Cut feed rate (mm/min).
    /// :param plunge_feed: Vertical plunge feed (mm/min).
    #[new]
    #[pyo3(signature = (
        tool_radius = 3.0,
        step_over = 2.0,
        step_length = 0.6,
        target_z = -5.0,
        safe_z = 2.0,
        wall_margin = 0.0,
        safe_margin = 1.0,
        stock_to_leave = 0.0,
        plunge_pitch = 1.0,
        angular_step = 0.1,
        area_tolerance = 1.0,
        max_deflection_deg = 30.0,
        finishing = false,
        max_entry_angle_deg = 0.0,
        feed_rate = None,
        plunge_feed = None,
    ))]
    fn new(
        tool_radius: f64,
        step_over: f64,
        step_length: f64,
        target_z: f64,
        safe_z: f64,
        wall_margin: f64,
        safe_margin: f64,
        stock_to_leave: f64,
        plunge_pitch: f64,
        angular_step: f64,
        area_tolerance: f64,
        max_deflection_deg: f64,
        finishing: bool,
        max_entry_angle_deg: f64,
        feed_rate: Option<f64>,
        plunge_feed: Option<f64>,
    ) -> Self {
        PyClearingWorkplanOptions {
            tool_radius,
            step_over,
            step_length,
            target_z,
            safe_z,
            wall_margin,
            safe_margin,
            stock_to_leave,
            plunge_pitch,
            angular_step,
            area_tolerance,
            max_deflection_deg,
            finishing,
            max_entry_angle_deg,
            feed_rate,
            plunge_feed,
        }
    }

    /// Options carrying the planner's canonical scales for
    /// ``tool_radius``: a fifth-of-a-radius step length, a
    /// fifth-of-a-radius safe margin, a plunge pitch of a third of the
    /// radius, and the planner's angular and area tuning (0.1 rad
    /// angular steps, a 1.0 mm² area tolerance, 30° max deflection).
    /// The per-operation fields keep their neutral defaults; set
    /// ``step_over``, ``wall_margin``, ``stock_to_leave``,
    /// ``finishing``, ``max_entry_angle_deg`` and the feeds on the
    /// result.
    ///
    /// :param tool_radius: Tool radius in mm.
    /// :param step_over: Radial step-over between passes (mm).
    /// :param target_z: Cutting depth (Z).
    /// :param safe_z: Safe (rapid) Z height.
    #[staticmethod]
    #[pyo3(name = "for_tool")]
    fn for_tool(
        tool_radius: f64,
        step_over: f64,
        target_z: f64,
        safe_z: f64,
    ) -> Self {
        Self::from_core(&ClearingWorkplanOptions::for_tool(
            tool_radius,
            step_over,
            target_z,
            safe_z,
        ))
    }
}

#[allow(clippy::too_many_arguments)]
#[gen_stub_pyfunction(module = "raygeo.cnc.plan.clearing")]
#[pyfunction(name = "plan_clearing")]
#[pyo3(signature = (
    part,
    face_id = "",
    tool_radius = 3.0,
    step_over = 2.0,
    step_length = 0.6,
    target_z = -5.0,
    safe_z = 2.0,
    wall_margin = 0.0,
    safe_margin = 1.0,
    stock_to_leave = 0.0,
    plunge_pitch = 1.0,
    angular_step = 0.1,
    area_tolerance = 1.0,
    max_deflection_deg = 30.0,
    finishing = false,
    feed_rate = None,
    plunge_feed = None,
    max_entry_angle_deg = 0.0,
))]
pub fn plan_clearing_py(
    part: &Bound<'_, PyPart>,
    face_id: &str,
    tool_radius: f64,
    step_over: f64,
    step_length: f64,
    target_z: f64,
    safe_z: f64,
    wall_margin: f64,
    safe_margin: f64,
    stock_to_leave: f64,
    plunge_pitch: f64,
    angular_step: f64,
    area_tolerance: f64,
    max_deflection_deg: f64,
    finishing: bool,
    feed_rate: Option<f64>,
    plunge_feed: Option<f64>,
    max_entry_angle_deg: f64,
) -> PyResult<Py<super::PyPlan>> {
    let opts = ClearingWorkplanOptions {
        tool_radius,
        step_over,
        step_length,
        target_z,
        safe_z,
        wall_margin,
        safe_margin,
        stock_to_leave,
        plunge_pitch,
        angular_step,
        area_tolerance,
        max_deflection_deg,
        finishing,
        feed_rate,
        plunge_feed,
        max_entry_angle_deg,
    };

    let plan = clearing::plan_clearing(&part.borrow().inner, face_id, &opts)?;
    Py::new(part.py(), super::PyPlan { inner: plan })
}
