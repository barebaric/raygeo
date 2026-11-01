pyo3_stub_gen::module_doc!(
    "raygeo.geo.algo.offset_exact",
    "{}",
    MODULE_DOC_OFFSET_EXACT
);

pub(crate) const MODULE_DOC_OFFSET_EXACT: &str = "\
Exact offsetting of closed line-and-arc contours.

Offsets boundaries keeping lines and arcs exact, so an arc wall
comes out as an arc rather than a chord approximation.
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::algo::offset_exact::offset_exact;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "offset_exact")?;
    m.setattr("__doc__", MODULE_DOC_OFFSET_EXACT)?;

    register_functions!(m, offset_exact_py,);

    algo_mod.add_submodule(&m)?;
    let sys_modules = algo_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.algo.offset_exact", &m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def offset_exact(
        geometry: raygeo.geo.Geometry,
        offset: float,
        tolerance: float = 0.1,
    ) -> list[raygeo.geo.Geometry]:
        """
        Offset the closed line-and-arc contours of a geometry exactly.

        Every closed subpath takes part: counter-clockwise loops bound
        material, clockwise ones bound holes. Lines and arcs stay exact
        (arc walls come out as arcs); other curves are followed as line
        chains within `tolerance`. Open subpaths are ignored. A positive
        `offset` grows the region, a negative one shrinks it.

        :param geometry: geometry whose closed subpaths are offset.
        :param offset: offset distance; positive grows the region.
        :param tolerance: line-chain tolerance for non-line-and-arc curves.
        :returns: the offset closed loops, one geometry per loop.
        :complexity: O(n^2) in the number of boundary pieces.
        """
"#,
    module = "raygeo.geo.algo.offset_exact"
)]
#[pyfunction(name = "offset_exact")]
#[pyo3(signature = (geometry, offset, tolerance = 0.1))]
fn offset_exact_py(
    py: Python<'_>,
    geometry: pyo3::Py<super::super::Geometry>,
    offset: f64,
    tolerance: f64,
) -> Vec<super::super::Geometry> {
    let inner = geometry.borrow(py).inner.clone();
    offset_exact(&inner, offset, tolerance)
        .into_iter()
        .map(|g| super::super::Geometry { inner: g })
        .collect()
}
