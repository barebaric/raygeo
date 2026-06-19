pyo3_stub_gen::module_doc!("raygeo.geo.algo.hsm", "{}", MODULE_DOC_HSM);

pub(crate) const MODULE_DOC_HSM: &str = "\
HSM (High-Speed Machining) adaptive clearing.

* ``adaptive_entry`` — find the optimal entry pole, then helix + spiral
  (wide area) or zigzag ramp (tight slot).
* ``adaptive_wavefronts`` — inside-out expansion loop: each iteration
  expands the cleared boundary outward by ``step_over``, clips to the
  valid tool area, applies a minimum-curvature filter, and updates the
  cleared state until convergence.
";

use crate::geo::algo::hsm;
use crate::python::geo::algo::cleared_area::ClearedArea as PyClearedArea;
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

type PyAdaptiveEntryResult = (Vec<(f64, f64, f64)>, Vec<Vec<(f64, f64)>>);

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "hsm")?;
    m.setattr("__doc__", MODULE_DOC_HSM)?;

    register_functions!(m, adaptive_entry_py, adaptive_wavefronts_py,);

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def adaptive_entry(
        pocket_boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        tool_radius: float = 3.0,
        step_over: float = 2.0,
        safe_z: float = 2.0,
        target_z: float = -5.0,
        plunge_pitch: float = 1.0,
        safe_margin: float = 1.0,
        angular_step: float = 0.1,
    ) -> tuple[list[tuple[float, float, float]], list[list[tuple[float, float]]]]:
        """Fast central clearing entry.

        Finds the optimal entry pole using ``find_largest_circle``, then
        generates either a helix->spiral (wide area) or zigzag ramp
        (tight slot).

        The returned *cleared_polygons* should be inserted into a
        ``ClearedArea`` via ``add_cleared_polygons``.

        :param pocket_boundary: Outer boundary of the pocket.
        :param islands: List of island (hole) polygons (default []).
        :param tool_radius: Tool radius in mm (default 3.0).
        :param step_over: Radial step-over per spiral revolution (default 2.0).
        :param safe_z: Safe (retract) Z height (default 2.0).
        :param target_z: Target cutting depth (default -5.0).
        :param plunge_pitch: Vertical descent per helix revolution (default 1.0).
        :param safe_margin: Extra margin from tool edge to boundary (default 1.0).
        :param angular_step: Angular step in radians for path vertices (default 0.1).
        :returns: ``(toolpath, cleared_polygons)`` where *toolpath* is a list
                  of (x, y, z) points and *cleared_polygons* is a list of
                  polygons (each a list of (x, y) points) to add to the
                  ``ClearedArea``.
        :complexity: O(n) for the spiral/helix generation, O(m log m) for
                     ``find_largest_circle`` where m is the polygon vertex count.
        """
"#,
    module = "raygeo.geo.algo.hsm"
)]
#[pyfunction(name = "adaptive_entry")]
#[pyo3(signature = (
    pocket_boundary,
    islands = None,
    tool_radius = 3.0,
    step_over = 2.0,
    safe_z = 2.0,
    target_z = -5.0,
    plunge_pitch = 1.0,
    safe_margin = 1.0,
    angular_step = 0.1,
))]
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn adaptive_entry_py(
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    step_over: f64,
    safe_z: f64,
    target_z: f64,
    plunge_pitch: f64,
    safe_margin: f64,
    angular_step: f64,
) -> PyAdaptiveEntryResult {
    let boundary: Vec<Point> = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_pts: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let opts = hsm::AdaptiveEntryOptions {
        pocket_boundary: boundary,
        islands: islands_pts,
        tool_radius,
        step_over,
        safe_z,
        target_z,
        plunge_pitch,
        safe_margin,
        angular_step,
    };

    let result = hsm::adaptive_entry(&opts);

    let toolpath: Vec<(f64, f64, f64)> = result
        .toolpath
        .into_iter()
        .map(|p| (p.x, p.y, p.z))
        .collect();
    let cleared_polys: Vec<Vec<(f64, f64)>> = result
        .cleared_polygons
        .into_iter()
        .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
        .collect();

    (toolpath, cleared_polys)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def adaptive_wavefronts(
        cleared: raygeo.geo.algo.cleared_area.ClearedArea,
        pocket_boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        tool_radius: float = 3.0,
        step_over: float = 2.0,
        z: float = 0.0,
        area_tolerance: float = 1.0,
    ) -> list[list[tuple[float, float, float]]]:
        """Inside-out adaptive wavefronts.

        Starting from the *cleared* state, each iteration expands the
        cleared boundary outward by *step_over*, clips to the valid tool
        area (pocket boundary offset inward by *tool_radius*, with
        islands excluded), and adds the result back to *cleared*.
        The loop terminates when the newly added area drops below
        *area_tolerance*.

        :param cleared: ``ClearedArea`` instance (mutated in place).
        :param pocket_boundary: Outer boundary of the pocket.
        :param islands: List of island (hole) polygons (default []).
        :param tool_radius: Tool radius in mm (default 3.0).
        :param step_over: Radial expansion per iteration (default 2.0).
        :param z: Z height for generated toolpath points (default 0.0).
        :param area_tolerance: Minimum area increase to continue (default 1.0).
        :returns: List of toolpaths — one ``list[(x, y, z)]`` per iteration.
        """
"#,
    module = "raygeo.geo.algo.hsm"
)]
#[pyfunction(name = "adaptive_wavefronts")]
#[pyo3(signature = (
    cleared,
    pocket_boundary,
    islands = None,
    tool_radius = 3.0,
    step_over = 2.0,
    z = 0.0,
    area_tolerance = 1.0,
))]
fn adaptive_wavefronts_py(
    cleared: &mut PyClearedArea,
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    step_over: f64,
    z: f64,
    area_tolerance: f64,
) -> Vec<Vec<(f64, f64, f64)>> {
    let boundary: Vec<Point> = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_pts: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let opts = hsm::AdaptiveWavefrontOptions {
        pocket_boundary: boundary,
        islands: islands_pts,
        tool_radius,
        step_over,
        z,
        area_tolerance,
    };

    let result = hsm::adaptive_wavefronts(&mut cleared.inner, &opts);

    result
        .toolpaths
        .into_iter()
        .map(|path| path.into_iter().map(|p| (p.x, p.y, p.z)).collect())
        .collect()
}
