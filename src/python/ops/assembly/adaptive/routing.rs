//! Python wrapper for the adaptive-clearing routing / path-smoothing helper.
//!
//! Mirrors [`crate::ops::assembly::adaptive::routing`].  Currently exposes
//! the pure-geometry path shortener ([`smooth_route`]) so it can be
//! exercised directly from Python tests.

use crate::types::{Point, Point3D, Polygon};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub(crate) fn register(adaptive_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let routing_mod = PyModule::new(adaptive_mod.py(), "routing")?;
    register_functions!(routing_mod, smooth_route_py,);
    adaptive_mod.add_submodule(&routing_mod)?;

    let sys_modules = adaptive_mod.py().import("sys")?.getattr("modules")?;
    sys_modules
        .set_item("raygeo.ops.assembly.adaptive.routing", &routing_mod)?;

    Ok(())
}

/// Smooth and shorten a cleared-territory travel path.
///
/// Feeds *raw* (a waypoint list known to stay inside cleared territory)
/// through ``build_smoothed_path`` against the *obstacles* so redundant
/// intermediate waypoints are shortcut away and sharp turns are rounded.
///
/// :param from_pt: Tool's current position ``(x, y, z)`` (preserved as the
///                 first point of the result).
/// :param raw: Waypoint list (e.g. from the MAT).
/// :param obstacles: Obstacle polygons (islands + remaining stock).
/// :param clearance: Minimum distance from obstacles (tool radius).
/// :returns: Shortened, smoothed path as a list of ``(x, y, z)`` points.
#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def smooth_route(
        from_pt: tuple[float, float, float],
        raw: collections.abc.Sequence[tuple[float, float, float]],
        obstacles: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        clearance: float = 1.0,
    ) -> list[tuple[float, float, float]]:
        """Smooth and shorten a cleared-territory travel path."""
    "#,
    module = "raygeo.ops.assembly.adaptive.routing"
)]
#[pyfunction(name = "smooth_route")]
#[pyo3(signature = (from_pt, raw, obstacles = None, clearance = 1.0))]
fn smooth_route_py(
    from_pt: (f64, f64, f64),
    raw: Vec<(f64, f64, f64)>,
    obstacles: Option<Vec<Vec<(f64, f64)>>>,
    clearance: f64,
) -> Vec<(f64, f64, f64)> {
    let from = Point3D::new(from_pt.0, from_pt.1, from_pt.2);
    let raw_pts: Vec<Point3D> = raw
        .into_iter()
        .map(|(x, y, z)| Point3D::new(x, y, z))
        .collect();
    let obstacles_pts: Vec<Polygon> = obstacles
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    crate::ops::assembly::adaptive::routing::smooth_route(
        from,
        &raw_pts,
        &obstacles_pts,
        clearance,
    )
    .into_iter()
    .map(|p| (p.x, p.y, p.z))
    .collect()
}
