use numpy::IntoPyArray;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::image::srgb as rust_srgb;

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def srgb_to_linear(
        array: numpy.typing.NDArray[numpy.uint8],
    ) -> numpy.typing.NDArray[numpy.float32]:
        """Convert sRGB pixel values to linear light values.

        :param array: Input array of sRGB uint8 values.
        :returns: Array of linear float32 values with the same shape.
        """
"#,
    module = "raygeo.image"
)]
#[pyfunction(name = "srgb_to_linear")]
fn py_srgb_to_linear(
    py: Python<'_>,
    array: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (array,))?;
    let flat: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    let mut output = vec![0.0f32; flat.len()];
    rust_srgb::srgb_to_linear(&flat, &mut output);

    let shape = arr.getattr("shape")?.extract::<Vec<usize>>()?;
    let result = output.into_pyarray(py);
    let reshaped = result.call_method1("reshape", (shape,))?;
    Ok(reshaped.unbind())
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def linear_to_srgb(
        array: numpy.typing.NDArray[numpy.float32],
        dither: bool = False,
    ) -> numpy.typing.NDArray[numpy.uint8]:
        """Convert linear light values to sRGB pixel values.

        :param array: Input array of linear float32 values in [0, 1].
        :param dither: Apply dithering to reduce banding artifacts.
        :returns: Array of sRGB uint8 values with the same shape.
        """
"#,
    module = "raygeo.image"
)]
#[pyfunction(name = "linear_to_srgb")]
#[pyo3(signature = (array, dither=false))]
fn py_linear_to_srgb(
    py: Python<'_>,
    array: &Bound<'_, PyAny>,
    dither: bool,
) -> PyResult<Py<PyAny>> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (array,))?;
    let flat: Vec<f32> = arr
        .call_method("astype", ("float32",), None)?
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    let mut output = vec![0u8; flat.len()];
    if dither {
        let rng = py.import("numpy.random")?.call_method0("default_rng")?;
        let noise: Vec<f32> = rng
            .call_method("uniform", (-0.5_f32, 0.5_f32, flat.len()), None)?
            .call_method0("tolist")?
            .extract()?;
        rust_srgb::linear_to_srgb_dithered(&flat, &mut output, &noise);
    } else {
        rust_srgb::linear_to_srgb(&flat, &mut output);
    }

    let shape = arr.getattr("shape")?.extract::<Vec<usize>>()?;
    let result = output.into_pyarray(py);
    let reshaped = result.call_method1("reshape", (shape,))?;
    Ok(reshaped.unbind())
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_srgb_to_linear, m.clone())?)?;
    m.add_function(wrap_pyfunction!(py_linear_to_srgb, m.clone())?)?;
    Ok(())
}
