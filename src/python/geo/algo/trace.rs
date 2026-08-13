pyo3_stub_gen::module_doc!("raygeo.geo.algo.trace", "{}", MODULE_DOC_TRACE);

pub(crate) const MODULE_DOC_TRACE: &str = "\
Contour extraction from binary images.

Provides boundary tracing of foreground regions in a boolean image,
returning ordered point loops around each component in pixel
coordinates (y increases downward).";

use super::super::flex_point::points_to_tuples;
use numpy::PyArrayMethods;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "trace")?;
    m.setattr("__doc__", MODULE_DOC_TRACE)?;

    register_functions!(m, find_external_contours_py);

    algo_mod.add_submodule(&m)?;
    let sys_modules = algo_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.algo.trace", &m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy

    def find_external_contours(
        boolean_image: numpy.ndarray,
    ) -> list[list[tuple[float, float]]]:
        """Trace the outer boundary of each foreground region.

        Pixels with value 0 are treated as background; non-zero values
        are foreground. Each contour is an ordered loop of (x, y) points
        in pixel coordinates; contours with fewer than 3 points are
        dropped.

        :param boolean_image: 2D boolean array.
        :returns: List of contours, each a list of (x, y) points.
        :complexity: O(w*h) time, O(w*h) space where w*h is the image size
        """
"#,
    module = "raygeo.geo.algo.trace"
)]
#[pyfunction(name = "find_external_contours")]
fn find_external_contours_py(
    py: Python<'_>,
    boolean_image: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<(f64, f64)>>> {
    let (flat, h, w) = extract_bool_image(py, boolean_image)?;
    let contours = crate::geo::algo::trace::find_external_contours(&flat, w, h);
    Ok(contours.into_iter().map(points_to_tuples).collect())
}

fn extract_bool_image(
    py: Python<'_>,
    obj: &Bound<'_, PyAny>,
) -> PyResult<(Vec<u8>, usize, usize)> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (obj,))?;
    let shape: (usize, usize) = arr.getattr("shape")?.extract()?;
    // Convert to a C-contiguous uint8 buffer and read it directly
    // instead of materializing a Python list via tolist().
    let contiguous = numpy.call_method1("ascontiguousarray", (arr, "uint8"))?;
    let array: Bound<'_, numpy::PyArray2<u8>> = contiguous.extract()?;
    let view = array.readonly();
    let data = view.as_array();
    let flat: Vec<u8> =
        data.iter().map(|&v| if v != 0 { 1 } else { 0 }).collect();
    Ok((flat, shape.0, shape.1))
}
