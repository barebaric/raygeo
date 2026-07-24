pyo3_stub_gen::module_doc!("raygeo.ops.convert.view", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
View rendering: rasterise Ops into pre-multiplied ARGB32 bitmaps on
the rayon pool.

Provides both a free-function API (render_ops / render_ops_batch) and
an Encoder spec class (raygeo.ops.convert.ViewSpec) that plugs into
the generic encoder pipeline.
";

use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction};
use rayon::prelude::*;

use crate::ops::convert::view::{
    render_ops as rs_render_ops, render_ops_into as rs_render_ops_into,
    ViewSpec,
};
use crate::python::ops::container::PyOps;

use super::PyViewSpec;

// ===================================================================
// RenderResult
// ===================================================================

/// Result of :func:`render_ops`.
///
/// Exposes the pre-multiplied ARGB32 bitmap as a `(H, W, 4)` ``numpy.uint8``
/// array (byte order B, G, R, A on little‑endian, matching Cairo's
/// ``FORMAT_ARGB32``) plus the geometry bbox and effective pixels-per-mm
/// actually applied by the renderer.
#[gen_stub_pyclass(module = "raygeo.ops.convert.view")]
#[pyclass(
    name = "RenderResult",
    module = "raygeo.ops.convert.view",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRenderResult {
    /// ``(H, W, 4)`` ARGB32 bitmap (BGRA on little-endian).
    #[pyo3(get)]
    pub bitmap: Py<PyAny>,
    /// Geometry bbox in mm as ``(min_x, min_y, max_x, max_y)``.
    #[pyo3(get)]
    pub bbox_mm: (f64, f64, f64, f64),
    /// Effective pixels-per-mm applied by the renderer after
    /// clamping to ``max_dimension_px`` / ``max_total_pixels``.
    #[pyo3(get)]
    pub effective_ppm: (f64, f64),
}

impl PyRenderResult {
    fn from_core(
        core: crate::ops::convert::view::RenderResult,
        py: Python<'_>,
    ) -> Self {
        use numpy::IntoPyArray;
        let total = core.buffer.len();
        debug_assert_eq!(total, core.height * core.width * 4);
        let arr = core.buffer.into_pyarray(py);
        let shape = (core.height as i64, core.width as i64, 4i64);
        let reshaped = arr
            .call_method1("reshape", shape)
            .expect("reshape buffer to (H, W, 4)");
        PyRenderResult {
            bitmap: reshaped.unbind().into_any(),
            bbox_mm: core.bbox_mm,
            effective_ppm: core.effective_ppm,
        }
    }
}

// ===================================================================
// render_ops
// ===================================================================

/// Rasterise a single :class:`raygeo.ops.Ops` to an ARGB32 bitmap.
///
/// :param ops: The :class:`~raygeo.ops.Ops` object to render.
/// :param spec: A :class:`~raygeo.ops.convert.ViewSpec` with the
///     rendering parameters (resolution, margins, colours, LUT).
/// :returns: A :class:`RenderResult`, or ``None`` if there is no
///     geometry to draw (empty Ops, all travel moves with
///     ``show_travel_moves = False``).
#[gen_stub_pyfunction(module = "raygeo.ops.convert.view")]
#[pyfunction(name = "render_ops")]
fn render_ops_py(
    py: Python<'_>,
    ops: &PyOps,
    spec: &PyViewSpec,
) -> PyResult<Option<PyRenderResult>> {
    let cut_lut = lut_to_array(spec.cut_lut.clone())?;
    let engrave_lut = lut_to_array(spec.engrave_lut.clone())?;
    let core_spec = ViewSpec {
        pixels_per_mm: spec.pixels_per_mm,
        show_travel_moves: spec.show_travel_moves,
        render_bbox: spec.render_bbox,
        max_dimension_px: spec.max_dimension_px,
        max_total_pixels: spec.max_total_pixels,
        cut_color: spec.cut_color,
        travel_color: spec.travel_color,
        zero_power_color: spec.zero_power_color,
        cut_lut,
        engrave_lut,
    };
    let ops_clone = ops.inner.clone();
    let core = py.detach(move || rs_render_ops(&ops_clone, &core_spec));
    Ok(core.map(|r| PyRenderResult::from_core(r, py)))
}

// ===================================================================
// render_ops_batch
// ===================================================================

/// Rasterise a batch of :class:`Ops` objects in parallel on the rayon
/// pool.
///
/// :param items: List of ``(ops, spec)`` tuples where *spec* is a
///     :class:`~raygeo.ops.convert.ViewSpec`.  Each item is rendered
///     independently.  See :func:`render_ops` for details.
/// :returns: A list parallel to *items*; each slot is either a
///     :class:`RenderResult` or ``None`` if that workpiece had no
///     geometry to draw.
#[gen_stub_pyfunction(module = "raygeo.ops.convert.view")]
#[pyfunction(name = "render_ops_batch")]
fn render_ops_batch_py(
    py: Python<'_>,
    items: &Bound<'_, PyList>,
) -> PyResult<Vec<Option<PyRenderResult>>> {
    use pyo3::types::PyTupleMethods;

    let mut prepared: Vec<(crate::ops::Ops, ViewSpec)> =
        Vec::with_capacity(items.len());

    for it in items.iter() {
        let tuple = it.cast::<pyo3::types::PyTuple>()?;
        let ops_py: Py<PyOps> = tuple.get_item(0)?.extract()?;
        let ops_clone = ops_py.bind(py).borrow().inner.clone();
        let spec_py: Py<PyViewSpec> = tuple.get_item(1)?.extract()?;
        let spec = spec_py.borrow(py);
        let cut_lut = lut_to_array(spec.cut_lut.clone())?;
        let engrave_lut = lut_to_array(spec.engrave_lut.clone())?;
        let core_spec = ViewSpec {
            pixels_per_mm: spec.pixels_per_mm,
            show_travel_moves: spec.show_travel_moves,
            render_bbox: spec.render_bbox,
            max_dimension_px: spec.max_dimension_px,
            max_total_pixels: spec.max_total_pixels,
            cut_color: spec.cut_color,
            travel_color: spec.travel_color,
            zero_power_color: spec.zero_power_color,
            cut_lut,
            engrave_lut,
        };
        prepared.push((ops_clone, core_spec));
    }

    let results: Vec<_> = py.detach(move || {
        prepared
            .into_par_iter()
            .map(|(ops, spec)| rs_render_ops(&ops, &spec))
            .collect()
    });

    Ok(results
        .into_iter()
        .map(|r| r.map(|c| PyRenderResult::from_core(c, py)))
        .collect())
}

// ===================================================================
// render_ops_into
// ===================================================================

/// Render an :class:`Ops` chunk directly into a caller-provided bitmap.
///
/// Unlike :func:`render_ops`, this does not allocate a buffer — it
/// writes into *bitmap*, a ``(H, W, 4)`` ``numpy.uint8`` array.  The
/// *view_bbox* ``(min_x, min_y, max_x, max_y)`` defines the mm-space
/// area the bitmap covers; the effective ppm is derived from the
/// bitmap dimensions and the bbox.
///
/// Texture (scanline) data is rendered first, then vertex strokes on
/// top.
///
/// :param ops: The :class:`~raygeo.ops.Ops` chunk to render.
/// :param spec: A :class:`~raygeo.ops.convert.ViewSpec` (only the
///     colour/LUT/show_travel fields are used; ``pixels_per_mm`` and
///     ``render_bbox`` are ignored — ppm is derived from the bitmap
///     and *view_bbox*).
/// :param bitmap: A ``(H, W, 4)`` ``numpy.uint8`` array to render
///     into.
/// :param view_bbox: The mm-space bbox ``(min_x, min_y, max_x,
///     max_y)`` the bitmap covers.
/// :returns: ``True`` if any content was drawn, ``False`` if the
///     bbox is degenerate.
#[gen_stub_pyfunction(module = "raygeo.ops.convert.view")]
#[pyfunction(name = "render_ops_into")]
fn render_ops_into_py(
    py: Python<'_>,
    ops: &PyOps,
    spec: &PyViewSpec,
    bitmap: &Bound<'_, PyAny>,
    view_bbox: (f64, f64, f64, f64),
) -> PyResult<bool> {
    use numpy::PyArrayMethods;

    let arr = bitmap.cast::<numpy::PyArray3<u8>>().map_err(|_| {
        pyo3::exceptions::PyTypeError::new_err(
            "bitmap must be a numpy.uint8 3-D array",
        )
    })?;

    let dims = arr.dims();
    let h_px = dims[0];
    let w_px = dims[1];
    if dims[2] != 4 || h_px == 0 || w_px == 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "bitmap must have shape (H, W, 4) with H, W > 0",
        ));
    }

    let cut_lut = lut_to_array(spec.cut_lut.clone())?;
    let engrave_lut = lut_to_array(spec.engrave_lut.clone())?;
    let core_spec = ViewSpec {
        pixels_per_mm: spec.pixels_per_mm,
        show_travel_moves: spec.show_travel_moves,
        render_bbox: spec.render_bbox,
        max_dimension_px: spec.max_dimension_px,
        max_total_pixels: spec.max_total_pixels,
        cut_color: spec.cut_color,
        travel_color: spec.travel_color,
        zero_power_color: spec.zero_power_color,
        cut_lut,
        engrave_lut,
    };

    let ops_clone = ops.inner.clone();
    let buf_addr = arr.data() as usize;
    let buf_len = h_px * w_px * 4;
    let result: bool = py.detach(move || {
        let slice = unsafe {
            std::slice::from_raw_parts_mut(buf_addr as *mut u8, buf_len)
        };
        rs_render_ops_into(&ops_clone, &core_spec, slice, w_px, h_px, view_bbox)
    });
    Ok(result)
}

// ===================================================================
// Helpers
// ===================================================================

pub(crate) fn lut_to_array(lut: Vec<[u8; 4]>) -> PyResult<[[u8; 4]; 256]> {
    if lut.len() != 256 {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "cut_lut must have exactly 256 entries; got {}",
            lut.len(),
        )));
    }
    let mut arr = [[0u8; 4]; 256];
    arr.copy_from_slice(&lut);
    Ok(arr)
}

// ===================================================================
// Module registration
// ===================================================================

pub(crate) fn register(convert_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = convert_mod.py();
    let view_mod = PyModule::new(py, "view")?;
    view_mod.setattr("__doc__", MODULE_DOC)?;

    view_mod.add_class::<PyRenderResult>()?;
    view_mod.add_function(pyo3::wrap_pyfunction!(
        render_ops_py,
        view_mod.clone()
    )?)?;
    view_mod.add_function(pyo3::wrap_pyfunction!(
        render_ops_batch_py,
        view_mod.clone()
    )?)?;
    view_mod.add_function(pyo3::wrap_pyfunction!(
        render_ops_into_py,
        view_mod.clone()
    )?)?;

    convert_mod.add_submodule(&view_mod)?;
    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.convert.view", &view_mod)?;
    Ok(())
}
