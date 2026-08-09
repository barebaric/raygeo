use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::python::compressed_array::PyCompressedArray;
use crate::python::ops::container::PyOps;

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def rasterize_scanlines(
        ops: raygeo.ops.Ops,
        width_px: int,
        height_px: int,
        px_per_mm: tuple[float, float],
        origin_mm: tuple[float, float] = (0.0, 0.0),
        radius_px: int = 0,
    ) -> raygeo.compressed_array.CompressedArray:
        """Rasterize ScanLine commands from *ops* into a 2D power-map buffer.

        Iterates all scanline commands in *ops*, converts their mm coordinates
        to pixel space using *px_per_mm*, and returns a uint8 array where each
        pixel holds the maximum power value written to it.

        When *radius_px* is greater than zero, each rasterized sample is
        expanded to a square brush of side ``2*radius_px + 1`` (max-merged),
        equivalent to a square morphological dilation of the thin raster.
        Coverage is bounds-clamped at the texture edges (no wraparound).

        :param ops: Command sequence to rasterize.
        :param width_px: Width of the output texture in pixels.
        :param height_px: Height of the output texture in pixels.
        :param px_per_mm: (x, y) resolution in pixels per millimeter.
        :param origin_mm: (x, y) origin offset in mm (default ``(0.0, 0.0)``).
        :param radius_px: Half-size of the dilation brush in pixels
            (default ``0`` -- no dilation).
        :returns: CompressedArray of shape (height_px, width_px), uint8.
        :complexity: O(scanline_pixels * (2*radius_px + 1))
        """
    "#,
    module = "raygeo.image"
)]
#[pyfunction(name = "rasterize_scanlines")]
#[pyo3(signature = (ops, width_px, height_px, px_per_mm, origin_mm=(0.0, 0.0), radius_px=0))]
fn py_rasterize_scanlines(
    py: Python<'_>,
    ops: &PyOps,
    width_px: u32,
    height_px: u32,
    px_per_mm: (f64, f64),
    origin_mm: (f64, f64),
    radius_px: u32,
) -> PyResult<Py<PyAny>> {
    let buffer = ops.inner.to_texture(
        width_px,
        height_px,
        px_per_mm,
        origin_mm,
        radius_px as i32,
    );

    if buffer.is_empty() || !buffer.iter().any(|&b| b != 0) {
        let numpy = py.import("numpy")?;
        let empty = numpy.call_method1("array", (vec![0u8; 0],))?;
        return Ok(empty.unbind());
    }

    let shape = vec![height_px as usize, width_px as usize];
    let compressed = PyCompressedArray::from_vec_u8(buffer, shape);
    Ok(Py::new(py, compressed)?.into_any())
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_rasterize_scanlines, m.clone())?)?;
    Ok(())
}
