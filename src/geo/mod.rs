pub(crate) mod algo;
mod flex_point;
pub(crate) mod geometry;
pub(crate) mod math;
pub(crate) mod shape;
pub(crate) mod types;

pub(crate) const MODULE_DOC: &str = "\
Geometry types and operations for 2D/3D path data.

The central type is Geometry — a mutable sequence of drawing commands
(move, line, arc, bezier) that represents one or more closed or open
paths. Geometry supports construction (add_rect, add_circle, etc.),
analysis (area, distance, bounding rect), and manipulation (transform,
simplify, linearize, fit curves, grow/shrink, split, clip).

Shape sub-modules provide primitive-specific operations: arc bounding
and intersection, bezier splitting and flattening, circle containment
tests, polygon boolean algebra and offsetting, and line intersection.

Algorithm sub-modules provide higher-level geometric processing such
as polyline simplification, smoothing, curve fitting, and Minkowski
sums for toolpath generation.
";

use pyo3::prelude::*;

use crate::geo::geometry::{Geometry, PyCommand};
use raygeo_core::{
    CMD_TYPE_ARC, CMD_TYPE_BEZIER, CMD_TYPE_LINE,
    CMD_TYPE_MOVE, COL_C1X, COL_C1Y, COL_C2X, COL_C2Y, COL_CW, COL_I, COL_J,
    COL_TYPE, COL_X, COL_Y, COL_Z, GEO_ARRAY_COLS,
};

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let geo_mod = PyModule::new(py, "geo")?;

    geo_mod.setattr("__doc__", MODULE_DOC)?;
    geo_mod.add(
        "__all__",
        vec![
            "Geometry",
            "PyCommand",
            "CMD_TYPE_MOVE",
            "CMD_TYPE_LINE",
            "CMD_TYPE_ARC",
            "CMD_TYPE_BEZIER",
            "COL_TYPE",
            "COL_X",
            "COL_Y",
            "COL_Z",
            "COL_I",
            "COL_J",
            "COL_CW",
            "COL_C1X",
            "COL_C1Y",
            "COL_C2X",
            "COL_C2Y",
            "GEO_ARRAY_COLS",
            "types",
        ],
    )?;

    add_functions(&geo_mod)?;
    add_submodules(&geo_mod)?;

    // Child submodule: raygeo.geo.types
    let types_mod = PyModule::new(py, "types")?;
    types::register(&types_mod)?;
    geo_mod.add_submodule(&types_mod)?;

    m.add_submodule(&geo_mod)?;
    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo", &geo_mod)?;
    sys_modules.set_item("raygeo.geo.types", &types_mod)?;

    add_constants(m)?;
    add_submodules(m)?;

    Ok(())
}

fn add_submodules(m: &Bound<'_, PyModule>) -> PyResult<()> {
    shape::register(m)?;
    algo::register(m)?;
    math::register(m)?;

    let py = m.py();
    let types_mod = py.import("types")?;
    let constants = types_mod.call_method0("SimpleNamespace")?;
    {
        use raygeo_core::{
            CMD_TYPE_ARC, CMD_TYPE_BEZIER, CMD_TYPE_LINE, CMD_TYPE_MOVE,
            COL_C1X, COL_C1Y, COL_C2X, COL_C2Y, COL_CW, COL_I, COL_J, COL_TYPE,
            COL_X, COL_Y, COL_Z, GEO_ARRAY_COLS,
        };
        constants.setattr("CMD_TYPE_MOVE", CMD_TYPE_MOVE)?;
        constants.setattr("CMD_TYPE_LINE", CMD_TYPE_LINE)?;
        constants.setattr("CMD_TYPE_ARC", CMD_TYPE_ARC)?;
        constants.setattr("CMD_TYPE_BEZIER", CMD_TYPE_BEZIER)?;
        constants.setattr("COL_TYPE", COL_TYPE)?;
        constants.setattr("COL_X", COL_X)?;
        constants.setattr("COL_Y", COL_Y)?;
        constants.setattr("COL_Z", COL_Z)?;
        constants.setattr("COL_I", COL_I)?;
        constants.setattr("COL_J", COL_J)?;
        constants.setattr("COL_CW", COL_CW)?;
        constants.setattr("COL_C1X", COL_C1X)?;
        constants.setattr("COL_C1Y", COL_C1Y)?;
        constants.setattr("COL_C2X", COL_C2X)?;
        constants.setattr("COL_C2Y", COL_C2Y)?;
        constants.setattr("GEO_ARRAY_COLS", GEO_ARRAY_COLS)?;
    }
    m.add("constants", constants)?;

    Ok(())
}

fn add_constants(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("CMD_TYPE_MOVE", CMD_TYPE_MOVE)?;
    m.add("CMD_TYPE_LINE", CMD_TYPE_LINE)?;
    m.add("CMD_TYPE_ARC", CMD_TYPE_ARC)?;
    m.add("CMD_TYPE_BEZIER", CMD_TYPE_BEZIER)?;
    m.add("COL_TYPE", COL_TYPE)?;
    m.add("COL_X", COL_X)?;
    m.add("COL_Y", COL_Y)?;
    m.add("COL_Z", COL_Z)?;
    m.add("COL_I", COL_I)?;
    m.add("COL_J", COL_J)?;
    m.add("COL_CW", COL_CW)?;
    m.add("COL_C1X", COL_C1X)?;
    m.add("COL_C1Y", COL_C1Y)?;
    m.add("COL_C2X", COL_C2X)?;
    m.add("COL_C2Y", COL_C2Y)?;
    m.add("GEO_ARRAY_COLS", GEO_ARRAY_COLS)?;
    Ok(())
}

fn add_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    add_constants(m)?;
    m.add_class::<Geometry>()?;
    m.add_class::<PyCommand>()?;
    Ok(())
}
