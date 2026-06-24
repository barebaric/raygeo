pyo3_stub_gen::module_doc!("raygeo.geo.algo.engagement", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Circle-boundary overlap (engagement) metrics.
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::algo::engagement;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "engagement")?;
    m.setattr("__doc__", MODULE_DOC)?;
    register_functions!(m, compute_engagement_py,);
    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    def compute_engagement(
        d_to_boundary: float,
        radius: float,
    ) -> tuple[float, float, float]:
        """Compute engagement angle, area, and chord depth.

        :param d_to_boundary: Signed distance from the point to the nearest boundary
            (mm).  Positive = outside the boundary.
        :param radius: Disk radius (mm).
        :returns: ``(angle_rad, area, chord_depth)``.
        """
    "#,
    module = "raygeo.geo.algo.engagement"
)]
#[pyfunction(name = "compute_engagement")]
fn compute_engagement_py(d_to_boundary: f64, radius: f64) -> (f64, f64, f64) {
    let e = engagement::compute_engagement(d_to_boundary, radius);
    (e.angle, e.area, e.chord_depth)
}
