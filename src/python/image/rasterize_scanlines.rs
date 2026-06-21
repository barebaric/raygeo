use numpy::IntoPyArray;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::image::rasterize;
use crate::python::ops::container::PyOps;

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing
    from raygeo.ops import Ops

    def rasterize_scanlines(
        ops: ops.Ops,
        width_px: int,
        height_px: int,
        px_per_mm: tuple[float, float],
        origin_mm: tuple[float, float] = (0.0, 0.0),
    ) -> numpy.typing.NDArray[numpy.uint8]:
        """Rasterize ScanLine commands from *ops* into a 2D power-map buffer.

        Iterates all scanline commands in *ops*, converts their mm coordinates
        to pixel space using *px_per_mm*, and returns a uint8 array where each
        pixel holds the maximum power value written to it.

        :param ops: Command sequence to rasterize.
        :param width_px: Width of the output texture in pixels.
        :param height_px: Height of the output texture in pixels.
        :param px_per_mm: (x, y) resolution in pixels per millimeter.
        :param origin_mm: (x, y) origin offset in mm (default ``(0.0, 0.0)``).
        :returns: 2D uint8 array of shape (height_px, width_px).
        :complexity: O(scanline_pixels)
        """
    "#,
    module = "raygeo.image"
)]
#[pyfunction(name = "rasterize_scanlines")]
#[pyo3(signature = (ops, width_px, height_px, px_per_mm, origin_mm=(0.0, 0.0)))]
fn py_rasterize_scanlines(
    py: Python<'_>,
    ops: &PyOps,
    width_px: u32,
    height_px: u32,
    px_per_mm: (f64, f64),
    origin_mm: (f64, f64),
) -> PyResult<Py<PyAny>> {
    let buffer = rasterize::rasterize_scanlines(
        &ops.inner, width_px, height_px, px_per_mm, origin_mm,
    );

    let numpy = py.import("numpy")?;
    if buffer.is_empty() {
        let empty = numpy.call_method1("array", (vec![0u8; 0],))?;
        return Ok(empty.unbind());
    }

    let result = buffer.into_pyarray(py);
    let reshaped =
        result.call_method1("reshape", (height_px as i32, width_px as i32))?;
    Ok(reshaped.unbind())
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_rasterize_scanlines, m.clone())?)?;
    Ok(())
}
