use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::raster::scan::{
    self, find_mask_bounding_box, generate_horizontal_scan_positions,
    generate_scan_lines, line_pixels, resample_rows,
};

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

    m.add_submodule(&raster_mod)?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.raster", &raster_mod)?;

    Ok(())
}
