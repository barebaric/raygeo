use pyo3::prelude::*;
use pyo3_stub_gen::derive::*;

use crate::image::composite::{
    composite_views_into as rs_composite_views_into, ViewInput,
};

// ===================================================================
// composite_views_into
// ===================================================================

/// Composite multiple ARGB32 bitmaps into a target buffer with
/// per-view positioning and scaling.
///
/// Each source is placed at (dst_x, dst_y) in target pixel coordinates,
/// scaled by (scale_x, scale_y).  Nearest-neighbour sampling is used.
/// Alpha blending follows the pre-multiplied ``over`` operator.
///
/// :param target: ``(H, W, 4)`` ``numpy.uint8`` target buffer
///     (zero-initialised ARGB32 premultiplied).
/// :param views: List of ``(source, dst_x, dst_y, scale_x, scale_y)``
///     tuples where *source* is ``(H, W, 4)`` ``numpy.uint8``.
/// :returns: ``None``
#[gen_stub_pyfunction(module = "raygeo.image.composite")]
#[pyfunction(name = "composite_views_into")]
fn composite_views_into_py(
    py: Python<'_>,
    target: &Bound<'_, PyAny>,
    views: &Bound<'_, PyAny>,
) -> PyResult<()> {
    use numpy::PyArrayMethods;

    let target_arr = target.cast::<numpy::PyArray3<u8>>().map_err(|_| {
        pyo3::exceptions::PyTypeError::new_err(
            "target must be a numpy.uint8 3-D array",
        )
    })?;

    let t_dims = target_arr.dims();
    let t_h = t_dims[0];
    let t_w = t_dims[1];
    if t_dims[2] != 4 || t_h == 0 || t_w == 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "target must have shape (H, W, 4) with H, W > 0",
        ));
    }

    // First pass: extract all view parameters and copy source data.
    struct ViewParams {
        data: Vec<u8>,
        src_w: u32,
        src_h: u32,
        dst_x: f64,
        dst_y: f64,
        scale_x: f64,
        scale_y: f64,
    }

    let mut params: Vec<ViewParams> = Vec::new();
    let views_iter = views.try_iter()?;

    for item in views_iter {
        let item = item?;
        let tuple: &Bound<'_, pyo3::types::PyTuple> = item.cast()?;
        if tuple.len() != 5 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "each view must be a tuple (source, dst_x, dst_y, scale_x, scale_y)",
            ));
        }

        let src = tuple.get_item(0)?;
        let src_arr = src.cast::<numpy::PyArray3<u8>>().map_err(|_| {
            pyo3::exceptions::PyTypeError::new_err(
                "source must be a numpy.uint8 3-D array",
            )
        })?;

        let s_dims = src_arr.dims();
        let s_h = s_dims[0];
        let s_w = s_dims[1];
        if s_dims[2] != 4 || s_h == 0 || s_w == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "source must have shape (H, W, 4) with H, W > 0",
            ));
        }

        let dst_x: f64 = tuple.get_item(1)?.extract()?;
        let dst_y: f64 = tuple.get_item(2)?.extract()?;
        let scale_x: f64 = tuple.get_item(3)?.extract()?;
        let scale_y: f64 = tuple.get_item(4)?.extract()?;

        params.push(ViewParams {
            data: unsafe {
                std::slice::from_raw_parts(
                    src_arr.data() as *const u8,
                    s_h * s_w * 4,
                )
            }
            .to_vec(),
            src_w: s_w as u32,
            src_h: s_h as u32,
            dst_x,
            dst_y,
            scale_x,
            scale_y,
        });
    }

    // Build ViewInputs from the collected parameters.
    let mut rust_views: Vec<ViewInput<'_>> = Vec::with_capacity(params.len());
    for p in &params {
        rust_views.push(ViewInput {
            bitmap: &p.data,
            src_w: p.src_w,
            src_h: p.src_h,
            dst_x: p.dst_x,
            dst_y: p.dst_y,
            scale_x: p.scale_x,
            scale_y: p.scale_y,
        });
    }

    let buf_addr = target_arr.data() as usize;
    let buf_len = t_h * t_w * 4;

    py.detach(move || {
        let slice = unsafe {
            std::slice::from_raw_parts_mut(buf_addr as *mut u8, buf_len)
        };
        rs_composite_views_into(slice, t_w as u32, t_h as u32, &rust_views);
    });

    Ok(())
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let sub_mod = PyModule::new(m.py(), "composite")?;
    sub_mod.add_function(pyo3::wrap_pyfunction!(
        composite_views_into_py,
        sub_mod.clone()
    )?)?;
    m.add_submodule(&sub_mod)?;
    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.image.composite", &sub_mod)?;
    Ok(())
}
