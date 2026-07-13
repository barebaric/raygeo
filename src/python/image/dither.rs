use numpy::IntoPyArray;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::image::dither;

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def apply_floyd_steinberg_dither(
        grayscale: numpy.typing.NDArray[numpy.uint8],
        invert: bool,
    ) -> numpy.typing.NDArray[numpy.uint8]:
        """Apply Floyd-Steinberg error-diffusion dithering.

        :param grayscale: 2D grayscale image as uint8 array.
        :param invert: If True, invert the output (swap black/white).
        :returns: 2D binary uint8 array (values 0 or 1).
        :complexity: O(w*h)
        """
"#,
    module = "raygeo.image.dither"
)]
#[pyfunction(name = "apply_floyd_steinberg_dither")]
fn py_apply_floyd_steinberg_dither(
    py: Python<'_>,
    grayscale: &Bound<'_, PyAny>,
    invert: bool,
) -> PyResult<Py<PyAny>> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (grayscale,))?;
    let shape = arr.getattr("shape")?.extract::<(usize, usize)>()?;
    let height = shape.0;
    let width = shape.1;
    let flat: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    let mut output = vec![0u8; height * width];
    dither::apply_floyd_steinberg_dither(
        &flat,
        width,
        height,
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

    def apply_minimum_run_length(
        binary: numpy.typing.NDArray[numpy.uint8],
        min_run_length: int,
    ) -> numpy.typing.NDArray[numpy.uint8]:
        """Remove binary runs shorter than the given minimum.

        :param binary: 2D binary uint8 array (values 0 or 1).
        :param min_run_length: Minimum run length to keep.
        :returns: 2D binary uint8 array with short runs removed.
        :complexity: O(w*h)
        """
"#,
    module = "raygeo.image.dither"
)]
#[pyfunction(name = "apply_minimum_run_length")]
fn py_apply_minimum_run_length(
    py: Python<'_>,
    binary: &Bound<'_, PyAny>,
    min_run_length: usize,
) -> PyResult<Py<PyAny>> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (binary,))?;
    let shape = arr.getattr("shape")?.extract::<(usize, usize)>()?;
    let height = shape.0;
    let width = shape.1;

    let mut flat: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;
    dither::apply_minimum_run_length(&mut flat, width, height, min_run_length);

    let result = flat.into_pyarray(py);
    let reshaped = result.call_method1("reshape", (height, width))?;
    Ok(reshaped.unbind())
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def apply_bayer_dither(
        grayscale: numpy.typing.NDArray[numpy.uint8],
        bayer_matrix: numpy.typing.NDArray[numpy.float32],
        invert: bool,
        cell_size: int = 1,
    ) -> numpy.typing.NDArray[numpy.uint8]:
        """Apply ordered (Bayer) dithering using a threshold matrix.

        :param grayscale: 2D grayscale image as uint8 array.
        :param bayer_matrix: 2D Bayer threshold matrix as float32.
        :param invert: If True, invert the output.
        :param cell_size: Pixel grouping size for the threshold.
        :returns: 2D binary uint8 array (values 0 or 1).
        :complexity: O(w*h)
        """
"#,
    module = "raygeo.image.dither"
)]
#[pyfunction(name = "apply_bayer_dither")]
#[pyo3(signature = (grayscale, bayer_matrix, invert, cell_size=1))]
fn py_apply_bayer_dither(
    py: Python<'_>,
    grayscale: &Bound<'_, PyAny>,
    bayer_matrix: &Bound<'_, PyAny>,
    invert: bool,
    cell_size: usize,
) -> PyResult<Py<PyAny>> {
    let numpy = py.import("numpy")?;

    let gs_arr = numpy.call_method1("asarray", (grayscale,))?;
    let gs_shape = gs_arr.getattr("shape")?.extract::<(usize, usize)>()?;
    let height = gs_shape.0;
    let width = gs_shape.1;
    let gs_flat: Vec<u8> = gs_arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    let bm_arr = numpy.call_method1("asarray", (bayer_matrix,))?;
    let bm_shape = bm_arr.getattr("shape")?.extract::<(usize, usize)>()?;
    let matrix_size = bm_shape.0;
    let bm_flat: Vec<f32> = bm_arr
        .call_method("astype", ("float32",), None)?
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    let mut output = vec![0u8; height * width];
    dither::apply_bayer_dither(
        &gs_flat,
        width,
        height,
        &bm_flat,
        matrix_size,
        invert,
        cell_size,
        &mut output,
    );

    let result = output.into_pyarray(py);
    let reshaped = result.call_method1("reshape", (height, width))?;
    Ok(reshaped.unbind())
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let sub_mod = PyModule::new(m.py(), "dither")?;
    sub_mod.add_function(wrap_pyfunction!(
        py_apply_floyd_steinberg_dither,
        sub_mod.clone()
    )?)?;
    sub_mod.add_function(wrap_pyfunction!(
        py_apply_minimum_run_length,
        sub_mod.clone()
    )?)?;
    sub_mod.add_function(wrap_pyfunction!(
        py_apply_bayer_dither,
        sub_mod.clone()
    )?)?;
    m.add_submodule(&sub_mod)?;
    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.image.dither", &sub_mod)?;
    Ok(())
}
