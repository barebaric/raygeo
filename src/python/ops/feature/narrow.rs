use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::ops::feature::narrow::{
    analyze_pocket, NarrowAnalysisOptions, PassageClass,
};
use crate::types::Point;

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def analyze_pocket(
        polygon: collections.abc.Sequence[tuple[float, float]],
        holes: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] | None = None,
        tool_radius: float = 3.0,
        tolerance: float = 0.5,
        min_slot_width: float | None = None,
    ) -> list[tuple[list[tuple[float, float]], str, float, list[int]]]:
        """Analyze a pocket and return classified narrow regions.

        Calls ``find_narrow_passages`` with ``max_width = 1.5 × 2 × tool_radius``,
        then classifies each passage as Narrow (toroidal), Slot, or Unreachable
        based on the minimum passage width.  Each entry in the returned list
        is a tuple of ``(polygon, class, min_width, entry_edge_indices)``.

        * ``polygon`` — list of ``(x, y)`` vertices of the narrow passage.
        * ``class`` — ``"narrow"``, ``"slot"``, or ``"unreachable"``.
        * ``min_width`` — estimated minimum passage width in mm.
        * ``entry_edge_indices`` — list of vertex indices of entry-side edges.

        :param polygon: Outer boundary polygon.
        :param holes: List of hole (island) polygons.
        :param tool_radius: Tool radius in mm.
        :param tolerance: Additional clearance tolerance in mm.
        :param min_slot_width: Minimum passage width for slotting in mm.
            Defaults to ``2 × tool_radius`` (tool diameter) when ``None``.
        :returns: List of ``(polygon, class, min_width, entry_edge_indices)`` tuples.
        :raises RuntimeError: If the polygon cannot be analyzed.
        """
    "#,
    module = "raygeo.ops.feature.narrow"
)]
#[allow(clippy::type_complexity)]
#[pyfunction(name = "analyze_pocket")]
#[pyo3(signature = (polygon, holes = None, tool_radius = 3.0, tolerance = 0.5, min_slot_width = None))]
fn analyze_pocket_py(
    polygon: Vec<(f64, f64)>,
    holes: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    tolerance: f64,
    min_slot_width: Option<f64>,
) -> PyResult<Vec<(Vec<(f64, f64)>, String, f64, Vec<usize>)>> {
    let poly: Vec<Point> =
        polygon.into_iter().map(|(x, y)| Point::new(x, y)).collect();
    let holes_pts: Vec<Vec<Point>> = holes
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let opts = NarrowAnalysisOptions {
        tool_radius,
        tolerance,
        min_slot_width: min_slot_width.unwrap_or(0.0),
    };

    let regions = analyze_pocket(&poly, &holes_pts, &opts)
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;

    let result = regions
        .into_iter()
        .map(|r| {
            let poly_py: Vec<(f64, f64)> =
                r.polygon.into_iter().map(|p| (p.x, p.y)).collect();
            let class_str: String = match r.class {
                PassageClass::Slot => "slot".into(),
                PassageClass::Narrow => "narrow".into(),
                PassageClass::Unreachable => "unreachable".into(),
            };
            (poly_py, class_str, r.min_width, r.entry_edge_indices)
        })
        .collect();

    Ok(result)
}

pub fn register(feature_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = feature_mod.py();
    let m = PyModule::new(py, "narrow")?;
    m.setattr(
        "__doc__",
        "Narrow-passage classification for machining analysis.",
    )?;

    m.add_function(pyo3::wrap_pyfunction!(analyze_pocket_py, m.clone())?)?;

    feature_mod.add_submodule(&m)?;
    Ok(())
}
