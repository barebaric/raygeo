mod algo;
mod flex_point;
pub(crate) mod geometry;
mod path;
mod shape;

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_type_alias_from_python;

use crate::geo::geometry::Geometry;
use raygeo_core::{
    CMD_TYPE_ARC, CMD_TYPE_BEZIER, CMD_TYPE_LINE,
    CMD_TYPE_MOVE, COL_C1X, COL_C1Y, COL_C2X, COL_C2Y, COL_CW, COL_I, COL_J,
    COL_TYPE, COL_X, COL_Y, COL_Z, GEO_ARRAY_COLS,
};

gen_type_alias_from_python!(
    "raygeo.geo",
    r#"
    type Point = tuple[float, float]
    "#
);

gen_type_alias_from_python!(
    "raygeo.geo",
    r#"
    type Point3D = tuple[float, float, float]
    "#
);

gen_type_alias_from_python!(
    "raygeo.geo",
    r#"
    type Point2DOr3D = Point | Point3D
    "#
);

gen_type_alias_from_python!(
    "raygeo.geo",
    r#"
    type Polygon = list[Point]
    "#
);

gen_type_alias_from_python!(
    "raygeo.geo",
    r#"
    type Rect = tuple[float, float, float, float]
    "#
);

gen_type_alias_from_python!(
    "raygeo.geo",
    r#"
    type TransformMatrix = list[list[float]] | numpy.ndarray[tuple[int, int], float]
    "#
);

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let geo_mod = PyModule::new(py, "geo")?;

    geo_mod.setattr(
        "__doc__",
        "Geometry types and operations for 2D/3D path data.\n\
        \n\
        Provides Geometry class, path submodules for analysis/cleanup/intersection,\n\
        shape submodules (arc, bezier, circle, line, polygon, rect, point),\n\
        and algorithm submodules (clipping, fitting, minkowski, simplify, smooth).\n\
        \n\
        Submodules:\n\
        - raygeo.geo.path — array-level path utilities\n\
        - raygeo.geo.shape — primitive shape operations\n\
        - raygeo.geo.algo — geometric algorithms",
    )?;
    geo_mod.add(
        "__all__",
        vec![
            "Geometry",
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
            "Point",
            "Point3D",
            "Rect",
            "Polygon",
            "IntPolygon",
            "IntPoint",
            "Edge",
            "CubicBezier",
            "Point2DOr3D",
            "Polygon3D",
            "Rect3D",
        ],
    )?;

    add_functions(&geo_mod)?;
    add_type_aliases(&geo_mod)?;
    add_submodules(&geo_mod)?;

    m.add_submodule(&geo_mod)?;
    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo", &geo_mod)?;

    add_functions(m)?;
    add_type_aliases(m)?;
    add_submodules(m)?;

    Ok(())
}

fn add_submodules(m: &Bound<'_, PyModule>) -> PyResult<()> {
    shape::register(m)?;
    algo::register(m)?;
    path::register(m)?;

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

fn add_type_aliases(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let typing = py.import("typing")?;
    let tuple_type = typing.getattr("Tuple")?;
    let list_type = typing.getattr("List")?;
    let union_type = typing.getattr("Union")?;
    let float_type = py.get_type::<pyo3::types::PyFloat>();
    let int_type = py.import("builtins")?.getattr("int")?;

    let point =
        tuple_type.get_item((float_type.clone(), float_type.clone()))?;
    let point3d = tuple_type.get_item((
        float_type.clone(),
        float_type.clone(),
        float_type.clone(),
    ))?;
    let rect = tuple_type.get_item((
        float_type.clone(),
        float_type.clone(),
        float_type.clone(),
        float_type,
    ))?;
    let polygon = list_type.get_item(point.clone())?;
    let polygon3d = list_type.get_item(point3d.clone())?;
    let int_point = tuple_type.get_item((int_type.clone(), int_type))?;
    let int_polygon = list_type.get_item(int_point.clone())?;
    let edge = tuple_type.get_item((point.clone(), point.clone()))?;
    let cubic_bezier = tuple_type.get_item((
        point.clone(),
        point.clone(),
        point.clone(),
        point.clone(),
    ))?;
    let point_2d_or_3d =
        union_type.get_item((point.clone(), point3d.clone()))?;

    m.add("Point", point)?;
    m.add("Point3D", point3d)?;
    m.add("Rect", rect)?;
    m.add("Polygon", polygon)?;
    m.add("Polygon3D", polygon3d)?;
    m.add("IntPoint", int_point)?;
    m.add("IntPolygon", int_polygon)?;
    m.add("Edge", edge)?;
    m.add("CubicBezier", cubic_bezier)?;
    m.add("Point2DOr3D", point_2d_or_3d)?;

    let collections = py.import("collections")?;
    let rect3d = collections.call_method1(
        "namedtuple",
        (
            "Rect3D",
            vec!["x_min", "x_max", "y_min", "y_max", "z_min", "z_max"],
        ),
    )?;
    m.add("Rect3D", rect3d)?;

    Ok(())
}

fn add_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
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

    m.add_class::<Geometry>()?;

    Ok(())
}
