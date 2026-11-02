pyo3_stub_gen::module_doc!("raygeo.cnc.plan.profile", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
CNC profile plan builder.

Produces a one-step :class:`~raygeo.cnc.plan.Plan` that profiles the
face boundary: the inset (inner) or grown (outer) offset, walked once
at the target depth.
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::cnc::plan::profile::{self, ProfileWorkplanOptions};
use crate::ops::assembly::profile::ProfileKind;
use crate::ops::types::CutDirection;
use crate::python::ops::part::part::PyPart;

pub(crate) fn register(plan_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = plan_mod.py();
    let m = PyModule::new(py, "profile")?;
    register_functions!(m, plan_profile_py,);
    plan_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.plan.profile", &m)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[gen_stub_pyfunction(
    python = r#"
    import typing

    import raygeo

    def plan_profile(
        part: raygeo.ops.part.Part,
        face_id: str = "",
        side: str = "inner",
        tool_radius: float = 3.0,
        step_over: float = 2.0,
        step_length: float = 0.6,
        target_z: float = -5.0,
        safe_z: float = 2.0,
        wall_margin: float = 0.0,
        stock_to_leave: float = 0.0,
        cut_direction: str = "ccw",
        ramp_angle_deg: float = 0.0,
        tolerance: float = 0.1,
        feed_rate: typing.Optional[float] = None,
        plunge_feed: typing.Optional[float] = None,
    ) -> raygeo.cnc.plan.Plan:
        """Build a plan that profiles a face boundary.
        The step walks the inset boundary (side "inner", the pocket side)
        or the grown boundary (side "outer", around the part) once at the
        target depth, honouring islands on the inner side. Curve-true
        face geometry is followed exactly, with arcs emitted as arcs;
        polygon-only parts take the chord path.

        With `ramp_angle_deg` above 0 and curve-true face geometry, the
        plan prepends a ramp-entry step that descends along the profile
        loop at that angle, so the tool enters the material gradually
        instead of plunging.

        :param part: part whose face is profiled.
        :param face_id: face to profile ("" is the default face).
        :param side: ``"inner"`` or ``"outer"``.
        :param tool_radius: tool radius (mm).
        :param step_over: radial step-over between passes (mm); at most
                          the tool diameter.
        :param step_length: forward step length (mm).
        :param target_z: cutting depth (mm).
        :param safe_z: safe rapid height (mm).
        :param wall_margin: extra distance to keep from the wall (mm).
        :param stock_to_leave: stock left on the wall (mm).
        :param cut_direction: ``"ccw"`` (wall on the left, climb for a
                              clockwise spindle) or ``"cw"``.
        :param ramp_angle_deg: steepest entry descent along the loop
                               (degrees); 0 plunges straight down.
        :param tolerance: boundary-following tolerance (mm).
        :param feed_rate: cut feed (mm/min); None leaves the plan's cut
                          state unset.
        :param plunge_feed: vertical plunge feed (mm/min); None falls
                            back to the cut feed.
        :returns: the profile plan.
        """
"#,
    module = "raygeo.cnc.plan.profile"
)]
#[pyfunction(name = "plan_profile")]
#[pyo3(signature = (
    part,
    face_id = "",
    side = "inner",
    tool_radius = 3.0,
    step_over = 2.0,
    step_length = 0.6,
    target_z = -5.0,
    safe_z = 2.0,
    wall_margin = 0.0,
    stock_to_leave = 0.0,
    cut_direction = "ccw",
    ramp_angle_deg = 0.0,
    tolerance = 0.1,
    feed_rate = None,
    plunge_feed = None,
))]
pub fn plan_profile_py(
    part: &Bound<'_, PyPart>,
    face_id: &str,
    side: &str,
    tool_radius: f64,
    step_over: f64,
    step_length: f64,
    target_z: f64,
    safe_z: f64,
    wall_margin: f64,
    stock_to_leave: f64,
    cut_direction: &str,
    ramp_angle_deg: f64,
    tolerance: f64,
    feed_rate: Option<f64>,
    plunge_feed: Option<f64>,
) -> PyResult<Py<super::PyPlan>> {
    let kind = match side {
        "outer" => ProfileKind::Outer,
        _ => ProfileKind::Inner,
    };
    let cd = match cut_direction.to_ascii_lowercase().as_str() {
        "cw" => CutDirection::Cw,
        _ => CutDirection::Ccw,
    };
    let opts = ProfileWorkplanOptions {
        side: kind,
        tool_radius,
        step_over,
        step_length,
        target_z,
        safe_z,
        wall_margin,
        stock_to_leave,
        cut_direction: cd,
        ramp_angle: ramp_angle_deg.to_radians(),
        tolerance,
        feed_rate,
        plunge_feed,
    };
    let plan = profile::plan_profile(&part.borrow().inner, face_id, &opts)?;
    Py::new(part.py(), super::PyPlan { inner: plan })
}
