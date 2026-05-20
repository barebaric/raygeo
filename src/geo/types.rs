use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_type_alias_from_python;

pub(crate) const MODULE_DOC: &str = "\
Type aliases used throughout the raygeo API.

Point — a 2D coordinate as ``(x, y)``.
Point3D — a 3D coordinate as ``(x, y, z)``.
Point2DOr3D — union of Point and Point3D for functions that accept both.
Polygon — a list of points forming a closed shape.
Rect — a bounding rectangle as ``(x_min, y_min, x_max, y_max)``.
TransformMatrix — a 4x4 affine transformation matrix for 2D/3D transforms.
Edge — a pair of points ``((x1, y1), (x2, y2))``.
CubicBezier — four control points ``(p0, p1, p2, p3)``.
IntPoint / IntPolygon — integer-coordinate variants for Clipper.
";

gen_type_alias_from_python!(
    "raygeo.geo.types",
    r#"
    type Point = tuple[float, float]
    "#
);

gen_type_alias_from_python!(
    "raygeo.geo.types",
    r#"
    type Point3D = tuple[float, float, float]
    "#
);

gen_type_alias_from_python!(
    "raygeo.geo.types",
    r#"
    type Point2DOr3D = Point | Point3D
    "#
);

gen_type_alias_from_python!(
    "raygeo.geo.types",
    r#"
    type Polygon = list[Point]
    "#
);

gen_type_alias_from_python!(
    "raygeo.geo.types",
    r#"
    type Rect = tuple[float, float, float, float]
    "#
);

gen_type_alias_from_python!(
    "raygeo.geo.types",
    r#"
    type TransformMatrix = list[list[float]] | numpy.ndarray[tuple[int, int], float]
    "#
);

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    m.setattr("__doc__", MODULE_DOC)?;
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
