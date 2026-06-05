use numpy::IntoPyArray;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::image::convert as rust_convert;

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def rgba_to_grayscale(
        rgba: numpy.typing.NDArray[numpy.uint8],
        width: int,
        height: int,
        stride: int,
    ) -> tuple[numpy.typing.NDArray[numpy.uint8], numpy.typing.NDArray[numpy.float32]]:
        """Convert raw BGRA pixel buffer to grayscale with alpha unpremultiplication.

        Performs proper unpremultiplication of alpha and blends to white
        background for grayscale calculation using BT.601 luminance weights.

        :param rgba: Flattened uint8 buffer of shape (stride * height * 4,).
        :param width: Image width in pixels.
        :param height: Image height in pixels.
        :param stride: Row stride in pixels (may be larger than width).
        :returns: Tuple of (grayscale_uint8, alpha_float32) arrays, each (height, width).
        """
"#,
    module = "raygeo.image"
)]
#[pyfunction(name = "rgba_to_grayscale")]
#[pyo3(signature = (rgba, width, height, stride))]
fn py_rgba_to_grayscale(
    py: Python<'_>,
    rgba: &Bound<'_, PyAny>,
    width: usize,
    height: usize,
    stride: usize,
) -> PyResult<Py<PyAny>> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (rgba,))?;
    let flat: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    let npix = width * height;
    let mut gray = vec![0u8; npix];
    let mut alpha = vec![0.0f32; npix];

    rust_convert::rgba_to_grayscale(
        &flat, width, height, stride, &mut gray, &mut alpha,
    );

    let gray_arr = gray.into_pyarray(py);
    let alpha_arr = alpha.into_pyarray(py);

    let gray_2d = gray_arr.call_method1("reshape", (height, width))?;
    let alpha_2d = alpha_arr.call_method1("reshape", (height, width))?;

    Ok(PyTuple::new(py, [gray_2d.as_any(), alpha_2d.as_any()])?
        .into_any()
        .unbind())
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def rgba_to_binary(
        rgba: numpy.typing.NDArray[numpy.uint8],
        width: int,
        height: int,
        stride: int,
        threshold: int = 128,
        invert: bool = False,
    ) -> numpy.typing.NDArray[numpy.uint8]:
        """Convert raw BGRA pixel buffer to binary image using thresholding.

        Transparent pixels (alpha == 0) are always treated as white (0).

        :param rgba: Flattened uint8 buffer of shape (stride * height * 4,).
        :param width: Image width in pixels.
        :param height: Image height in pixels.
        :param stride: Row stride in pixels.
        :param threshold: Brightness value (0-255) for binarization.
        :param invert: If True, pixels above threshold become black (1).
        :returns: 2D binary uint8 array (values 0 or 1) with shape (height, width).
        """
"#,
    module = "raygeo.image"
)]
#[pyfunction(name = "rgba_to_binary")]
#[pyo3(signature = (rgba, width, height, stride, threshold=128, invert=false))]
fn py_rgba_to_binary(
    py: Python<'_>,
    rgba: &Bound<'_, PyAny>,
    width: usize,
    height: usize,
    stride: usize,
    threshold: u8,
    invert: bool,
) -> PyResult<Py<PyAny>> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (rgba,))?;
    let flat: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    let mut output = vec![0u8; width * height];
    rust_convert::rgba_to_binary(
        &flat,
        width,
        height,
        stride,
        threshold,
        invert,
        &mut output,
    );

    let result = output.into_pyarray(py);
    let reshaped = result.call_method1("reshape", (height, width))?;
    Ok(reshaped.unbind())
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def rgba_to_grayscale_inplace(
        rgba: numpy.typing.NDArray[numpy.uint8],
        width: int,
        height: int,
        stride: int,
    ) -> None:
        """Convert raw BGRA pixel buffer to grayscale in place.

        Modifies the buffer directly, converting BGR channels to grayscale
        while preserving the alpha channel.

        :param rgba: Flattened uint8 buffer of shape (stride * height * 4,).
        :param width: Image width in pixels.
        :param height: Image height in pixels.
        :param stride: Row stride in pixels.
        """
"#,
    module = "raygeo.image"
)]
#[pyfunction(name = "rgba_to_grayscale_inplace")]
#[pyo3(signature = (rgba, width, height, stride))]
fn py_rgba_to_grayscale_inplace(
    py: Python<'_>,
    rgba: &Bound<'_, PyAny>,
    width: usize,
    height: usize,
    stride: usize,
) -> PyResult<()> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (rgba,))?;
    let flat: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    let mut buf = flat;
    rust_convert::rgba_to_grayscale_inplace(&mut buf, width, height, stride);

    let src = buf.into_pyarray(py);
    numpy.call_method("copyto", (arr, src), None)?;
    Ok(())
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_rgba_to_grayscale, m.clone())?)?;
    m.add_function(wrap_pyfunction!(py_rgba_to_binary, m.clone())?)?;
    m.add_function(wrap_pyfunction!(py_rgba_to_grayscale_inplace, m.clone())?)?;
    Ok(())
}
