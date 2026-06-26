pyo3_stub_gen::module_doc!("raygeo.ops.cut", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Cleared-area tracker for material removal.

Maintains a union of swept-disk polygons and provides a spatial-indexed
windowed query for efficient engagement computation.
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::ops::cut::cut_area;
use crate::python::geo::flex_point::polygons_from_tuples;
use crate::types::Point;

pub(crate) mod cleared_area;
pub(crate) mod search;
pub(crate) mod stepper;

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
#[gen_stub_pyfunction(module = "raygeo.ops.cut")]
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

pub fn register(ops_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = ops_mod.py();
    let m = PyModule::new(py, "cut")?;
    m.setattr("__doc__", MODULE_DOC)?;

    cleared_area::register(&m)?;
    search::register(&m)?;
    stepper::register(&m)?;
    m.add_function(wrap_pyfunction!(cut_area_py, &m)?)?;

    ops_mod.add_submodule(&m)?;
    Ok(())
}
