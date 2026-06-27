use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::ops::cut::cut_area;
use crate::python::geo::flex_point::polygons_from_tuples;
use crate::types::Point;

/// Area of ``disk(c2) − disk(c1) − fragments``, intersected with
/// *valid_area*.
///
/// Returns ``(total, left)`` where *left* is the portion on the left
/// side of the step vector ``c1 → c2``.
///
/// :param c1: Previous centre ``(x, y)``.
/// :param c2: Next centre ``(x, y)``.
/// :param radius: Disk radius (mm).
/// :param fragments: List of polygons (cleared fragments).
/// :param valid_area: Valid region polygons (intersection).
/// :returns: ``(total_area, left_area)`` (mm²).
#[gen_stub_pyfunction(module = "raygeo.ops.cut.crescent")]
#[pyfunction(name = "cut_area")]
fn cut_area_py(
    c1: (f64, f64),
    c2: (f64, f64),
    radius: f64,
    fragments: Vec<Vec<(f64, f64)>>,
    valid_area: Vec<Vec<(f64, f64)>>,
) -> (f64, f64) {
    let frags = polygons_from_tuples(fragments);
    let valid = polygons_from_tuples(valid_area);
    cut_area(
        Point::new(c1.0, c1.1),
        Point::new(c2.0, c2.1),
        radius,
        &frags,
        &valid,
    )
}

pub fn register(cut_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = cut_mod.py();
    let m = PyModule::new(py, "crescent")?;

    m.add_function(wrap_pyfunction!(cut_area_py, &m)?)?;

    cut_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.cut.crescent", &m)?;

    Ok(())
}
