use numpy::IntoPyArray;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::image::dither;

fn extract_grayscale(
    py: Python<'_>,
    grayscale: &Bound<'_, PyAny>,
) -> PyResult<(usize, usize, Vec<u8>)> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (grayscale,))?;
    let shape = arr.getattr("shape")?.extract::<(usize, usize)>()?;
    let flat: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;
    Ok((shape.0, shape.1, flat))
}

fn reshape_output(
    py: Python<'_>,
    output: Vec<u8>,
    height: usize,
    width: usize,
) -> PyResult<Py<PyAny>> {
    let result = output.into_pyarray(py);
    let reshaped = result.call_method1("reshape", (height, width))?;
    Ok(reshaped.unbind())
}

fn checked_pixels_per_mm(pixels_per_mm: (f64, f64)) -> PyResult<()> {
    if pixels_per_mm.0 <= 0.0 || pixels_per_mm.1 <= 0.0 {
        return Err(PyValueError::new_err(format!(
            "pixels_per_mm must be positive, got {:?}",
            pixels_per_mm
        )));
    }
    Ok(())
}

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

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def apply_error_diffusion_dither(
        grayscale: numpy.typing.NDArray[numpy.uint8],
        kernel: str,
        invert: bool = False,
        serpentine: bool = False,
    ) -> numpy.typing.NDArray[numpy.uint8]:
        """Error-diffusion dithering with a named kernel.

        Dithers in linear light. Available kernels: atkinson, burkes,
        floyd_steinberg, jarvis_judice_ninke, sierra, sierra_2row,
        sierra_lite, stucki.

        :param grayscale: 2D grayscale image as uint8 array.
        :param kernel: Name of the error-diffusion kernel to use.
        :param invert: If True, invert the output (swap black/white).
        :param serpentine: Scan odd rows right-to-left, mirroring the
            kernel. Avoids directional artifacts.
        :returns: 2D binary uint8 array (values 0 or 1, 1 marks dark).
        :raises ValueError: If the kernel name is not recognized.
        :complexity: O(w*h)
        """
"#,
    module = "raygeo.image.dither"
)]
#[pyfunction(name = "apply_error_diffusion_dither")]
#[pyo3(signature = (grayscale, kernel, invert=false, serpentine=false))]
fn py_apply_error_diffusion_dither(
    py: Python<'_>,
    grayscale: &Bound<'_, PyAny>,
    kernel: &str,
    invert: bool,
    serpentine: bool,
) -> PyResult<Py<PyAny>> {
    let kernel_impl = match dither::diffusion_kernel_by_name(kernel) {
        Some(kernel_impl) => kernel_impl,
        None => {
            return Err(PyValueError::new_err(format!(
                "Unknown dither kernel '{}'. Available kernels: {}",
                kernel,
                dither::DIFFUSION_KERNEL_NAMES.join(", ")
            )));
        }
    };
    let (height, width, flat) = extract_grayscale(py, grayscale)?;

    let mut output = vec![0u8; height * width];
    dither::apply_error_diffusion_dither(
        &flat,
        width,
        height,
        kernel_impl,
        invert,
        serpentine,
        &mut output,
    );

    reshape_output(py, output, height, width)
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def apply_halftone_dither(
        grayscale: numpy.typing.NDArray[numpy.uint8],
        cell_size_mm: float,
        angle_degrees: float,
        pixels_per_mm: tuple[float, float] = (1.0, 1.0),
        invert: bool = False,
    ) -> numpy.typing.NDArray[numpy.uint8]:
        """Amplitude-modulated halftone screen with round dots.

        The screen is laid out in millimetres, so dots stay round on
        non-square pixels. Dot area tracks darkness: a pixel is set
        when its brightness is below the share of the cell that the
        spot function ranks at or under it.

        :param grayscale: 2D grayscale image as uint8 array.
        :param cell_size_mm: Distance between dot centres in mm.
            Non-positive values fall back to one-pixel cells.
        :param angle_degrees: Rotation of the dot grid in degrees.
        :param pixels_per_mm: (x, y) image resolution in pixels per mm.
        :param invert: If True, invert the output (swap black/white).
        :returns: 2D binary uint8 array (values 0 or 1, 1 marks dark).
        :raises ValueError: If pixels_per_mm is not positive.
        :complexity: O(w*h)
        """
"#,
    module = "raygeo.image.dither"
)]
#[pyfunction(name = "apply_halftone_dither")]
#[pyo3(signature = (grayscale, cell_size_mm, angle_degrees, pixels_per_mm=(1.0, 1.0), invert=false))]
fn py_apply_halftone_dither(
    py: Python<'_>,
    grayscale: &Bound<'_, PyAny>,
    cell_size_mm: f64,
    angle_degrees: f64,
    pixels_per_mm: (f64, f64),
    invert: bool,
) -> PyResult<Py<PyAny>> {
    checked_pixels_per_mm(pixels_per_mm)?;
    let (height, width, flat) = extract_grayscale(py, grayscale)?;

    let mut output = vec![0u8; height * width];
    dither::apply_halftone_dither(
        &flat,
        width,
        height,
        cell_size_mm,
        angle_degrees,
        pixels_per_mm.0,
        pixels_per_mm.1,
        invert,
        &mut output,
    );

    reshape_output(py, output, height, width)
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def apply_newsprint_dither(
        grayscale: numpy.typing.NDArray[numpy.uint8],
        cell_size: int = 1,
        pixels_per_mm: tuple[float, float] = (1.0, 1.0),
        invert: bool = False,
    ) -> numpy.typing.NDArray[numpy.uint8]:
        """Clustered-dot ordered dithering (newsprint screen).

        Tiles an 8x8 threshold matrix whose dots sit on a 45 degree
        lattice. Cells are kept square in millimetres using
        pixels_per_mm, so the screen does not stretch on non-square
        pixels.

        :param grayscale: 2D grayscale image as uint8 array.
        :param cell_size: Screen cell size in pixels.
        :param pixels_per_mm: (x, y) image resolution in pixels per mm.
        :param invert: If True, invert the output (swap black/white).
        :returns: 2D binary uint8 array (values 0 or 1, 1 marks dark).
        :raises ValueError: If pixels_per_mm is not positive.
        :complexity: O(w*h)
        """
"#,
    module = "raygeo.image.dither"
)]
#[pyfunction(name = "apply_newsprint_dither")]
#[pyo3(signature = (grayscale, cell_size=1, pixels_per_mm=(1.0, 1.0), invert=false))]
fn py_apply_newsprint_dither(
    py: Python<'_>,
    grayscale: &Bound<'_, PyAny>,
    cell_size: usize,
    pixels_per_mm: (f64, f64),
    invert: bool,
) -> PyResult<Py<PyAny>> {
    checked_pixels_per_mm(pixels_per_mm)?;
    let (height, width, flat) = extract_grayscale(py, grayscale)?;

    let mut output = vec![0u8; height * width];
    dither::apply_newsprint_dither(
        &flat,
        width,
        height,
        cell_size,
        pixels_per_mm.0,
        pixels_per_mm.1,
        invert,
        &mut output,
    );

    reshape_output(py, output, height, width)
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
    sub_mod.add_function(wrap_pyfunction!(
        py_apply_error_diffusion_dither,
        sub_mod.clone()
    )?)?;
    sub_mod.add_function(wrap_pyfunction!(
        py_apply_halftone_dither,
        sub_mod.clone()
    )?)?;
    sub_mod.add_function(wrap_pyfunction!(
        py_apply_newsprint_dither,
        sub_mod.clone()
    )?)?;
    m.add_submodule(&sub_mod)?;
    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.image.dither", &sub_mod)?;
    Ok(())
}
