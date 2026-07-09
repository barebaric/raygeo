use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::ops::feature::near::find_plunge_point;
use crate::types::Point;

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def find_plunge_point(
        near: tuple[float, float],
        cleared_polygons: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] | None = None,
        tool_radius: float = 3.0,
        search_radius: float = 10.0,
    ) -> tuple[float, float] | None:
        """Find a plunge point near the given position within the cleared area.

        Searches for a valid tool placement point inside the union of
        ``cleared_polygons`` that is within ``search_radius`` of ``near``,
        fully inside ``boundary``, and does not overlap any ``island``.

        The algorithm first checks ``near`` itself, then searches outward
        on concentric rings with a step of ``tool_radius / 2``.

        :param near: Target point ``(x, y)`` to search near.
        :param cleared_polygons: List of cleared-area polygons.
        :param boundary: Outer boundary polygon.
        :param islands: List of island polygons (optional).
        :param tool_radius: Tool radius in mm.
        :param search_radius: Maximum search distance from ``near`` in mm.
        :returns: ``(x, y)`` plunge point or ``None`` if no valid point found.
        """
    "#,
    module = "raygeo.ops.feature.near"
)]
#[pyfunction(name = "find_plunge_point")]
#[pyo3(signature = (near, cleared_polygons, boundary, islands = None, tool_radius = 3.0, search_radius = 10.0))]
fn find_plunge_point_py(
    near: (f64, f64),
    cleared_polygons: Vec<Vec<(f64, f64)>>,
    boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    search_radius: f64,
) -> Option<(f64, f64)> {
    let near_pt = Point::new(near.0, near.1);

    let cleared: Vec<Vec<Point>> = cleared_polygons
        .into_iter()
        .map(|p| p.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    let boundary_pts: Vec<Point> = boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_pts: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    find_plunge_point(
        near_pt,
        &cleared,
        &boundary_pts,
        &islands_pts,
        tool_radius,
        search_radius,
    )
    .map(|p| (p.x, p.y))
}

pub fn register(feature_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = feature_mod.py();
    let m = PyModule::new(py, "near")?;
    m.setattr(
        "__doc__",
        "Plunge-point finder for adaptive clearing entry.",
    )?;

    m.add_function(pyo3::wrap_pyfunction!(find_plunge_point_py, m.clone())?)?;

    feature_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.feature.near", &m)?;

    Ok(())
}
