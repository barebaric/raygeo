pyo3_stub_gen::module_doc!("raygeo.geo.algo.spiral", "{}", MODULE_DOC_SPIRAL);

pub(crate) const MODULE_DOC_SPIRAL: &str = "\
Flat Archimedean spiral path generation.

Produces a constant-Z spiral from a start radius to an end radius with
configurable direction, revolutions, and angular resolution. Used for
widening after a helical entry (EntryStrategy.Helix expand_to_diameter).
";

use crate::geo::algo::spiral;
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "spiral")?;
    m.setattr("__doc__", MODULE_DOC_SPIRAL)?;

    register_functions!(m, generate_spiral_py,);

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types
    import raygeo.geo.algo.helix as helix

    def generate_spiral(
        center: tuple[float, float],
        z: float,
        start_radius: float,
        end_radius: float,
        revolutions: float,
        direction: helix.HelixDirection,
        angular_step: float = 0.1,
    ) -> list[tuple[float, float, float]]:
        """Generate a flat Archimedean spiral at constant Z.

        The spiral sweeps linearly from `start_radius` to `end_radius`
        over the requested number of revolutions.

        :param center: Center (x, y) of the spiral.
        :param z: Constant Z height of the spiral.
        :param start_radius: Starting radius.
        :param end_radius: Ending radius.
        :param revolutions: Total turns (may be fractional).
        :param direction: CW or CCW revolution.
        :param angular_step: Angular step in radians per vertex (default 0.1).
        :returns: List of (x, y, z) points approximating the spiral.
        :complexity: O(n) time, O(n) space where n = total_angle / angular_step
        """
"#,
    module = "raygeo.geo.algo.spiral"
)]
#[pyfunction(name = "generate_spiral")]
#[pyo3(signature = (
    center,
    z,
    start_radius,
    end_radius,
    revolutions,
    direction,
    angular_step = 0.1,
))]
fn generate_spiral_py(
    center: (f64, f64),
    z: f64,
    start_radius: f64,
    end_radius: f64,
    revolutions: f64,
    direction: crate::python::geo::algo::helix::PyHelixDirection,
    angular_step: f64,
) -> Vec<(f64, f64, f64)> {
    let opts = spiral::SpiralOptions {
        center: Point::new(center.0, center.1),
        z,
        start_radius,
        end_radius,
        revolutions,
        direction: direction.into(),
        angular_step,
    };
    let pts = spiral::generate_spiral(&opts);
    pts.into_iter().map(|p| (p.x, p.y, p.z)).collect()
}
