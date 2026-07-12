//! Python bindings for all shape operations.

pub(crate) mod arc;
pub(crate) mod bezier;
pub(crate) mod circle;
pub(crate) mod line;
pub(crate) mod point;
pub(crate) mod polygon;
pub(crate) mod polygon3d;
pub(crate) mod polyline;
pub(crate) mod rect;
pub(crate) mod text;

pyo3_stub_gen::module_doc!("raygeo.geo.shape", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Primitive shape operations — arc, bezier, circle, line, point, polygon, rect.

Provides functions for geometric queries on primitive shapes including
bounding boxes, intersection tests, containment checks, linearization,
and affine transformations.
";

use pyo3::prelude::*;

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let shape_mod = PyModule::new(py, "shape")?;
    shape_mod.setattr("__doc__", MODULE_DOC)?;

    arc::register(&shape_mod)?;
    bezier::register(&shape_mod)?;
    circle::register(&shape_mod)?;
    line::register(&shape_mod)?;
    point::register(&shape_mod)?;
    polygon::register(&shape_mod)?;
    polygon3d::register(&shape_mod)?;
    polyline::register(&shape_mod)?;
    rect::register(&shape_mod)?;
    text::register(&shape_mod)?;

    m.add_submodule(&shape_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.shape", &shape_mod)?;
    sys_modules.set_item("raygeo.geo.shape.arc", &shape_mod.getattr("arc")?)?;
    sys_modules
        .set_item("raygeo.geo.shape.bezier", &shape_mod.getattr("bezier")?)?;
    sys_modules
        .set_item("raygeo.geo.shape.circle", &shape_mod.getattr("circle")?)?;
    sys_modules
        .set_item("raygeo.geo.shape.polygon", &shape_mod.getattr("polygon")?)?;
    sys_modules.set_item(
        "raygeo.geo.shape.polyline",
        &shape_mod.getattr("polyline")?,
    )?;
    sys_modules
        .set_item("raygeo.geo.shape.line", &shape_mod.getattr("line")?)?;
    sys_modules
        .set_item("raygeo.geo.shape.rect", &shape_mod.getattr("rect")?)?;
    sys_modules
        .set_item("raygeo.geo.shape.point", &shape_mod.getattr("point")?)?;
    Ok(())
}
