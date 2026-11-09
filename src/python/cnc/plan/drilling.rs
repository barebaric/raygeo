pyo3_stub_gen::module_doc!("raygeo.cnc.plan.drilling", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
CNC drilling plan builder.

Produces a plan that drills holes: one cycle per hole, all sharing
the same parameters. Holes come either from an explicit center list
or are detected automatically as circular island contours of the
face. Cycles are expanded into explicit moves and wrapped in
DRILL_CYCLE sections so encoders can substitute canned cycles.
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::cnc::plan::drilling::{
    self, DrillCycleKind, DrillingWorkplanOptions, HoleOrder,
};
use crate::ops::CoolantMode;
use crate::python::ops::part::part::PyPart;

pub(crate) fn register(plan_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = plan_mod.py();
    let m = PyModule::new(py, "drilling")?;
    register_functions!(m, plan_drilling_py,);
    plan_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.plan.drilling", &m)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing

    import raygeo

    def plan_drilling(
        part: raygeo.ops.part.Part,
        face_id: str = "",
        cycle: str = "plain",
        diameter: float = 5.0,
        target_z: float = -10.0,
        tip_angle_deg: float = 118.0,
        z_surface: float = 0.0,
        rapid_z: float = 1.0,
        safe_z: float = 5.0,
        peck_increment: float = 1.0,
        chip_break_retract: float = 0.5,
        dwell_ms: float = 0.0,
        feed_rate: typing.Optional[float] = None,
        plunge_feed: typing.Optional[float] = None,
        spindle_rpm: typing.Optional[int] = None,
        coolant: typing.Optional[str] = None,
        holes: typing.Optional[collections.abc.Sequence[tuple[float, float]]] = None,
        hole_tolerance: float = 0.1,
        hole_order: str = "detection",
    ) -> raygeo.cnc.plan.Plan:
        """Build a plan that drills holes on one face.

        One cycle per hole; all holes share the parameters. Holes come
        from ``holes`` when given (explicit centers, in solid material
        — typical for spot drilling), otherwise they are detected as
        circular island contours of the face (existing bores, e.g. to
        re-drill or countersink). Islands smaller than the drill
        diameter are skipped in detection mode. ``target_z`` is the
        full-diameter depth; the tip descends deeper by the tip
        compensation ``(diameter / 2) / tan(tip_angle / 2)``. Holes
        needing different depths or parameters need a separate call.

        Cycles: ``"plain"`` single plunge, ``"spot"`` shallow marking
        cycle, ``"peck"`` full-retract pecking (G83), ``"chip_break"``
        chip-break pecking (G73).

        :param part: part whose face is drilled.
        :param face_id: face to drill ("" is the default face).
        :param cycle: ``"plain"``, ``"spot"``, ``"peck"`` or
                      ``"chip_break"``.
        :param diameter: drill (and hole) diameter (mm).
        :param target_z: full-diameter depth (mm).
        :param tip_angle_deg: drill point angle in degrees (default 118).
        :param z_surface: surface Z the holes start at (default 0).
        :param rapid_z: rapid plane above the surface (default 1).
        :param safe_z: safe rapid height (default 5).
        :param peck_increment: axial depth per peck (mm); pecking
                               cycles only.
        :param chip_break_retract: chip-break retract distance (mm);
                                   chip-break only.
        :param dwell_ms: dwell at the bottom of every plunge (ms).
        :param feed_rate: cut feed (mm/min); None leaves the plan's cut
                          state unset.
        :param plunge_feed: vertical plunge feed (mm/min); None falls
                            back to the cut feed.
        :param spindle_rpm: spindle speed (rpm).
        :param coolant: ``"flood"``, ``"mist"`` or ``"off"``; None
                        leaves the plan's coolant unset.
        :param holes: explicit hole centers ``[(x, y), ...]``; None
                      detects circular islands on the face.
        :param hole_tolerance: max distance (mm) between an island
                               contour sample and the fitted circle for
                               it to count as a hole (default 0.1).
        :param hole_order: ``"detection"`` or ``"nearest_neighbor"``.
        :returns: the drilling plan, one step per hole.
        """
"#,
    module = "raygeo.cnc.plan.drilling"
)]
#[pyfunction(name = "plan_drilling")]
#[pyo3(signature = (
    part,
    face_id = "",
    cycle = "plain",
    diameter = 5.0,
    target_z = -10.0,
    tip_angle_deg = 118.0,
    z_surface = 0.0,
    rapid_z = 1.0,
    safe_z = 5.0,
    peck_increment = 1.0,
    chip_break_retract = 0.5,
    dwell_ms = 0.0,
    feed_rate = None,
    plunge_feed = None,
    spindle_rpm = None,
    coolant = None,
    holes = None,
    hole_tolerance = 0.1,
    hole_order = "detection",
))]
pub fn plan_drilling_py(
    part: &Bound<'_, PyPart>,
    face_id: &str,
    cycle: &str,
    diameter: f64,
    target_z: f64,
    tip_angle_deg: f64,
    z_surface: f64,
    rapid_z: f64,
    safe_z: f64,
    peck_increment: f64,
    chip_break_retract: f64,
    dwell_ms: f64,
    feed_rate: Option<f64>,
    plunge_feed: Option<f64>,
    spindle_rpm: Option<u32>,
    coolant: Option<&str>,
    holes: Option<Vec<(f64, f64)>>,
    hole_tolerance: f64,
    hole_order: &str,
) -> PyResult<Py<super::PyPlan>> {
    let cycle = match cycle {
        "spot" => DrillCycleKind::Spot,
        "peck" => DrillCycleKind::Peck,
        "chip_break" => DrillCycleKind::ChipBreak,
        _ => DrillCycleKind::Plain,
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
    let opts = DrillingWorkplanOptions {
        cycle,
        diameter,
        tip_angle: tip_angle_deg,
        z_surface,
        target_z,
        rapid_z,
        safe_z,
        peck_increment,
        chip_break_retract,
        dwell_ms,
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
    let plan = drilling::plan_drilling(&part.borrow().inner, face_id, &opts)?;
    Py::new(part.py(), super::PyPlan { inner: plan })
}
