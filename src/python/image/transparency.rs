use numpy::IntoPyArray;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::image::transparency;

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def make_transparent_by_brightness(
        rgba: numpy.typing.NDArray[numpy.uint8],
        width: int,
        height: int,
        stride: int,
        threshold: int = 250,
    ) -> None:
        """Clear alpha for bright pixels in an ARGB32 buffer (in-place).

        Pixels with BT.601-weighted brightness >= threshold have their
        alpha channel set to 0.

        :param rgba: Flattened uint8 buffer of shape (stride * height * 4,).
        :param width: Image width in pixels.
        :param height: Image height in pixels.
        :param stride: Row stride in pixels (may be larger than width).
        :param threshold: Brightness threshold (0-255).
        :complexity: O(w*h)
        """
"#,
    module = "raygeo.image"
)]
#[pyfunction(name = "make_transparent_by_brightness")]
#[pyo3(signature = (rgba, width, height, stride, threshold=250))]
fn py_make_transparent_by_brightness(
    py: Python<'_>,
    rgba: &Bound<'_, PyAny>,
    width: usize,
    height: usize,
    stride: usize,
    threshold: u8,
) -> PyResult<()> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (rgba,))?;
    let flat: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    let mut buf = flat;
    transparency::make_transparent_by_brightness(
        &mut buf, width, height, stride, threshold,
    );

    let src = buf.into_pyarray(py);
    numpy.call_method("copyto", (arr, src), None)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def make_transparent_except_color(
        rgba: numpy.typing.NDArray[numpy.uint8],
        width: int,
        height: int,
        stride: int,
        target_r: int,
        target_g: int,
        target_b: int,
    ) -> None:
        """Clear alpha for non-matching pixels in an ARGB32 buffer (in-place).

        Pixels that do not match the target RGB color have their alpha
        channel set to 0.

        :param rgba: Flattened uint8 buffer of shape (stride * height * 4,).
        :param width: Image width in pixels.
        :param height: Image height in pixels.
        :param stride: Row stride in pixels.
        :param target_r: Target red channel value (0-255).
        :param target_g: Target green channel value (0-255).
        :param target_b: Target blue channel value (0-255).
        :complexity: O(w*h)
        """
"#,
    module = "raygeo.image"
)]
#[pyfunction(name = "make_transparent_except_color")]
#[pyo3(signature = (rgba, width, height, stride, target_r, target_g, target_b))]
#[allow(clippy::too_many_arguments)]
fn py_make_transparent_except_color(
    py: Python<'_>,
    rgba: &Bound<'_, PyAny>,
    width: usize,
    height: usize,
    stride: usize,
    target_r: u8,
    target_g: u8,
    target_b: u8,
) -> PyResult<()> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (rgba,))?;
    let flat: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    let mut buf = flat;
    transparency::make_transparent_except_color(
        &mut buf, width, height, stride, target_r, target_g, target_b,
    );

    let src = buf.into_pyarray(py);
    numpy.call_method("copyto", (arr, src), None)?;
    Ok(())
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(
        py_make_transparent_by_brightness,
        m.clone()
    )?)?;
    m.add_function(wrap_pyfunction!(
        py_make_transparent_except_color,
        m.clone()
    )?)?;
    Ok(())
}
