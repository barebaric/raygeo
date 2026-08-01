pyo3_stub_gen::module_doc!(
    "raygeo.geo.algo.trochoid",
    "{}",
    MODULE_DOC_TROCHOID
);

pub(crate) const MODULE_DOC_TROCHOID: &str = "\
Trochoidal path generation along a carrier polyline.

Provides generation of trochoidal paths with configurable diameter,
engagement angle, and step-over ratio.
";

use crate::geo::algo::trochoid::{
    self, TrochoidOptions as RustTrochoidOptions,
    TrochoidOptionsRamped as RustTrochoidOptionsRamped,
};
use crate::geo::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "trochoid")?;
    m.setattr("__doc__", MODULE_DOC_TROCHOID)?;

    register_functions!(m, trochoid_along_py, trochoid_ramped_py,);

    algo_mod.add_submodule(&m)?;
    let sys_modules = algo_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.algo.trochoid", &m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_trochoid_along_3d(
        carrier: collections.abc.Sequence[tuple[float, float]],
        diameter: float,
        engagement_angle_deg: float = 90.0,
        step_over_ratio: float = 0.2,
        min_loop_radius: float = 0.5,
        z: float = 0.0,
    ) -> list[tuple[float, float, float]]:
        """Generate a trochoidal path along a carrier polyline.

        :param carrier: Sequence of (x, y) points defining the centerline.
        :param diameter: Trochoid generating circle diameter.
        :param engagement_angle_deg: Engagement angle in degrees (default 90).
        :param step_over_ratio: Forward advance per loop as fraction of diameter (default 0.2).
        :param min_loop_radius: Minimum trochoid loop radius in mm (default 0.5).
        :param z: Z height for all points (default 0.0).
        :returns: List of (x, y, z) points forming the trochoidal path.
        :complexity: O(n) time, O(n) space where n is proportional to path length / step
        """
"#,
    module = "raygeo.geo.algo.trochoid"
)]
#[pyfunction(name = "get_trochoid_along_3d")]
#[pyo3(signature = (
    carrier,
    diameter,
    engagement_angle_deg = 90.0,
    step_over_ratio = 0.2,
    min_loop_radius = 0.5,
    z = 0.0,
))]
fn trochoid_along_py(
    carrier: Vec<(f64, f64)>,
    diameter: f64,
    engagement_angle_deg: f64,
    step_over_ratio: f64,
    min_loop_radius: f64,
    z: f64,
) -> Vec<(f64, f64, f64)> {
    let carrier_pts: Vec<Point> =
        carrier.into_iter().map(|(x, y)| Point::new(x, y)).collect();
    let opts = RustTrochoidOptions {
        diameter,
        engagement_angle_deg,
        step_over_ratio,
        min_loop_radius,
        z,
    };
    let pts = trochoid::get_trochoid_along_3d(&carrier_pts, &opts);
    pts.into_iter().map(|p| (p.x, p.y, p.z)).collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_trochoid_along_3d_ramped(
        carrier: collections.abc.Sequence[tuple[float, float]],
        diameter: float,
        z_start: float,
        z_end: float,
        engagement_angle_deg: float = 90.0,
        step_over_ratio: float = 0.2,
        min_loop_radius: float = 0.5,
    ) -> list[tuple[float, float, float]]:
        """Generate a trochoidal path with Z ramped along the carrier.

        The Z coordinate descends linearly from ``z_start`` to ``z_end``
        as a function of cumulative arc-length along the carrier.

        :param carrier: Sequence of (x, y) points defining the centerline.
        :param diameter: Trochoid generating circle diameter.
        :param z_start: Z height at the start of the carrier.
        :param z_end: Z height at the end of the carrier.
        :param engagement_angle_deg: Engagement angle in degrees (default 90).
        :param step_over_ratio: Forward advance per loop as fraction of diameter (default 0.2).
        :param min_loop_radius: Minimum trochoid loop radius in mm (default 0.5).
        :returns: List of (x, y, z) points forming the ramped trochoidal path.
        :complexity: O(n) time, O(n) space where n is proportional to path length / step
        """
    "#,
    module = "raygeo.geo.algo.trochoid"
)]
#[pyfunction(name = "get_trochoid_along_3d_ramped")]
#[pyo3(signature = (
    carrier,
    diameter,
    z_start,
    z_end,
    engagement_angle_deg = 90.0,
    step_over_ratio = 0.2,
    min_loop_radius = 0.5,
))]
fn trochoid_ramped_py(
    carrier: Vec<(f64, f64)>,
    diameter: f64,
    z_start: f64,
    z_end: f64,
    engagement_angle_deg: f64,
    step_over_ratio: f64,
    min_loop_radius: f64,
) -> Vec<(f64, f64, f64)> {
    let carrier_pts: Vec<Point> =
        carrier.into_iter().map(|(x, y)| Point::new(x, y)).collect();
    let opts = RustTrochoidOptionsRamped {
        diameter,
        engagement_angle_deg,
        step_over_ratio,
        min_loop_radius,
        z_start,
        z_end,
    };
    let pts = trochoid::get_trochoid_along_3d_ramped(&carrier_pts, &opts);
    pts.into_iter().map(|p| (p.x, p.y, p.z)).collect()
}
