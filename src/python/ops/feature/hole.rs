use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::types::{Point, Polygon};
use crate::ops::feature::hole::find_holes;

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def find_holes(
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        tolerance: float = 0.1,
    ) -> list[tuple[float, float, float]]:
        """Find circular holes among island contours.

        An island counts as a hole when every densified sample of its
        outline lies on a common circle within ``tolerance`` (mm).
        Non-circular islands (bosses, slots, arbitrary pockets) are
        rejected.

        :param islands: Island contours as ``[[(x, y), ...], ...]``.
        :param tolerance: Maximum distance (mm) between any contour
            sample and the fitted circle (default 0.1).
        :returns: ``[(x, y, diameter), ...]`` in the order the islands
            were passed.
        """
    "#,
    module = "raygeo.ops.feature.hole"
)]
#[pyfunction(name = "find_holes")]
#[pyo3(signature = (islands, tolerance = 0.1))]
fn find_holes_py(
    islands: Vec<Vec<(f64, f64)>>,
    tolerance: f64,
) -> Vec<(f64, f64, f64)> {
    let contours: Vec<Polygon> = islands
        .into_iter()
        .map(|pts| pts.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    find_holes(&contours, tolerance)
        .into_iter()
        .map(|h| (h.center.x, h.center.y, h.diameter))
        .collect()
}

pub fn register(feature_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = feature_mod.py();
    let m = PyModule::new(py, "hole")?;
    m.setattr("__doc__", "Circular hole detection in part faces.")?;

    m.add_function(pyo3::wrap_pyfunction!(find_holes_py, m.clone())?)?;

    feature_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.feature.hole", &m)?;

    Ok(())
}
