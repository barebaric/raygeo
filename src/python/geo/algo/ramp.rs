pyo3_stub_gen::module_doc!("raygeo.geo.algo.ramp", "{}", MODULE_DOC_RAMP);

pub(crate) const MODULE_DOC_RAMP: &str = "\
Ramp entry path generation for milling.

Provides linear and zig-zag ramp generation for tool entry into material,
with automatic extension when the ramp angle exceeds the maximum.
";

use crate::geo::algo::ramp::{self, RampStyle as RustRampStyle};
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass_enum, gen_stub_pyfunction};

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "ramp")?;
    m.setattr("__doc__", MODULE_DOC_RAMP)?;

    m.add_class::<PyRampStyle>()?;
    register_functions!(m, generate_ramp_py,);

    algo_mod.add_submodule(&m)?;
    let sys_modules = algo_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.algo.ramp", &m)?;
    Ok(())
}

#[gen_stub_pyclass_enum]
#[pyclass(module = "raygeo.geo.algo.ramp", name = "RampStyle", from_py_object)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyRampStyle {
    Linear,
    ZigZag,
}

impl From<PyRampStyle> for RustRampStyle {
    fn from(s: PyRampStyle) -> Self {
        match s {
            PyRampStyle::Linear => RustRampStyle::Linear,
            PyRampStyle::ZigZag => RustRampStyle::ZigZag,
        }
    }
}

#[pymethods]
impl PyRampStyle {
    fn __repr__(&self) -> String {
        match self {
            PyRampStyle::Linear => "RampStyle.Linear".to_string(),
            PyRampStyle::ZigZag => "RampStyle.ZigZag".to_string(),
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

    def generate_ramp_3d(
        start: tuple[float, float],
        end: tuple[float, float],
        z_start: float,
        z_end: float,
        max_ramp_angle_deg: float = 45.0,
        style: RampStyle = RampStyle.Linear,
        lateral_amplitude: float = 1.0,
    ) -> list[tuple[float, float, float]]:
        """Generate a ramp entry polyline.

        If the direct ramp angle exceeds max_ramp_angle_deg, the ramp is
        extended in both directions along the same line.

        :param start: Start XY position.
        :param end: End XY position.
        :param z_start: Starting Z height.
        :param z_end: Ending Z height (must be lower than z_start).
        :param max_ramp_angle_deg: Maximum allowed ramp angle in degrees (default 45).
        :param style: Ramp style — Linear or ZigZag (default Linear).
        :param lateral_amplitude: Lateral oscillation amplitude for ZigZag (default 1.0).
        :returns: List of (x, y, z) points along the ramp.
        :complexity: O(n) time, O(n) space where n is proportional to path length
        """
"#,
    module = "raygeo.geo.algo.ramp"
)]
#[pyfunction(name = "generate_ramp_3d")]
#[pyo3(signature = (
    start,
    end,
    z_start,
    z_end,
    max_ramp_angle_deg = 45.0,
    style = PyRampStyle::Linear,
    lateral_amplitude = 1.0,
))]
fn generate_ramp_py(
    start: (f64, f64),
    end: (f64, f64),
    z_start: f64,
    z_end: f64,
    max_ramp_angle_deg: f64,
    style: PyRampStyle,
    lateral_amplitude: f64,
) -> Vec<(f64, f64, f64)> {
    let opts = ramp::RampOptions {
        start: Point::new(start.0, start.1),
        end: Point::new(end.0, end.1),
        z_start,
        z_end,
        max_ramp_angle_deg,
        style: style.into(),
        lateral_amplitude,
    };
    let pts = ramp::generate_ramp_3d(&opts);
    pts.into_iter().map(|p| (p.x, p.y, p.z)).collect()
}
