use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::cnc::machining::entry::{self, EntryWorkplanOptions};
use crate::cnc::machining::plan::WorkplanStep;
use crate::ops::feature::region::Region;
use crate::types::Point;

pub(crate) fn register(machining_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = machining_mod.py();
    let m = PyModule::new(py, "entry")?;
    register_functions!(m, build_entry_workplan_py,);
    machining_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.machining.entry", &m)?;

    Ok(())
}

// ── build_entry_workplan ───────────────────────────────────────────

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def build_entry_workplan(
        region_polygon: collections.abc.Sequence[tuple[float, float]],
        entry_point: tuple[float, float],
        r_max: float,
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] | None = None,
        tool_radius: float = 3.0,
        step_over: float = 2.0,
        safe_z: float = 2.0,
        target_z: float = -5.0,
        plunge_pitch: float = 1.0,
        safe_margin: float = 1.0,
        angular_step: float = 0.1,
    ) -> list[dict]:
        """Build an entry workplan for a single wide region.

        Strategy is chosen based on ``r_max``: helix+spiral when
        ``r_max >= 2 × tool_diameter``, toroidal ramp if a carrier is
        found, or zigzag ramp as fallback.

        Use :func:`raygeo.ops.feature.region.find_regions` to obtain
        region data first.

        :param region_polygon: Region boundary as [(x, y), ...].
        :param entry_point: Inscribed-circle centre (x, y).
        :param r_max: Largest inscribed circle radius in mm.
        :param islands: List of island polygons (default None).
        :param tool_radius: Tool radius in mm (default 3.0).
        :param step_over: Radial step-over (default 2.0).
        :param safe_z: Safe Z height (default 2.0).
        :param target_z: Target cutting depth (default -5.0).
        :param plunge_pitch: Helix pitch per revolution (default 1.0).
        :param safe_margin: Safety margin from tool edge (default 1.0).
        :param angular_step: Angular step in radians (default 0.1).
        :returns: List of WorkplanStep dicts with a "kind" key.
        """
    "#,
    module = "raygeo.cnc.machining.entry"
)]
#[pyfunction(name = "build_entry_workplan")]
#[pyo3(signature = (
    region_polygon,
    entry_point,
    r_max,
    islands = None,
    tool_radius = 3.0,
    step_over = 2.0,
    safe_z = 2.0,
    target_z = -5.0,
    plunge_pitch = 1.0,
    safe_margin = 1.0,
    angular_step = 0.1,
))]
#[allow(clippy::too_many_arguments)]
fn build_entry_workplan_py(
    py: Python<'_>,
    region_polygon: Vec<(f64, f64)>,
    entry_point: (f64, f64),
    r_max: f64,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    step_over: f64,
    safe_z: f64,
    target_z: f64,
    plunge_pitch: f64,
    safe_margin: f64,
    angular_step: f64,
) -> PyResult<Vec<Bound<'_, PyDict>>> {
    let polygon: Vec<Point> = region_polygon
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_vec: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let region = Region {
        polygon,
        area: 0.0,
        entry_pt: Point::new(entry_point.0, entry_point.1),
        r_max,
    };

    let opts = EntryWorkplanOptions {
        islands: islands_vec,
        tool_radius,
        step_over,
        safe_z,
        target_z,
        plunge_pitch,
        safe_margin,
        angular_step,
    };

    let steps: Vec<WorkplanStep> = entry::build_entry_workplan(&region, &opts)?;
    let mut result: Vec<Bound<'_, PyDict>> = Vec::with_capacity(steps.len());
    for step in &steps {
        result.push(super::plan::step_to_dict(py, step)?);
    }
    Ok(result)
}
