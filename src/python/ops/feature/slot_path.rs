use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::ops::feature::slot_path::find_slot_path;
use crate::types::Point;

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def find_slot_path(
        slot_polygon: collections.abc.Sequence[tuple[float, float]],
        entry_edges: collections.abc.Sequence[int],
        entry_point: tuple[float, float],
        tool_radius: float = 3.0,
    ) -> list[tuple[float, float]] | None:
        """Find the 2D carrier axis for a slot clearing operation.

        Returns a ``[(x1, y1), (x2, y2)]`` list representing the longitudinal
        axis of the slot, with the first point on the entry side. Both points
        are valid tool centres that fit within the eroded slot.

        :param slot_polygon: Slot polygon as ``[(x, y), ...]``.
        :param entry_edges: Indices of entry edges into the slot polygon.
        :param entry_point: Entry point ``(x, y)`` (used only for side
            determination).
        :param tool_radius: Tool radius in mm (default 3.0).
        :returns: ``[(x1, y1), (x2, y2)]`` or ``None`` if the slot is too
            narrow.
        """
    "#,
    module = "raygeo.ops.feature.slot_path"
)]
#[pyfunction(name = "find_slot_path")]
#[pyo3(signature = (slot_polygon, entry_edges, entry_point, tool_radius = 3.0))]
fn find_slot_path_py(
    slot_polygon: Vec<(f64, f64)>,
    entry_edges: Vec<usize>,
    entry_point: (f64, f64),
    tool_radius: f64,
) -> Option<Vec<(f64, f64)>> {
    let slot_points: Vec<Point> = slot_polygon
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let entry = Point::new(entry_point.0, entry_point.1);

    find_slot_path(&slot_points, &entry_edges, entry, tool_radius)
        .map(|pts| pts.into_iter().map(|p| (p.x, p.y)).collect())
}

pub fn register(feature_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = feature_mod.py();
    let m = PyModule::new(py, "slot_path")?;
    m.setattr("__doc__", "Slot carrier finder for adaptive clearing.")?;

    m.add_function(pyo3::wrap_pyfunction!(find_slot_path_py, m.clone())?)?;

    feature_mod.add_submodule(&m)?;
    Ok(())
}
