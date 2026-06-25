pyo3_stub_gen::module_doc!("raygeo.geo.algo.engagement", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Circle-boundary overlap (engagement) metrics.
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::algo::engagement;
use crate::python::geo::flex_point::polygons_from_tuples;
use crate::types::Point;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "engagement")?;
    m.setattr("__doc__", MODULE_DOC)?;
    register_functions!(
        m,
        compute_engagement_py,
        point_engagement_py,
        angular_engagement_py,
        cut_area_py,
    );
    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    def compute_engagement(
        d_to_boundary: float,
        radius: float,
    ) -> tuple[float, float, float]:
        """Compute engagement angle, area, and chord depth.

        :param d_to_boundary: Signed distance from the point to the nearest boundary
            (mm).  Positive = outside the boundary.
        :param radius: Disk radius (mm).
        :returns: ``(angle_rad, area, chord_depth)``.
        """
    "#,
    module = "raygeo.geo.algo.engagement"
)]
#[pyfunction(name = "compute_engagement")]
fn compute_engagement_py(d_to_boundary: f64, radius: f64) -> (f64, f64, f64) {
    let e = engagement::compute_engagement(d_to_boundary, radius);
    (e.angle, e.area, e.chord_depth)
}

#[gen_stub_pyfunction(
    python = r#"
    def point_engagement(
        center: tuple[float, float],
        radius: float,
        fragments: list[list[tuple[float, float]]],
    ) -> tuple[float, float, float]:
        """Engagement angle, area, and chord depth at a disk centre.

        :param center: Disk centre ``(x, y)``.
        :param radius: Disk radius (mm).
        :param fragments: List of polygons (cleared fragments).
        :returns: ``(angle_rad, area, chord_depth)``.
        """
    "#,
    module = "raygeo.geo.algo.engagement"
)]
#[pyfunction(name = "point_engagement")]
fn point_engagement_py(
    center: (f64, f64),
    radius: f64,
    fragments: Vec<Vec<(f64, f64)>>,
) -> (f64, f64, f64) {
    let frags = polygons_from_tuples(fragments);
    let e = engagement::point_engagement(
        Point::new(center.0, center.1),
        radius,
        &frags,
    );
    (e.angle, e.area, e.chord_depth)
}

#[gen_stub_pyfunction(
    python = r#"
    def angular_engagement(
        center: tuple[float, float],
        radius: float,
        fragments: list[list[tuple[float, float]]],
    ) -> float:
        """Angular engagement (exact circle–polygon intersection).

        Returns uncleared angular extent in ``[0, 2π]``.

        :param center: Disk centre ``(x, y)``.
        :param radius: Disk radius (mm).
        :param fragments: List of polygons (cleared fragments).
        :returns: Angular engagement in radians.
        """
    "#,
    module = "raygeo.geo.algo.engagement"
)]
#[pyfunction(name = "angular_engagement")]
fn angular_engagement_py(
    center: (f64, f64),
    radius: f64,
    fragments: Vec<Vec<(f64, f64)>>,
) -> f64 {
    let frags = polygons_from_tuples(fragments);
    engagement::angular_engagement(
        Point::new(center.0, center.1),
        radius,
        &frags,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    def cut_area(
        c1: tuple[float, float],
        c2: tuple[float, float],
        radius: float,
        fragments: list[list[tuple[float, float]]],
    ) -> float:
        """Incremental cut area when moving from c1 to c2.

        The crescent ``disk(c2) − disk(c1)`` is intersected against
        *fragments* and the fresh (uncleared) area is returned.

        :param c1: Previous centre ``(x, y)``.
        :param c2: Next centre ``(x, y)``.
        :param radius: Disk radius (mm).
        :param fragments: List of polygons (cleared fragments).
        :returns: Fresh cut area (mm²).
        """
    "#,
    module = "raygeo.geo.algo.engagement"
)]
#[pyfunction(name = "cut_area")]
fn cut_area_py(
    c1: (f64, f64),
    c2: (f64, f64),
    radius: f64,
    fragments: Vec<Vec<(f64, f64)>>,
) -> f64 {
    let frags = polygons_from_tuples(fragments);
    engagement::cut_area(
        Point::new(c1.0, c1.1),
        Point::new(c2.0, c2.1),
        radius,
        &frags,
    )
}
