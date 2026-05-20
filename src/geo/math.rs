use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub(crate) const MODULE_DOC: &str = "\
Matrix math utilities for geometric transformations.

Provides matrix multiplication for 4x4 affine transformation matrices,
which are used by Geometry.transform() and Ops.transform() to apply
translation, rotation, scaling, and shearing to paths.
";

use raygeo_core::geo::math::mat4_mul;

#[gen_stub_pyfunction(
    python = r#"
    def mat4_mul(
        a: Sequence[Sequence[float]],
        b: Sequence[Sequence[float]],
    ) -> list[list[float]]:
        """Multiply two 4x4 matrices.

        :param a: First 4x4 matrix.
        :param b: Second 4x4 matrix.
        :returns: Resulting 4x4 matrix.
        """
"#,
    module = "raygeo.geo.math"
)]
#[pyfunction(name = "mat4_mul")]
fn mat4_mul_py(
    a: Vec<Vec<f64>>,
    b: Vec<Vec<f64>>,
) -> Vec<Vec<f64>> {
    let mat_a: [[f64; 4]; 4] = [
        [a[0][0], a[0][1], a[0][2], a[0][3]],
        [a[1][0], a[1][1], a[1][2], a[1][3]],
        [a[2][0], a[2][1], a[2][2], a[2][3]],
        [a[3][0], a[3][1], a[3][2], a[3][3]],
    ];
    let mat_b: [[f64; 4]; 4] = [
        [b[0][0], b[0][1], b[0][2], b[0][3]],
        [b[1][0], b[1][1], b[1][2], b[1][3]],
        [b[2][0], b[2][1], b[2][2], b[2][3]],
        [b[3][0], b[3][1], b[3][2], b[3][3]],
    ];
    let result = mat4_mul(&mat_a, &mat_b);
    result.iter().map(|row| row.to_vec()).collect()
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let math_mod = PyModule::new(py, "math")?;
    math_mod.setattr("__doc__", MODULE_DOC)?;

    math_mod.add_function(wrap_pyfunction!(
        mat4_mul_py,
        math_mod.clone()
    )?)?;

    m.add_submodule(&math_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.math", &math_mod)?;
    sys_modules.set_item("raygeo.math", &math_mod)?;

    Ok(())
}
