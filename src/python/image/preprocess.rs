use numpy::IntoPyArray;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::image::preprocess as rust_preprocess;

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
    module = "raygeo.image"
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

    let output = rust_preprocess::grayscale_to_binary(
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
    module = "raygeo.image"
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

    Ok(rust_preprocess::get_component_areas(&flat, width, height))
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
    module = "raygeo.image"
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

    let output =
        rust_preprocess::filter_components(&flat, width, height, min_area);

    let result = output.into_pyarray(py);
    let reshaped = result.call_method1("reshape", (height, width))?;
    Ok(reshaped.unbind())
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_grayscale_to_binary, m.clone())?)?;
    m.add_function(wrap_pyfunction!(py_get_component_areas, m.clone())?)?;
    m.add_function(wrap_pyfunction!(py_filter_components, m.clone())?)?;
    Ok(())
}
