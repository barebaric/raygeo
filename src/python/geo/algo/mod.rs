pub(crate) mod analysis;
pub(crate) mod clipping;
pub(crate) mod cylindrical;
pub(crate) mod fitting;
pub(crate) mod hull;
pub(crate) mod interp;
pub(crate) mod minkowski;
pub(crate) mod overcut;
pub(crate) mod simplify;
pub(crate) mod smooth;

pyo3_stub_gen::module_doc!("raygeo.geo.algo", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Geometric algorithms for path processing.

This module provides algorithms that operate on geometry paths and point
sequences. It covers several categories of geometric processing:

Clipping — intersect and clip line segments against rectangles and
polygon regions. Also includes coordinate conversion between floating
point and Clipper's integer grid system for boolean-accuracy clipping.

Fitting — reconstruct curves (arcs, lines, beziers) from unordered
point sequences. Includes recursive primitive fitting, circle fitting,
polyline linearization, and deviation analysis to evaluate fit quality.

Minkowski sums — compute Minkowski sums, convolutions, and no-fit
polygons for 2D toolpath generation, nesting, and packing algorithms.

Simplification — reduce the number of points in a polyline while
preserving shape within a tolerance (Ramer-Douglas-Peucker).

Smoothing — apply Gaussian kernel smoothing to polylines with
configurable corner-angle thresholds to preserve sharp features.
";

use pyo3::prelude::*;

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let algo_mod = PyModule::new(py, "algo")?;
    algo_mod.setattr("__doc__", MODULE_DOC)?;

    analysis::register(&algo_mod)?;
    clipping::register(&algo_mod)?;
    cylindrical::register(&algo_mod)?;
    fitting::register(&algo_mod)?;
    hull::register(&algo_mod)?;
    interp::register(&algo_mod)?;
    minkowski::register(&algo_mod)?;
    overcut::register(&algo_mod)?;
    simplify::register(&algo_mod)?;
    smooth::register(&algo_mod)?;

    m.add_submodule(&algo_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.algo", &algo_mod)?;
    sys_modules
        .set_item("raygeo.geo.algo.analysis", &algo_mod.getattr("analysis")?)?;
    sys_modules
        .set_item("raygeo.geo.algo.clipping", &algo_mod.getattr("clipping")?)?;
    sys_modules
        .set_item("raygeo.geo.algo.fitting", &algo_mod.getattr("fitting")?)?;
    sys_modules.set_item("raygeo.geo.algo.hull", &algo_mod.getattr("hull")?)?;
    sys_modules
        .set_item("raygeo.geo.algo.interp", &algo_mod.getattr("interp")?)?;
    sys_modules.set_item(
        "raygeo.geo.algo.minkowski",
        &algo_mod.getattr("minkowski")?,
    )?;
    sys_modules
        .set_item("raygeo.geo.algo.overcut", &algo_mod.getattr("overcut")?)?;
    sys_modules
        .set_item("raygeo.geo.algo.simplify", &algo_mod.getattr("simplify")?)?;
    sys_modules
        .set_item("raygeo.geo.algo.smooth", &algo_mod.getattr("smooth")?)?;
    sys_modules.set_item(
        "raygeo.geo.algo.cylindrical",
        &algo_mod.getattr("cylindrical")?,
    )?;

    Ok(())
}
