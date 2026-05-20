use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;
use numpy::PyArray2;

use raygeo_core::geo::math::apply_affine_transform_to_array;

fn to_data_array(data: Vec<Vec<f64>>) -> Vec<[f64; 8]> {
    data.into_iter()
        .map(|r| {
            let mut a = [0.0; 8];
            let len = r.len().min(8);
            a[..len].copy_from_slice(&r[..len]);
            a
        })
        .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    def apply_affine_transform_to_array(
        data: Sequence[Sequence[float]],
        matrix: Sequence[Sequence[float]],
    ) -> Any:
        """Apply an affine transform to path data.

        :param data: Array of command data.
        :param matrix: 4x4 affine transformation matrix.
        :returns: Numpy array of transformed data.
        """
"#,
    module = "raygeo.geo.math"
)]
#[pyfunction(name = "apply_affine_transform_to_array")]
fn apply_affine_transform_to_array_py(
    py: Python<'_>,
    data: Vec<Vec<f64>>,
    matrix: Vec<Vec<f64>>,
) -> Bound<'_, pyo3::types::PyAny> {
    let arr = to_data_array(data);
    let mat: [[f64; 4]; 4] = [
        [matrix[0][0], matrix[0][1], matrix[0][2], matrix[0][3]],
        [matrix[1][0], matrix[1][1], matrix[1][2], matrix[1][3]],
        [matrix[2][0], matrix[2][1], matrix[2][2], matrix[2][3]],
        [matrix[3][0], matrix[3][1], matrix[3][2], matrix[3][3]],
    ];
    let result = apply_affine_transform_to_array(&arr, &mat);
    let vecs: Vec<Vec<f64>> = result.into_iter().map(|r| r.to_vec()).collect();
    if vecs.is_empty() {
        return PyArray2::<f64>::zeros(py, [0, 8], false).as_any().clone();
    }
    PyArray2::<f64>::from_vec2(py, &vecs)
        .expect("failed to create numpy array")
        .as_any()
        .clone()
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let math_mod = PyModule::new(py, "math")?;

    math_mod.add_function(wrap_pyfunction!(
        apply_affine_transform_to_array_py,
        math_mod.clone()
    )?)?;

    m.add_submodule(&math_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.math", &math_mod)?;
    sys_modules.set_item("raygeo.math", &math_mod)?;

    Ok(())
}
