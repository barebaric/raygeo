pyo3_stub_gen::module_doc!("raygeo.cnc.plan.tapping", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
CNC rigid-tapping plan builder.

Produces a plan that rigid-taps internal threads: one spindle-
synchronized cycle per pilot bore, all sharing the same thread
parameters. Bores come either from an explicit center list or are
detected automatically as circular island contours of the face and
validated against the thread geometry. Tapping feeds derive from the
thread itself — every synchronized move travels pitch * rpm mm/min —
so the spindle speed is mandatory.
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::cnc::plan::drilling::HoleOrder;
use crate::cnc::plan::tapping::{self, TappingWorkplanOptions};
use crate::ops::assembly::thread_mill::ThreadHandedness;
use crate::ops::CoolantMode;
use crate::python::ops::part::part::PyPart;

pub(crate) fn register(plan_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = plan_mod.py();
    let m = PyModule::new(py, "tapping")?;
    register_functions!(m, plan_tapping_py,);
    plan_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.plan.tapping", &m)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing

    import raygeo

    def plan_tapping(
        part: raygeo.ops.part.Part,
        face_id: str = "",
        nominal_diameter: float = 6.0,
        pitch: float = 1.0,
        target_z: float = -12.0,
        z_surface: float = 0.0,
        rapid_z: float = 1.0,
        safe_z: float = 5.0,
        handedness: str = "rh",
        peck: typing.Optional[float] = None,
        dwell_ms: float = 0.0,
        spindle_rpm: typing.Optional[int] = None,
        coolant: typing.Optional[str] = None,
        holes: typing.Optional[collections.abc.Sequence[tuple[float, float]]] = None,
        hole_tolerance: float = 0.1,
        hole_order: str = "detection",
    ) -> raygeo.cnc.plan.Plan:
        """Build a plan that rigid-taps internal threads on one face.

        One spindle-synchronized cycle per pilot bore; all bores share
        the thread parameters. Bores come from ``holes`` when given
        (explicit centers), otherwise they are detected as circular
        island contours of the face and validated: a bore at or above
        the nominal diameter has nothing left to tap. The synchronized
        moves travel ``pitch * spindle_rpm`` mm/min, so
        ``spindle_rpm`` is mandatory. Bores needing different
        parameters need a separate call.

        :param part: part whose face is tapped.
        :param face_id: face to tap ("" is the default face).
        :param nominal_diameter: nominal (major) thread diameter (mm).
        :param pitch: thread pitch (lead) (mm).
        :param target_z: depth of the thread (default -12).
        :param z_surface: surface Z the holes start at (default 0).
        :param rapid_z: rapid plane above the surface (default 1).
        :param safe_z: safe rapid height (default 5).
        :param handedness: ``"rh"`` (default) or ``"lh"``.
        :param peck: optional peck increment; full synchronized retract
                     between pecks (default None).
        :param dwell_ms: dwell at the bottom before the spindle
                         reversal (default 0).
        :param spindle_rpm: spindle speed (rpm); required.
        :param coolant: ``"flood"``, ``"mist"`` or ``"off"``; None
                        leaves the plan's coolant unset.
        :param holes: explicit pilot-bore centers ``[(x, y), ...]``;
                      None detects circular islands on the face.
        :param hole_tolerance: max distance (mm) between an island
                               contour sample and the fitted circle for
                               it to count as a bore (default 0.1).
        :param hole_order: ``"detection"`` or ``"nearest_neighbor"``.
        :returns: the tapping plan, one step per bore.
        """
"#,
    module = "raygeo.cnc.plan.tapping"
)]
#[pyfunction(name = "plan_tapping")]
#[pyo3(signature = (
    part,
    face_id = "",
    nominal_diameter = 6.0,
    pitch = 1.0,
    target_z = -12.0,
    z_surface = 0.0,
    rapid_z = 1.0,
    safe_z = 5.0,
    handedness = "rh",
    peck = None,
    dwell_ms = 0.0,
    spindle_rpm = None,
    coolant = None,
    holes = None,
    hole_tolerance = 0.1,
    hole_order = "detection",
))]
pub fn plan_tapping_py(
    part: &Bound<'_, PyPart>,
    face_id: &str,
    nominal_diameter: f64,
    pitch: f64,
    target_z: f64,
    z_surface: f64,
    rapid_z: f64,
    safe_z: f64,
    handedness: &str,
    peck: Option<f64>,
    dwell_ms: f64,
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
    let opts = TappingWorkplanOptions {
        nominal_diameter,
        pitch,
        handedness,
        z_surface,
        target_z,
        rapid_z,
        safe_z,
        peck,
        dwell_ms,
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
    let plan = tapping::plan_tapping(&part.borrow().inner, face_id, &opts)?;
    Py::new(part.py(), super::PyPlan { inner: plan })
}
