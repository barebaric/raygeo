//! Python bindings for 3D polygon boolean and offset operations.

use super::super::flex_point::points3d_to_tuples;
use crate::geo::shape::polygon3d::{
    get_polygons_difference_3d, get_polygons_group_difference_3d,
    get_polygons_group_intersection_3d, get_polygons_intersection_3d,
    get_polygons_union_3d, offset_polygon_3d,
};
use crate::types::Point3D;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

fn extract_polygon3d(ob: &Bound<'_, PyAny>) -> PyResult<Vec<Point3D>> {
    let mut points = Vec::new();
    for item in ob.try_iter()? {
        let item = item?;
        let (x, y, z) = item.extract::<(f64, f64, f64)>()?;
        points.push(Point3D::new(x, y, z));
    }
    Ok(points)
}

fn extract_polygons3d(ob: &Bound<'_, PyAny>) -> PyResult<Vec<Vec<Point3D>>> {
    let mut result = Vec::new();
    for item in ob.try_iter()? {
        let item = item?;
        result.push(extract_polygon3d(&item)?);
    }
    Ok(result)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def get_polygons_union_3d(polygons: typing.Any) -> list[types.Polygon3D]:
        """Compute the union of 3D polygons (XY-plane, Z preserved).

        :param polygons: List of 3D polygons.
        :returns: Union result with Z from first polygon.
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygons_union_3d")]
fn get_polygons_union_3d_py(
    polygons: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<(f64, f64, f64)>>> {
    let polys = extract_polygons3d(polygons)?;
    let result = get_polygons_union_3d(&polys);
    Ok(result.into_iter().map(points3d_to_tuples).collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_polygons_intersection_3d(poly1: typing.Any, poly2: typing.Any) -> list[types.Polygon3D]:
        """Compute the intersection of two 3D polygons (XY-plane, Z preserved).

        :param poly1: First 3D polygon.
        :param poly2: Second 3D polygon.
        :returns: Intersection result with Z from first polygon.
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygons_intersection_3d")]
fn get_polygons_intersection_3d_py(
    poly1: &Bound<'_, PyAny>,
    poly2: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<(f64, f64, f64)>>> {
    let p1 = extract_polygon3d(poly1)?;
    let p2 = extract_polygon3d(poly2)?;
    let result = get_polygons_intersection_3d(&p1, &p2);
    Ok(result.into_iter().map(points3d_to_tuples).collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_polygons_difference_3d(poly1: typing.Any, poly2: typing.Any) -> list[types.Polygon3D]:
        """Compute the difference of two 3D polygons (poly1 - poly2).

        :param poly1: Subject 3D polygon.
        :param poly2: Clip 3D polygon.
        :returns: Difference result with Z from first polygon.
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygons_difference_3d")]
fn get_polygons_difference_3d_py(
    poly1: &Bound<'_, PyAny>,
    poly2: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<(f64, f64, f64)>>> {
    let p1 = extract_polygon3d(poly1)?;
    let p2 = extract_polygon3d(poly2)?;
    let result = get_polygons_difference_3d(&p1, &p2);
    Ok(result.into_iter().map(points3d_to_tuples).collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_polygons_group_intersection_3d(subject: typing.Any, clip: typing.Any) -> list[types.Polygon3D]:
        """Group intersection of 3D polygons (subject ∩ clip).

        :param subject: Subject group of 3D polygons.
        :param clip: Clip group of 3D polygons.
        :returns: Intersection result with Z from first subject polygon.
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygons_group_intersection_3d")]
fn get_polygons_group_intersection_3d_py(
    subject: &Bound<'_, PyAny>,
    clip: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<(f64, f64, f64)>>> {
    let subj = extract_polygons3d(subject)?;
    let clp = extract_polygons3d(clip)?;
    let result = get_polygons_group_intersection_3d(&subj, &clp);
    Ok(result.into_iter().map(points3d_to_tuples).collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_polygons_group_difference_3d(subject: typing.Any, clip: typing.Any) -> list[types.Polygon3D]:
        """Group difference of 3D polygons (subject - clip).

        :param subject: Subject group of 3D polygons.
        :param clip: Clip group of 3D polygons.
        :returns: Difference result with Z from first subject polygon.
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "get_polygons_group_difference_3d")]
fn get_polygons_group_difference_3d_py(
    subject: &Bound<'_, PyAny>,
    clip: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<(f64, f64, f64)>>> {
    let subj = extract_polygons3d(subject)?;
    let clp = extract_polygons3d(clip)?;
    let result = get_polygons_group_difference_3d(&subj, &clp);
    Ok(result.into_iter().map(points3d_to_tuples).collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def offset_polygon_3d(polygon: typing.Any, offset: float) -> list[types.Polygon3D]:
        """Offset (inflate/deflate) a closed 3D polygon.

        :param polygon: Input 3D polygon.
        :param offset: Offset distance (positive = grow, negative = shrink).
        :returns: Offset polygons with Z from input.
        """
"#,
    module = "raygeo.geo.shape.polygon3d"
)]
#[pyfunction(name = "offset_polygon_3d")]
fn offset_polygon_3d_py(
    polygon: &Bound<'_, PyAny>,
    offset: f64,
) -> PyResult<Vec<Vec<(f64, f64, f64)>>> {
    let poly = extract_polygon3d(polygon)?;
    let result = offset_polygon_3d(&poly, offset);
    Ok(result.into_iter().map(points3d_to_tuples).collect())
}

pub fn register(shape_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = shape_mod.py();
    let m = PyModule::new(py, "polygon3d")?;

    register_functions!(
        m,
        get_polygons_union_3d_py,
        get_polygons_intersection_3d_py,
        get_polygons_difference_3d_py,
        get_polygons_group_intersection_3d_py,
        get_polygons_group_difference_3d_py,
        offset_polygon_3d_py,
    );

    shape_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.shape.polygon3d", &m)?;
    sys_modules.set_item("raygeo.shape.polygon3d", &m)?;

    Ok(())
}
