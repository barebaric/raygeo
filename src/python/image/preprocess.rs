use numpy::IntoPyArray;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::image::preprocess;

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def grayscale_to_binary(
        gray: numpy.typing.NDArray[numpy.uint8],
        threshold: float = 0.5,
        invert: bool = False,
        auto_threshold: bool = True,
    ) -> numpy.typing.NDArray[numpy.uint8]:
        """Convert grayscale image to binary using Otsu or fixed threshold.

        Pixels at or below the threshold become foreground (1).
        Uses Otsu's method when auto_threshold is True.

        :param gray: 2D grayscale uint8 image.
        :param threshold: Fixed threshold (0.0-1.0), used only if auto_threshold is False.
        :param invert: If True, pixels above threshold become foreground.
        :param auto_threshold: If True, compute threshold via Otsu's method.
        :returns: 2D binary uint8 array (values 0 or 1).
        :complexity: O(w*h)
        """
"#,
    module = "raygeo.image.preprocess"
)]
#[pyfunction(name = "grayscale_to_binary")]
#[pyo3(signature = (gray, threshold=0.5, invert=false, auto_threshold=true))]
fn py_grayscale_to_binary(
    py: Python<'_>,
    gray: &Bound<'_, PyAny>,
    threshold: f64,
    invert: bool,
    auto_threshold: bool,
) -> PyResult<Py<PyAny>> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (gray,))?;
    let shape = arr.getattr("shape")?.extract::<(usize, usize)>()?;
    let height = shape.0;
    let width = shape.1;
    let flat: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    let thr_u8 = (threshold.clamp(0.0, 1.0) * 255.0) as u8;

    let output = preprocess::grayscale_to_binary(
        &flat,
        width,
        height,
        thr_u8,
        invert,
        auto_threshold,
    );

    let result = output.into_pyarray(py);
    let reshaped = result.call_method1("reshape", (height, width))?;
    Ok(reshaped.unbind())
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def get_component_areas(
        binary: numpy.typing.NDArray[numpy.uint8],
    ) -> list[int]:
        """Compute the pixel area of each connected component.

        Uses 8-connectivity. Areas are returned sorted ascending.
        Background (0-valued pixels) is excluded.

        :param binary: 2D binary uint8 array (values 0 or 1).
        :returns: Sorted list of component pixel areas.
        :complexity: O(w*h)
        """
"#,
    module = "raygeo.image.preprocess"
)]
#[pyfunction(name = "get_component_areas")]
fn py_get_component_areas(
    py: Python<'_>,
    binary: &Bound<'_, PyAny>,
) -> PyResult<Vec<u32>> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (binary,))?;
    let shape = arr.getattr("shape")?.extract::<(usize, usize)>()?;
    let height = shape.0;
    let width = shape.1;
    let flat: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    Ok(preprocess::get_component_areas(&flat, width, height))
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def filter_components(
        binary: numpy.typing.NDArray[numpy.uint8],
        min_area: int,
    ) -> numpy.typing.NDArray[numpy.uint8]:
        """Remove connected components smaller than min_area.

        Uses 8-connectivity for component detection.

        :param binary: 2D binary uint8 array (values 0 or 1).
        :param min_area: Minimum pixel count to keep a component.
        :returns: 2D binary uint8 array (values 0 or 1).
        :complexity: O(w*h)
        """
"#,
    module = "raygeo.image.preprocess"
)]
#[pyfunction(name = "filter_components")]
fn py_filter_components(
    py: Python<'_>,
    binary: &Bound<'_, PyAny>,
    min_area: usize,
) -> PyResult<Py<PyAny>> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (binary,))?;
    let shape = arr.getattr("shape")?.extract::<(usize, usize)>()?;
    let height = shape.0;
    let width = shape.1;
    let flat: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    let output = preprocess::filter_components(&flat, width, height, min_area);

    let result = output.into_pyarray(py);
    let reshaped = result.call_method1("reshape", (height, width))?;
    Ok(reshaped.unbind())
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def denoise_binary(
        binary: numpy.typing.NDArray[numpy.uint8],
    ) -> numpy.typing.NDArray[numpy.uint8]:
        """Remove small noise components from a binary image using adaptive thresholding.

        Computes connected components, finds the largest gap in component area
        distribution to separate noise from content, and removes small components.
        Uses the same algorithm as the legacy Python ``_find_adaptive_area_threshold``.

        :param binary: 2D binary uint8 array (values 0 or 1).
        :returns: 2D binary uint8 array with noise removed.
        :complexity: O(w*h)
        """
"#,
    module = "raygeo.image.preprocess"
)]
#[pyfunction(name = "denoise_binary")]
fn py_denoise_binary(
    py: Python<'_>,
    binary: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (binary,))?;
    let shape = arr.getattr("shape")?.extract::<(usize, usize)>()?;
    let height = shape.0;
    let width = shape.1;
    let flat: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    let output = preprocess::denoise_binary(&flat, width, height);

    let result = output.into_pyarray(py);
    let reshaped = result.call_method1("reshape", (height, width))?;
    Ok(reshaped.unbind())
}

#[gen_stub_pyfunction(
    python = r#"
    def compute_adaptive_threshold(
        areas: list[int],
    ) -> int:
        """Compute an adaptive area threshold to separate noise from content.

        Analyses the distribution of connected component areas and finds the
        largest gap to determine a threshold that separates noise (small
        components) from meaningful content.

        :param areas: Sorted list of component pixel areas.
        :returns: Adaptive threshold value (minimum area to keep).
        :complexity: O(n) where n = number of unique area values
        """
"#,
    module = "raygeo.image.preprocess"
)]
#[pyfunction(name = "compute_adaptive_threshold")]
fn py_compute_adaptive_threshold(areas: Vec<u32>) -> usize {
    preprocess::compute_adaptive_threshold(&areas)
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let sub_mod = PyModule::new(m.py(), "preprocess")?;
    sub_mod.add_function(wrap_pyfunction!(
        py_grayscale_to_binary,
        sub_mod.clone()
    )?)?;
    sub_mod.add_function(wrap_pyfunction!(
        py_get_component_areas,
        sub_mod.clone()
    )?)?;
    sub_mod.add_function(wrap_pyfunction!(
        py_filter_components,
        sub_mod.clone()
    )?)?;
    sub_mod
        .add_function(wrap_pyfunction!(py_denoise_binary, sub_mod.clone())?)?;
    sub_mod.add_function(wrap_pyfunction!(
        py_compute_adaptive_threshold,
        sub_mod.clone()
    )?)?;
    m.add_submodule(&sub_mod)?;
    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.image.preprocess", &sub_mod)?;
    Ok(())
}
