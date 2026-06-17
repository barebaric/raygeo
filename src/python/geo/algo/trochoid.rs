pyo3_stub_gen::module_doc!(
    "raygeo.geo.algo.trochoid",
    "{}",
    MODULE_DOC_TROCHOID
);

pub(crate) const MODULE_DOC_TROCHOID: &str = "\
Trochoidal path generation for constant-engagement milling.

Provides generation of trochoidal toolpaths along a carrier polyline,
with configurable tool diameter, engagement angle, and step-over ratio.
";

use crate::geo::algo::trochoid::{
    self, TrochoidOptions as RustTrochoidOptions,
};
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "trochoid")?;
    m.setattr("__doc__", MODULE_DOC_TROCHOID)?;

    register_functions!(m, trochoid_along_py,);

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def trochoid_along(
        carrier: collections.abc.Sequence[tuple[float, float]],
        tool_diameter: float,
        engagement_angle_deg: float = 90.0,
        step_over_ratio: float = 0.2,
        min_loop_radius: float = 0.5,
        z: float = 0.0,
    ) -> list[tuple[float, float, float]]:
        """Generate a trochoidal cutting path along a carrier polyline.

        :param carrier: Sequence of (x, y) points defining the centerline.
        :param tool_diameter: Tool diameter in mm.
        :param engagement_angle_deg: Target engagement angle in degrees (default 90).
        :param step_over_ratio: Forward advance per loop as fraction of tool diameter (default 0.2).
        :param min_loop_radius: Minimum trochoid loop radius in mm (default 0.5).
        :param z: Z height for all points (default 0.0).
        :returns: List of (x, y, z) points forming the trochoidal path.
        :complexity: O(n) time, O(n) space where n is proportional to path length / step
        """
"#,
    module = "raygeo.geo.algo.trochoid"
)]
#[pyfunction(name = "trochoid_along")]
#[pyo3(signature = (
    carrier,
    tool_diameter,
    engagement_angle_deg = 90.0,
    step_over_ratio = 0.2,
    min_loop_radius = 0.5,
    z = 0.0,
))]
fn trochoid_along_py(
    carrier: Vec<(f64, f64)>,
    tool_diameter: f64,
    engagement_angle_deg: f64,
    step_over_ratio: f64,
    min_loop_radius: f64,
    z: f64,
) -> Vec<(f64, f64, f64)> {
    let carrier_pts: Vec<Point> =
        carrier.into_iter().map(|(x, y)| Point::new(x, y)).collect();
    let opts = RustTrochoidOptions {
        tool_diameter,
        engagement_angle_deg,
        step_over_ratio,
        min_loop_radius,
        z,
    };
    let pts = trochoid::trochoid_along(&carrier_pts, &opts);
    pts.into_iter().map(|p| (p.x, p.y, p.z)).collect()
}
