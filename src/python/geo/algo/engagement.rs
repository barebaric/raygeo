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
        disk_segment_area_py,
        point_engagement_py,
        angular_engagement_py,
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
    def get_disk_segment_area(
        x: float,
        r: float,
    ) -> float:
        """Area under 2*sqrt(r²-x²) from x to r.

        Equivalent to the area of the circular segment to the right of
        the vertical line at ``x`` for a disk of radius ``r`` centred
        at the origin.

        :param x: Left boundary of the segment.
        :param r: Disk radius.
        :returns: Area of the circular segment.
        """
    "#,
    module = "raygeo.geo.algo.engagement"
)]
#[pyfunction(name = "get_disk_segment_area")]
fn disk_segment_area_py(x: f64, r: f64) -> f64 {
    engagement::get_disk_segment_area(x, r)
}

#[gen_stub_pyfunction(
    python = r#"
    def get_point_engagement(
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
#[pyfunction(name = "get_point_engagement")]
fn point_engagement_py(
    center: (f64, f64),
    radius: f64,
    fragments: Vec<Vec<(f64, f64)>>,
) -> (f64, f64, f64) {
    let frags = polygons_from_tuples(fragments);
    let e = engagement::get_point_engagement(
        Point::new(center.0, center.1),
        radius,
        &frags,
    );
    (e.angle, e.area, e.chord_depth)
}

#[gen_stub_pyfunction(
    python = r#"
    def get_angular_engagement(
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
#[pyfunction(name = "get_angular_engagement")]
fn angular_engagement_py(
    center: (f64, f64),
    radius: f64,
    fragments: Vec<Vec<(f64, f64)>>,
) -> f64 {
    let frags = polygons_from_tuples(fragments);
    engagement::get_angular_engagement(
        Point::new(center.0, center.1),
        radius,
        &frags,
    )
}
