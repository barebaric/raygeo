use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::ops::raster::rasterize::{
    rasterize_mask_lines, rasterize_mask_scan, rasterize_multi_pass,
    rasterize_power_modulation,
};
use crate::ops::raster::scan::{
    self, downsample_power_values, find_mask_bounding_box,
    generate_horizontal_scan_positions, generate_scan_lines, line_pixels,
    resample_rows,
};
use crate::python::ops::container::PyOps;

#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops.raster", name = "ScanLine", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyScanLine {
    pub index: i64,
    pub start_mm: (f64, f64),
    pub end_mm: (f64, f64),
    pub pixels: Vec<(i32, i32)>,
    pub line_interval_mm: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyScanLine {
    #[new]
    fn new(
        index: i64,
        start_mm: (f64, f64),
        end_mm: (f64, f64),
        pixels: Vec<(i32, i32)>,
        line_interval_mm: f64,
    ) -> Self {
        PyScanLine {
            index,
            start_mm,
            end_mm,
            pixels,
            line_interval_mm,
        }
    }

    #[getter]
    fn index(&self) -> i64 {
        self.index
    }

    #[getter]
    fn start_mm(&self) -> (f64, f64) {
        self.start_mm
    }

    #[getter]
    fn end_mm(&self) -> (f64, f64) {
        self.end_mm
    }

    #[getter]
    fn pixels(&self) -> Vec<(i32, i32)> {
        self.pixels.clone()
    }

    #[getter]
    fn line_interval_mm(&self) -> f64 {
        self.line_interval_mm
    }

    fn length_mm(&self) -> f64 {
        let dx = self.end_mm.0 - self.start_mm.0;
        let dy = self.end_mm.1 - self.start_mm.1;
        (dx * dx + dy * dy).sqrt()
    }

    fn direction(&self) -> (f64, f64) {
        let length = self.length_mm();
        if length < 1e-9 {
            return (1.0, 0.0);
        }
        (
            (self.end_mm.0 - self.start_mm.0) / length,
            (self.end_mm.1 - self.start_mm.1) / length,
        )
    }

    fn pixel_to_mm(
        &self,
        px: i32,
        py: i32,
        pixels_per_mm: (f64, f64),
    ) -> (f64, f64) {
        let sl = scan::ScanLine {
            index: self.index,
            start_mm: self.start_mm,
            end_mm: self.end_mm,
            pixels: self.pixels.clone(),
            line_interval_mm: self.line_interval_mm,
        };
        sl.pixel_to_mm(px, py, pixels_per_mm)
    }
}

#[pyfunction(name = "find_mask_bounding_box")]
fn py_find_mask_bounding_box(
    py: Python<'_>,
    mask: &Bound<'_, PyAny>,
) -> PyResult<Option<(i32, i32, i32, i32)>> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (mask,))?;
    let shape: (usize, usize) = arr.getattr("shape")?.extract()?;
    let height = shape.0;
    let width = shape.1;

    let flat_list: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    Ok(find_mask_bounding_box(&flat_list, height, width))
}

#[pyfunction(name = "find_segments")]
fn py_find_segments(
    values: &Bound<'_, PyAny>,
) -> PyResult<Vec<(usize, usize)>> {
    let list: Vec<u8> = values.call_method0("tolist")?.extract()?;
    Ok(scan::find_segments(&list))
}

#[pyfunction(name = "line_pixels")]
fn py_line_pixels(
    start: (f64, f64),
    end: (f64, f64),
    width: i32,
    height: i32,
) -> Vec<(i32, i32)> {
    line_pixels(start, end, width, height)
}

#[pyfunction(name = "generate_scan_lines")]
#[pyo3(signature = (bbox, image_size, pixels_per_mm, line_interval_mm, direction_degrees=0.0, offset_x_mm=0.0, offset_y_mm=0.0, global_center_mm=None))]
#[allow(clippy::too_many_arguments)]
fn py_generate_scan_lines(
    bbox: (i32, i32, i32, i32),
    image_size: (i32, i32),
    pixels_per_mm: (f64, f64),
    line_interval_mm: f64,
    direction_degrees: f64,
    offset_x_mm: f64,
    offset_y_mm: f64,
    global_center_mm: Option<(f64, f64)>,
) -> Vec<PyScanLine> {
    let lines = generate_scan_lines(
        bbox,
        image_size,
        pixels_per_mm,
        line_interval_mm,
        direction_degrees,
        offset_x_mm,
        offset_y_mm,
        global_center_mm,
    );
    lines
        .into_iter()
        .map(|sl| PyScanLine {
            index: sl.index,
            start_mm: sl.start_mm,
            end_mm: sl.end_mm,
            pixels: sl.pixels,
            line_interval_mm: sl.line_interval_mm,
        })
        .collect()
}

#[pyfunction(name = "generate_horizontal_scan_positions")]
fn py_generate_horizontal_scan_positions(
    y_min_px: i32,
    y_max_px: i32,
    height_px: i32,
    pixels_per_mm: (f64, f64),
    line_interval_mm: f64,
    offset_y_mm: f64,
) -> (Vec<f64>, Vec<f64>) {
    generate_horizontal_scan_positions(
        y_min_px,
        y_max_px,
        height_px,
        pixels_per_mm,
        line_interval_mm,
        offset_y_mm,
    )
}

#[pyfunction(name = "resample_rows")]
fn py_resample_rows(
    py: Python<'_>,
    image: &Bound<'_, PyAny>,
    y_coords_px: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let numpy = py.import("numpy")?;

    let arr = numpy.call_method1("asarray", (image,))?;
    let shape: (usize, usize) = arr.getattr("shape")?.extract()?;
    let height = shape.0;
    let width = shape.1;

    let flat_list: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    let y_coords: Vec<f64> = y_coords_px.call_method0("tolist")?.extract()?;

    let result = resample_rows(&flat_list, height, width, &y_coords);

    let np_arr = numpy.call_method1("array", (result,))?;
    let reshaped = np_arr.call_method1("reshape", (y_coords.len(), width))?;
    Ok(reshaped.unbind())
}

fn extract_flat_u8(
    py: Python<'_>,
    arr: &Bound<'_, PyAny>,
) -> PyResult<(Vec<u8>, usize, usize)> {
    let numpy = py.import("numpy")?;
    let a = numpy.call_method1("asarray", (arr,))?;
    let shape: (usize, usize) = a.getattr("shape")?.extract()?;
    let flat: Vec<u8> = a
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;
    Ok((flat, shape.0, shape.1))
}

#[pyfunction(name = "downsample_power_values")]
fn py_downsample_power_values(
    power_values: &Bound<'_, PyAny>,
    start_mm: (f64, f64),
    end_mm: (f64, f64),
    sample_interval_mm: f64,
) -> PyResult<(Vec<u8>, Vec<f64>, Vec<f64>)> {
    let pv: Vec<u8> = power_values.call_method0("tolist")?.extract()?;
    let ds = downsample_power_values(&pv, start_mm, end_mm, sample_interval_mm);
    Ok((ds.power, ds.x_mm, ds.y_mm))
}

#[gen_stub_pyfunction(python = r#"
    import numpy
    import numpy.typing
    from raygeo.ops import Ops

    def rasterize_power_modulation(
        gray_image: numpy.typing.NDArray[numpy.uint8],
        alpha: numpy.typing.NDArray[numpy.uint8],
        pixels_per_mm: tuple[float, float],
        offset_x_mm: float,
        offset_y_mm: float,
        line_interval_mm: float,
        sample_interval_mm: float,
        min_power: float = 0.0,
        max_power: float = 1.0,
        step_power: float = 1.0,
        num_power_levels: int = 256,
        angle: float = 0.0,
    ) -> Ops: ...
"#)]
#[pyfunction(name = "rasterize_power_modulation")]
#[pyo3(signature = (gray_image, alpha, pixels_per_mm, offset_x_mm, offset_y_mm, line_interval_mm, sample_interval_mm, min_power=0.0, max_power=1.0, step_power=1.0, num_power_levels=256, angle=0.0))]
#[allow(clippy::too_many_arguments)]
fn py_rasterize_power_modulation(
    py: Python<'_>,
    gray_image: &Bound<'_, PyAny>,
    alpha: &Bound<'_, PyAny>,
    pixels_per_mm: (f64, f64),
    offset_x_mm: f64,
    offset_y_mm: f64,
    line_interval_mm: f64,
    sample_interval_mm: f64,
    min_power: f64,
    max_power: f64,
    step_power: f64,
    num_power_levels: usize,
    angle: f64,
) -> PyResult<PyOps> {
    let (gray, h, w) = extract_flat_u8(py, gray_image)?;
    let (alp, h2, w2) = extract_flat_u8(py, alpha)?;
    debug_assert_eq!(h, h2);
    debug_assert_eq!(w, w2);
    let ops = rasterize_power_modulation(
        &gray,
        &alp,
        h,
        w,
        pixels_per_mm,
        offset_x_mm,
        offset_y_mm,
        line_interval_mm,
        sample_interval_mm,
        min_power,
        max_power,
        step_power,
        num_power_levels,
        angle,
    );
    Ok(PyOps { inner: ops })
}

#[gen_stub_pyfunction(python = r#"
    import numpy
    import numpy.typing
    from raygeo.ops import Ops

    def rasterize_mask_scan(
        mask: numpy.typing.NDArray[numpy.uint8],
        pixels_per_mm: tuple[float, float],
        offset_x_mm: float,
        offset_y_mm: float,
        line_interval_mm: float,
        step_power: float = 1.0,
        angle: float = 0.0,
    ) -> Ops: ...
"#)]
#[pyfunction(name = "rasterize_mask_scan")]
#[pyo3(signature = (mask, pixels_per_mm, offset_x_mm, offset_y_mm, line_interval_mm, step_power=1.0, angle=0.0))]
#[allow(clippy::too_many_arguments)]
fn py_rasterize_mask_scan(
    py: Python<'_>,
    mask: &Bound<'_, PyAny>,
    pixels_per_mm: (f64, f64),
    offset_x_mm: f64,
    offset_y_mm: f64,
    line_interval_mm: f64,
    step_power: f64,
    angle: f64,
) -> PyResult<PyOps> {
    let (m, h, w) = extract_flat_u8(py, mask)?;
    let ops = rasterize_mask_scan(
        &m,
        h,
        w,
        pixels_per_mm,
        offset_x_mm,
        offset_y_mm,
        line_interval_mm,
        step_power,
        angle,
    );
    Ok(PyOps { inner: ops })
}

#[gen_stub_pyfunction(python = r#"
    import numpy
    import numpy.typing
    from raygeo.ops import Ops

    def rasterize_mask_lines(
        mask: numpy.typing.NDArray[numpy.uint8],
        pixels_per_mm: tuple[float, float],
        offset_x_mm: float,
        offset_y_mm: float,
        line_interval_mm: float,
        z: float = 0.0,
        angle: float = 0.0,
    ) -> Ops: ...
"#)]
#[pyfunction(name = "rasterize_mask_lines")]
#[pyo3(signature = (mask, pixels_per_mm, offset_x_mm, offset_y_mm, line_interval_mm, z=0.0, angle=0.0))]
#[allow(clippy::too_many_arguments)]
fn py_rasterize_mask_lines(
    py: Python<'_>,
    mask: &Bound<'_, PyAny>,
    pixels_per_mm: (f64, f64),
    offset_x_mm: f64,
    offset_y_mm: f64,
    line_interval_mm: f64,
    z: f64,
    angle: f64,
) -> PyResult<PyOps> {
    let (m, h, w) = extract_flat_u8(py, mask)?;
    let ops = rasterize_mask_lines(
        &m,
        h,
        w,
        pixels_per_mm,
        offset_x_mm,
        offset_y_mm,
        line_interval_mm,
        z,
        angle,
    );
    Ok(PyOps { inner: ops })
}

#[gen_stub_pyfunction(python = r#"
    import numpy
    import numpy.typing
    from raygeo.ops import Ops

    def rasterize_multi_pass(
        gray_image: numpy.typing.NDArray[numpy.uint8],
        pixels_per_mm: tuple[float, float],
        offset_x_mm: float,
        offset_y_mm: float,
        line_interval_mm: float,
        num_depth_levels: int,
        z_step_down: float,
        angle: float = 0.0,
        angle_increment: float = 0.0,
    ) -> Ops: ...
"#)]
#[pyfunction(name = "rasterize_multi_pass")]
#[pyo3(signature = (gray_image, pixels_per_mm, offset_x_mm, offset_y_mm, line_interval_mm, num_depth_levels, z_step_down, angle=0.0, angle_increment=0.0))]
#[allow(clippy::too_many_arguments)]
fn py_rasterize_multi_pass(
    py: Python<'_>,
    gray_image: &Bound<'_, PyAny>,
    pixels_per_mm: (f64, f64),
    offset_x_mm: f64,
    offset_y_mm: f64,
    line_interval_mm: f64,
    num_depth_levels: usize,
    z_step_down: f64,
    angle: f64,
    angle_increment: f64,
) -> PyResult<PyOps> {
    let (gray, h, w) = extract_flat_u8(py, gray_image)?;
    let ops = rasterize_multi_pass(
        &gray,
        h,
        w,
        pixels_per_mm,
        offset_x_mm,
        offset_y_mm,
        line_interval_mm,
        num_depth_levels,
        z_step_down,
        angle,
        angle_increment,
    );
    Ok(PyOps { inner: ops })
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let raster_mod = PyModule::new(m.py(), "raster")?;

    raster_mod.add_class::<PyScanLine>()?;
    raster_mod.add_function(wrap_pyfunction!(
        py_find_mask_bounding_box,
        raster_mod.clone()
    )?)?;
    raster_mod.add_function(wrap_pyfunction!(
        py_find_segments,
        raster_mod.clone()
    )?)?;
    raster_mod
        .add_function(wrap_pyfunction!(py_line_pixels, raster_mod.clone())?)?;
    raster_mod.add_function(wrap_pyfunction!(
        py_generate_scan_lines,
        raster_mod.clone()
    )?)?;
    raster_mod.add_function(wrap_pyfunction!(
        py_generate_horizontal_scan_positions,
        raster_mod.clone()
    )?)?;
    raster_mod.add_function(wrap_pyfunction!(
        py_resample_rows,
        raster_mod.clone()
    )?)?;
    raster_mod.add_function(wrap_pyfunction!(
        py_downsample_power_values,
        raster_mod.clone()
    )?)?;
    raster_mod.add_function(wrap_pyfunction!(
        py_rasterize_power_modulation,
        raster_mod.clone()
    )?)?;
    raster_mod.add_function(wrap_pyfunction!(
        py_rasterize_mask_scan,
        raster_mod.clone()
    )?)?;
    raster_mod.add_function(wrap_pyfunction!(
        py_rasterize_mask_lines,
        raster_mod.clone()
    )?)?;
    raster_mod.add_function(wrap_pyfunction!(
        py_rasterize_multi_pass,
        raster_mod.clone()
    )?)?;

    m.add_submodule(&raster_mod)?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.raster", &raster_mod)?;

    Ok(())
}
