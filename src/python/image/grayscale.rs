use numpy::IntoPyArray;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::image::grayscale;

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def compute_auto_levels(
        gray_image: numpy.typing.NDArray[numpy.uint8],
        clip_percent: float = 1.0,
    ) -> tuple[int, int]:
        """Compute auto black/white levels from a grayscale image histogram.

        :param gray_image: Grayscale image as uint8 array.
        :param clip_percent: Percentage of pixels to clip from each end.
        :returns: Tuple of (black_point, white_point).
        :complexity: O(n) where n = number of pixels
        """
"#,
    module = "raygeo.image.grayscale"
)]
#[pyfunction(name = "compute_auto_levels")]
#[pyo3(signature = (gray_image, clip_percent=1.0))]
fn py_compute_auto_levels(
    py: Python<'_>,
    gray_image: &Bound<'_, PyAny>,
    clip_percent: f32,
) -> PyResult<(u8, u8)> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (gray_image,))?;
    let flat: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;
    Ok(grayscale::compute_auto_levels(&flat, clip_percent))
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def normalize_grayscale(
        gray_image: numpy.typing.NDArray[numpy.uint8],
        black_point: int = 0,
        white_point: int = 255,
    ) -> numpy.typing.NDArray[numpy.uint8]:
        """Normalize a grayscale image by stretching the dynamic range.

        :param gray_image: Input grayscale image as uint8 array.
        :param black_point: Black point for normalization.
        :param white_point: White point for normalization.
        :returns: Normalized grayscale image with the same shape.
        :raises ValueError: If black_point >= white_point.
        :complexity: O(n) where n = number of pixels
        """
"#,
    module = "raygeo.image.grayscale"
)]
#[pyfunction(name = "normalize_grayscale")]
#[pyo3(signature = (gray_image, black_point=0, white_point=255))]
fn py_normalize_grayscale(
    py: Python<'_>,
    gray_image: &Bound<'_, PyAny>,
    black_point: u8,
    white_point: u8,
) -> PyResult<Py<PyAny>> {
    if black_point >= white_point {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "black_point must be less than white_point",
        ));
    }
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (gray_image,))?;
    let shape = arr.getattr("shape")?.extract::<Vec<usize>>()?;
    let flat: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    let mut output = vec![0u8; flat.len()];
    grayscale::normalize_grayscale(
        &flat,
        black_point,
        white_point,
        &mut output,
    );

    let result = output.into_pyarray(py);
    let reshaped = result.call_method1("reshape", (shape,))?;
    Ok(reshaped.unbind())
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let sub_mod = PyModule::new(m.py(), "grayscale")?;
    sub_mod.add_function(wrap_pyfunction!(
        py_compute_auto_levels,
        sub_mod.clone()
    )?)?;
    sub_mod.add_function(wrap_pyfunction!(
        py_normalize_grayscale,
        sub_mod.clone()
    )?)?;
    m.add_submodule(&sub_mod)?;
    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.image.grayscale", &sub_mod)?;
    Ok(())
}
