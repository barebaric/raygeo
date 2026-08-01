pyo3_stub_gen::module_doc!("raygeo.geo.shape.rect", "{}", MODULE_DOC_RECT);

pub(crate) const MODULE_DOC_RECT: &str = "\
Rectangle intersection and containment tests.

Provides functions to test whether two axis-aligned rectangles intersect,
whether one rectangle fully contains another, and utilities for computing
the union bounding rectangle of multiple geometries.
";

use crate::geo::shape::line::is_point_inside_rect;
use crate::geo::shape::rect::do_rects_intersect;
use crate::geo::shape::rect::does_rect_contain_rect;
use crate::geo::types::{Point, Rect};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(shape_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = shape_mod.py();
    let m = PyModule::new(py, "rect")?;
    m.setattr("__doc__", MODULE_DOC_RECT)?;

    register_functions!(
        m,
        is_point_inside_rect_py,
        does_rect_contain_rect_py,
        do_rects_intersect_py,
        get_combined_rect_py,
    );

    shape_mod.add_submodule(&m)?;
    let sys_modules = shape_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.shape.rect", &m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def is_point_inside_rect(
        point: types.Point,
        rect: types.Rect,
    ) -> bool:
        """Check if a point is inside a rectangle.

        :param point: Point (x, y) to test.
        :param rect: Rectangle (x_min, y_min, x_max, y_max).
        :returns: True if the point is inside the rectangle.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.rect"
)]
#[pyfunction(name = "is_point_inside_rect")]
fn is_point_inside_rect_py(
    point: (f64, f64),
    rect: (f64, f64, f64, f64),
) -> bool {
    is_point_inside_rect(
        Point::new(point.0, point.1),
        Rect::new(rect.0, rect.1, rect.2, rect.3),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def does_rect_contain_rect(
        outer: types.Rect,
        inner: types.Rect,
    ) -> bool:
        """Check if one rectangle contains another.

        :param outer: Outer rectangle (x_min, y_min, x_max, y_max).
        :param inner: Inner rectangle (x_min, y_min, x_max, y_max).
        :returns: True if outer fully contains inner.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.rect"
)]
#[pyfunction(name = "does_rect_contain_rect")]
fn does_rect_contain_rect_py(
    outer: (f64, f64, f64, f64),
    inner: (f64, f64, f64, f64),
) -> bool {
    does_rect_contain_rect(
        Rect::new(outer.0, outer.1, outer.2, outer.3),
        Rect::new(inner.0, inner.1, inner.2, inner.3),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def do_rects_intersect(
        r1: types.Rect,
        r2: types.Rect,
    ) -> bool:
        """Check if two rectangles intersect.

        :param r1: First rectangle (x_min, y_min, x_max, y_max).
        :param r2: Second rectangle (x_min, y_min, x_max, y_max).
        :returns: True if the rectangles intersect.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.rect"
)]
#[pyfunction(name = "do_rects_intersect")]
fn do_rects_intersect_py(
    r1: (f64, f64, f64, f64),
    r2: (f64, f64, f64, f64),
) -> bool {
    do_rects_intersect(
        Rect::new(r1.0, r1.1, r1.2, r1.3),
        Rect::new(r2.0, r2.1, r2.2, r2.3),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo
    import raygeo.geo.types

    def get_combined_rect(
        geometries: list[raygeo.geo.Geometry],
    ) -> types.Rect:
        """Compute the union bounding box of multiple geometries.

        :param geometries: List of Geometry objects.
        :returns: Union bounding rectangle (x_min, y_min, x_max, y_max).
        :complexity: O(n) time, O(1) space where n is the number of geometries
        """
"#,
    module = "raygeo.geo.shape.rect"
)]
#[pyfunction(name = "get_combined_rect")]
fn get_combined_rect_py(
    geometries: &Bound<'_, PyAny>,
) -> PyResult<(f64, f64, f64, f64)> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for item in geometries.try_iter()? {
        let item = item?;
        let geo: &Bound<'_, super::super::geometry::Geometry> =
            item.cast().map_err(|e| {
                pyo3::exceptions::PyTypeError::new_err(format!(
                    "Expected Geometry object: {}",
                    e
                ))
            })?;
        let r = geo.borrow().inner.rect();
        min_x = min_x.min(r.min.x);
        min_y = min_y.min(r.min.y);
        max_x = max_x.max(r.max.x);
        max_y = max_y.max(r.max.y);
    }
    if min_x.is_infinite() {
        return Ok((0.0, 0.0, 0.0, 0.0));
    }
    Ok((min_x, min_y, max_x, max_y))
}
