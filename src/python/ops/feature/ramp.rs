use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::ops::feature::ramp::find_ramp_carrier;
use crate::types::Point;

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def find_ramp_carrier(
        boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] | None = None,
        tool_radius: float = 3.0,
        max_ramp_angle_deg: float = 45.0,
    ) -> tuple[tuple[float, float], tuple[float, float]] | None:
        """Find the longest straight carrier segment suitable for ramp entry.

        Returns a ``((x1, y1), (x2, y2))`` tuple representing the longest
        straight-line segment within the valid tool-centre region (boundary
        eroded by ``tool_radius`` minus dilated islands) that is long enough
        for a ramp descent of one pass at the given maximum ramp angle.

        The returned segment is oriented so the start point has the smaller
        coordinate on the dominant axis.

        :param boundary: Outer boundary polygon as ``[(x, y), ...]``.
        :param islands: List of island polygons (optional).
        :param tool_radius: Tool radius in mm.
        :param max_ramp_angle_deg: Maximum ramp angle in degrees.
        :returns: ``((x1, y1), (x2, y2))`` or ``None`` if no carrier found.
        """
    "#,
    module = "raygeo.ops.feature.ramp"
)]
#[pyfunction(name = "find_ramp_carrier")]
#[pyo3(signature = (boundary, islands = None, tool_radius = 3.0, max_ramp_angle_deg = 45.0))]
fn find_ramp_carrier_py(
    boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    max_ramp_angle_deg: f64,
) -> Option<((f64, f64), (f64, f64))> {
    let boundary_pts: Vec<Point> = boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_pts: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    find_ramp_carrier(
        &boundary_pts,
        &islands_pts,
        tool_radius,
        max_ramp_angle_deg,
    )
    .map(|(p1, p2)| ((p1.x, p1.y), (p2.x, p2.y)))
}

pub fn register(feature_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = feature_mod.py();
    let m = PyModule::new(py, "ramp")?;
    m.setattr(
        "__doc__",
        "Ramp carrier finder for adaptive clearing entry.",
    )?;

    m.add_function(pyo3::wrap_pyfunction!(find_ramp_carrier_py, m.clone())?)?;

    feature_mod.add_submodule(&m)?;
    Ok(())
}
