pyo3_stub_gen::module_doc!("raygeo.geo.algo.overcut", "{}", MODULE_DOC_OVERCUT);

pub(crate) const MODULE_DOC_OVERCUT: &str = "\
Overcut operations for closed contours.

Extends closed contours past their start point to ensure complete
cuts through the material, particularly useful in laser cutting
where the laser may not fully penetrate at the start/end point.
";

use super::super::Geometry;
pub(super) use crate::geo::algo::overcut::apply_overcut;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "overcut")?;
    m.setattr("__doc__", MODULE_DOC_OVERCUT)?;

    register_functions!(m, apply_overcut_py,);

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo

    def apply_overcut(
        geometry: geo.Geometry,
        overcut: float,
    ) -> geo.Geometry:
        """Extend a closed contour past its start point.

        When laser-cutting closed contours, the laser slows down at
        corners and may not cut through completely. This function
        extends the path by ``overcut`` distance past the start point
        to ensure a clean cut.

        If the geometry is not closed, empty, or overcut is <= 0, the
        geometry is returned unchanged.

        :param geometry: The input geometry (must be closed).
        :param overcut: Distance to extend past the start point.
        :returns: A new geometry with the overcut applied.
        :complexity: O(n) time, O(n) space
        """
"#,
    module = "raygeo.geo.algo.overcut"
)]
#[pyfunction(name = "apply_overcut")]
fn apply_overcut_py(
    geometry: &Geometry,
    overcut: f64,
) -> super::super::Geometry {
    super::super::Geometry {
        inner: apply_overcut(&geometry.inner, overcut),
    }
}
