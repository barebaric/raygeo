pyo3_stub_gen::module_doc!("raygeo.geo.algo.offset", "{}", MODULE_DOC_OFFSET);

pub(crate) const MODULE_DOC_OFFSET: &str = "\
Polygon offsetting operations for geometry data.

Provides concentric inward offset generation for adaptive clearing
and pocketing toolpath generation.
";

use crate::geo::algo::offset::concentric_offsets as rust_concentric_offsets;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "offset")?;
    m.setattr("__doc__", MODULE_DOC_OFFSET)?;

    register_functions!(m, concentric_offsets_py,);

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def concentric_offsets(
        geom: raygeo.Geometry,
        step: float,
        max_passes: int = 10,
        min_area: float = 1.0,
    ) -> list[raygeo.Geometry]:
        """Generate concentric inward offsets of a geometry.

        Each successive offset shrinks the boundary by `step`. Stops early
        when the enclosed area drops below `min_area` or `max_passes` is
        reached. Returns offsets outermost-first.

        :param geom: A closed geometry.
        :param step: Inward offset distance per pass.
        :param max_passes: Maximum number of offset passes (default 10).
        :param min_area: Minimum area to stop at (default 1.0).
        :returns: List of offset geometries, outermost first.
        :complexity: O(n * p) time, O(n) space where n is the number of contour vertices and p the number of passes
        """
"#,
    module = "raygeo.geo.algo.offset"
)]
#[pyfunction(name = "concentric_offsets")]
#[pyo3(signature = (geom, step, max_passes=10, min_area=1.0))]
fn concentric_offsets_py(
    py: Python<'_>,
    geom: pyo3::Py<super::super::Geometry>,
    step: f64,
    max_passes: usize,
    min_area: f64,
) -> Vec<super::super::Geometry> {
    let inner = geom.borrow(py).inner.clone();
    let results = rust_concentric_offsets(&inner, step, max_passes, min_area);
    results
        .into_iter()
        .map(|g| super::super::Geometry { inner: g })
        .collect()
}
