pyo3_stub_gen::module_doc!("raygeo.geo.algo.helix", "{}", MODULE_DOC_HELIX);

pub(crate) const MODULE_DOC_HELIX: &str = "\
Helical and conical helical path generation.

Provides generation of 3D helical polylines (cylindrical or conical)
with configurable direction (CW/CCW), pitch, and expansion/reduction.
";

use crate::geo::algo::helix::{self, HelixDirection as RustHelixDirection};
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass_enum, gen_stub_pyfunction};

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "helix")?;
    m.setattr("__doc__", MODULE_DOC_HELIX)?;

    m.add_class::<PyHelixDirection>()?;
    register_functions!(m, generate_helix_py,);

    algo_mod.add_submodule(&m)?;
    let sys_modules = algo_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.algo.helix", &m)?;
    Ok(())
}

#[gen_stub_pyclass_enum]
#[pyclass(
    module = "raygeo.geo.algo.helix",
    name = "HelixDirection",
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyHelixDirection {
    Cw,
    Ccw,
}

impl From<PyHelixDirection> for RustHelixDirection {
    fn from(d: PyHelixDirection) -> Self {
        match d {
            PyHelixDirection::Cw => RustHelixDirection::Cw,
            PyHelixDirection::Ccw => RustHelixDirection::Ccw,
        }
    }
}

#[pymethods]
impl PyHelixDirection {
    fn __repr__(&self) -> String {
        match self {
            PyHelixDirection::Cw => "HelixDirection.Cw".to_string(),
            PyHelixDirection::Ccw => "HelixDirection.Ccw".to_string(),
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def generate_helix_3d(
        center: tuple[float, float],
        start_radius: float,
        end_radius: float,
        z_start: float,
        z_end: float,
        pitch: float,
        direction: HelixDirection,
        angular_step: float = 0.1,
        min_revolutions: int | None = None,
    ) -> list[tuple[float, float, float]]:
        """Generate a 3D helical polyline.

        :param center: Center (x, y) of the helix.
        :param start_radius: Starting radius at z_start.
        :param end_radius: Ending radius at z_end.
        :param z_start: Starting Z height.
        :param z_end: Ending Z height (must be lower than z_start).
        :param pitch: Z descent per full revolution.
        :param direction: CW or CCW revolution.
        :param angular_step: Angular step in radians per vertex (default 0.1).
        :param min_revolutions: Minimum number of revolutions (optional).
        :returns: List of (x, y, z) points approximating the helix.
        :complexity: O(n) time, O(n) space where n = total_angle / angular_step
        """
"#,
    module = "raygeo.geo.algo.helix"
)]
#[pyfunction(name = "generate_helix_3d")]
#[pyo3(signature = (
    center,
    start_radius,
    end_radius,
    z_start,
    z_end,
    pitch,
    direction,
    angular_step = 0.1,
    min_revolutions = None,
))]
#[allow(clippy::too_many_arguments)]
fn generate_helix_py(
    center: (f64, f64),
    start_radius: f64,
    end_radius: f64,
    z_start: f64,
    z_end: f64,
    pitch: f64,
    direction: PyHelixDirection,
    angular_step: f64,
    min_revolutions: Option<u32>,
) -> Vec<(f64, f64, f64)> {
    let opts = helix::HelixOptions {
        center: Point::new(center.0, center.1),
        start_radius,
        end_radius,
        z_start,
        z_end,
        pitch,
        direction: direction.into(),
        angular_step,
        min_revolutions,
    };
    let pts = helix::generate_helix_3d(&opts);
    pts.into_iter().map(|p| (p.x, p.y, p.z)).collect()
}
