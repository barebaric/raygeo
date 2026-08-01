pyo3_stub_gen::module_doc!("raygeo.geo.algo.hull", "{}", MODULE_DOC_HULL);

pub(crate) const MODULE_DOC_HULL: &str = "\
Hull computation from binary images.

Provides convex and concave (shrink-wrap) hull generation from boolean images, \
using contour tracing and Bézier gravity attraction. \
Coordinates are returned in image pixel space (y increases downward).";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "hull")?;
    m.setattr("__doc__", MODULE_DOC_HULL)?;

    register_functions!(
        m,
        get_enclosing_hull_py,
        get_hulls_from_image_py,
        get_concave_hull_py,
    );

    algo_mod.add_submodule(&m)?;
    let sys_modules = algo_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.algo.hull", &m)?;
    Ok(())
}

fn points_to_geometry(
    pts: &[crate::types::Point],
) -> Option<crate::geo::geometry::Geometry> {
    if pts.len() < 2 {
        return None;
    }
    let mut geo = crate::geo::geometry::Geometry::new();
    geo.move_to(pts[0].x, pts[0].y, 0.0);
    for p in &pts[1..] {
        geo.line_to(p.x, p.y, 0.0);
    }
    geo.close_path();
    Some(geo)
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import raygeo

    def get_enclosing_hull(
        boolean_image: numpy.ndarray,
    ) -> raygeo.geo.Geometry | None:
        """Compute a single convex hull enclosing all content.

        :param boolean_image: 2D boolean array.
        :returns: Convex hull as Geometry in pixel coords, or None.
        :complexity: O(w*h + n log n) time, O(n) space where w*h is the image size and n the number of contour points
        """
"#,
    module = "raygeo.geo.algo.hull"
)]
#[pyfunction(name = "get_enclosing_hull")]
fn get_enclosing_hull_py(
    py: Python<'_>,
    boolean_image: &Bound<'_, PyAny>,
) -> PyResult<Option<super::super::Geometry>> {
    let (flat, h, w) = extract_bool_image(py, boolean_image)?;
    let pts = crate::geo::algo::hull::get_enclosing_hull(&flat, w, h);
    Ok(pts
        .and_then(|p| points_to_geometry(&p))
        .map(|g| super::super::Geometry { inner: g }))
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import raygeo

    def get_hulls_from_image(
        boolean_image: numpy.ndarray,
    ) -> list[raygeo.geo.Geometry]:
        """Compute a separate convex hull for each distinct component.

        :param boolean_image: 2D boolean array.
        :returns: List of Geometry objects in pixel coords.
        :complexity: O(w*h + n log n) time, O(n) space where w*h is the image size and n the total number of contour points
        """
"#,
    module = "raygeo.geo.algo.hull"
)]
#[pyfunction(name = "get_hulls_from_image")]
fn get_hulls_from_image_py(
    py: Python<'_>,
    boolean_image: &Bound<'_, PyAny>,
) -> PyResult<Vec<super::super::Geometry>> {
    let (flat, h, w) = extract_bool_image(py, boolean_image)?;
    let hulls = crate::geo::algo::hull::get_hulls_from_image(&flat, w, h);
    Ok(hulls
        .into_iter()
        .filter_map(|pts| {
            points_to_geometry(&pts)
                .map(|g| super::super::Geometry { inner: g })
        })
        .collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import raygeo

    def get_concave_hull(
        boolean_image: numpy.ndarray,
        gravity: float = 0.1,
    ) -> raygeo.geo.Geometry | None:
        """Compute a concave (shrink-wrap) hull with Bézier gravity.

        :param boolean_image: 2D boolean array.
        :param gravity: Shrink-wrap factor 0.0-1.0. 0 gives convex hull.
        :returns: Concave hull as Geometry in pixel coords, or None.
        :complexity: O(w*h + n log n + n * g) time, O(n) space where w*h is the image size, n the number of contour points, and g the number of gravity iterations
        """
"#,
    module = "raygeo.geo.algo.hull"
)]
#[pyfunction(name = "get_concave_hull")]
#[pyo3(signature = (boolean_image, gravity=0.1))]
fn get_concave_hull_py(
    py: Python<'_>,
    boolean_image: &Bound<'_, PyAny>,
    gravity: f64,
) -> PyResult<Option<super::super::Geometry>> {
    let (flat, h, w) = extract_bool_image(py, boolean_image)?;
    let pts = crate::geo::algo::hull::get_concave_hull(&flat, w, h, gravity);
    Ok(pts
        .and_then(|p| points_to_geometry(&p))
        .map(|g| super::super::Geometry { inner: g }))
}

fn extract_bool_image(
    py: Python<'_>,
    obj: &Bound<'_, PyAny>,
) -> PyResult<(Vec<u8>, usize, usize)> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (obj,))?;
    let shape: (usize, usize) = arr.getattr("shape")?.extract()?;
    let flat: Vec<u8> = arr
        .call_method("astype", ("uint8",), None)?
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;
    let nonzero: Vec<u8> =
        flat.iter().map(|&v| if v != 0 { 1 } else { 0 }).collect();
    Ok((nonzero, shape.0, shape.1))
}
