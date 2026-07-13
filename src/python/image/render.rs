use pyo3::prelude::*;
use pyo3::types::PyAnyMethods;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::image::render::{geometry_to_image, RenderOptions};
use crate::python::geo::geometry::Geometry as PyGeometry;

pub(crate) fn register(image_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let render_mod = PyModule::new(image_mod.py(), "render")?;
    render_mod.add_function(pyo3::wrap_pyfunction!(
        geometry_to_image_py,
        render_mod.clone()
    )?)?;
    image_mod.add_submodule(&render_mod)?;
    let sys_modules = image_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.image.render", &render_mod)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    from raygeo.geo import Geometry

    def geometry_to_image(
        strokes: geo.Geometry,
        fills: geo.Geometry,
        size_mm: tuple[float, float],
        dpi: float = 96.0,
    ) -> numpy.ndarray:
        """Rasterise vector geometry into an RGBA uint8 image.

        Converts mm‑space Y‑up geometry into a pixel buffer (Y‑down,
        origin top‑left) at the given *dpi*.

        Uses basic anti-aliasing for the wire‑frame strokes, and scan‑line
        filling for the closed polygons.

        :param strokes: Wire‑frame paths (stroked in black).
        :param fills: Closed polygons (scan‑line filled in light gray).
        :param size_mm: Physical size ``(width, height)`` in mm.
        :param dpi: Output resolution in dots per inch (default 96).
        :returns: ``numpy.ndarray`` with shape ``(H, W, 4)`` and dtype ``uint8``.
        """
    "#,
    module = "raygeo.image.render"
)]
#[pyfunction(name = "geometry_to_image")]
#[pyo3(signature = (
    strokes,
    fills,
    size_mm,
    dpi = 96.0,
))]
fn geometry_to_image_py(
    py: Python<'_>,
    strokes: &PyGeometry,
    fills: &PyGeometry,
    size_mm: (f64, f64),
    dpi: f64,
) -> PyResult<Py<PyAny>> {
    let opts = RenderOptions {
        dpi,
        ..Default::default()
    };

    let (buf, height, width) =
        geometry_to_image(&strokes.inner, &fills.inner, size_mm, &opts);

    if buf.is_empty() {
        let numpy = py.import("numpy")?;
        let arr = numpy.call_method1("zeros", ((0, 0, 4), "uint8"))?;
        return Ok(arr.into_pyobject(py)?.into_any().unbind());
    }

    use pyo3::types::PyBytes;
    let py_bytes = PyBytes::new(py, &buf);
    let numpy = py.import("numpy")?;
    let arr = numpy
        .call_method1("frombuffer", (py_bytes, numpy.getattr("uint8")?))?;
    let arr =
        arr.call_method1("reshape", (height as i64, width as i64, 4i64))?;
    Ok(arr.into_pyobject(py)?.into_any().unbind())
}
