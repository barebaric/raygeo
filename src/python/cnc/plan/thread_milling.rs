pyo3_stub_gen::module_doc!("raygeo.cnc.plan.thread_milling", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
CNC thread-milling plan builder.

Produces a plan that thread-mills internal threads: one helical
interpolation cycle per pilot bore, all sharing the same thread
parameters. Bores come either from an explicit center list or are
detected automatically as circular island contours of the face and
validated against the thread geometry.
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::cnc::plan::drilling::HoleOrder;
use crate::cnc::plan::thread_milling::{self, ThreadMillingWorkplanOptions};
use crate::ops::assembly::thread_mill::ThreadHandedness;
use crate::ops::CoolantMode;
use crate::python::ops::part::part::PyPart;

pub(crate) fn register(plan_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = plan_mod.py();
    let m = PyModule::new(py, "thread_milling")?;
    register_functions!(m, plan_thread_milling_py,);
    plan_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.plan.thread_milling", &m)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing

    import raygeo

    def plan_thread_milling(
        part: raygeo.ops.part.Part,
        face_id: str = "",
        nominal_diameter: float = 6.0,
        pitch: float = 1.0,
        tool_diameter: float = 4.0,
        z_top: float = 0.0,
        z_bottom: float = -12.0,
        handedness: str = "rh",
        safe_z: float = 5.0,
        feed_rate: typing.Optional[float] = None,
        plunge_feed: typing.Optional[float] = None,
        spindle_rpm: typing.Optional[int] = None,
        coolant: typing.Optional[str] = None,
        holes: typing.Optional[collections.abc.Sequence[tuple[float, float]]] = None,
        hole_tolerance: float = 0.1,
        hole_order: str = "detection",
    ) -> raygeo.cnc.plan.Plan:
        """Build a plan that thread-mills internal threads on one face.

        One helical-interpolation cycle per pilot bore; all bores share
        the thread parameters. Bores come from ``holes`` when given
        (explicit centers), otherwise they are detected as circular
        island contours of the face and validated: the thread mill must
        fit into the bore, and the bore must not be oversized beyond
        the thread minor diameter (or there is nothing left to
        thread). The thread length (``z_top - z_bottom``) must be an
        integer multiple of the pitch. Bores needing different
        parameters need a separate call.

        :param part: part whose face is threaded.
        :param face_id: face to thread ("" is the default face).
        :param nominal_diameter: nominal (major) thread diameter (mm).
        :param pitch: thread pitch (lead) (mm).
        :param tool_diameter: thread mill cutter diameter (mm); must be
                              below the minor diameter.
        :param z_top: Z of the top end of the thread (default 0).
        :param z_bottom: Z of the bottom end of the thread.
        :param handedness: ``"rh"`` (default) or ``"lh"``.
        :param safe_z: safe rapid height (default 5).
        :param feed_rate: cut feed (mm/min); None leaves the plan's cut
                          state unset.
        :param plunge_feed: vertical plunge feed (mm/min); None falls
                            back to the cut feed.
        :param spindle_rpm: spindle speed (rpm).
        :param coolant: ``"flood"``, ``"mist"`` or ``"off"``; None
                        leaves the plan's coolant unset.
        :param holes: explicit pilot-bore centers ``[(x, y), ...]``;
                      None detects circular islands on the face.
        :param hole_tolerance: max distance (mm) between an island
                               contour sample and the fitted circle for
                               it to count as a bore (default 0.1).
        :param hole_order: ``"detection"`` or ``"nearest_neighbor"``.
        :returns: the thread-milling plan, one step per bore.
        """
"#,
    module = "raygeo.cnc.plan.thread_milling"
)]
#[pyfunction(name = "plan_thread_milling")]
#[pyo3(signature = (
    part,
    face_id = "",
    nominal_diameter = 6.0,
    pitch = 1.0,
    tool_diameter = 4.0,
    z_top = 0.0,
    z_bottom = -12.0,
    handedness = "rh",
    safe_z = 5.0,
    feed_rate = None,
    plunge_feed = None,
    spindle_rpm = None,
    coolant = None,
    holes = None,
    hole_tolerance = 0.1,
    hole_order = "detection",
))]
pub fn plan_thread_milling_py(
    part: &Bound<'_, PyPart>,
    face_id: &str,
    nominal_diameter: f64,
    pitch: f64,
    tool_diameter: f64,
    z_top: f64,
    z_bottom: f64,
    handedness: &str,
    safe_z: f64,
    feed_rate: Option<f64>,
    plunge_feed: Option<f64>,
    spindle_rpm: Option<u32>,
    coolant: Option<&str>,
    holes: Option<Vec<(f64, f64)>>,
    hole_tolerance: f64,
    hole_order: &str,
) -> PyResult<Py<super::PyPlan>> {
    let handedness = if handedness.eq_ignore_ascii_case("lh") {
        ThreadHandedness::Lh
    } else {
        ThreadHandedness::Rh
    };
    let coolant = match coolant {
        Some("flood") => Some(CoolantMode::Flood),
        Some("mist") => Some(CoolantMode::Mist),
        Some("off") => Some(CoolantMode::Off),
        _ => None,
    };
    let hole_order = if hole_order == "nearest_neighbor" {
        HoleOrder::NearestNeighbor
    } else {
        HoleOrder::DetectionOrder
    };
    let opts = ThreadMillingWorkplanOptions {
        nominal_diameter,
        pitch,
        tool_diameter,
        z_top,
        z_bottom,
        handedness,
        safe_z,
        feed_rate,
        plunge_feed,
        spindle_rpm,
        coolant,
        holes: holes.map(|pts| {
            pts.into_iter()
                .map(|(x, y)| crate::geo::types::Point::new(x, y))
                .collect()
        }),
        hole_tolerance,
        hole_order,
    };
    let plan = thread_milling::plan_thread_milling(
        &part.borrow().inner,
        face_id,
        &opts,
    )?;
    Py::new(part.py(), super::PyPlan { inner: plan })
}
