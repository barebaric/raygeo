use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pyfunction,
    gen_stub_pymethods,
};

use crate::image::scan::{
    self, downsample_power_values, find_mask_bounding_box,
    generate_horizontal_scan_positions, generate_scan_lines, line_pixels,
    resample_rows,
};
use crate::ops::convert::image::ScanMode as RustScanMode;

/// Scan mode for raster operations.
///
/// ``SEGMENTED`` skips zero-power gaps within a scan line.
/// ``FULL_SWEEP`` emits the full line with power values (zeros included).
#[gen_stub_pyclass_enum]
#[pyclass(module = "raygeo.image.scan", name = "ScanMode", from_py_object)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PyScanMode {
    #[pyo3(name = "SEGMENTED")]
    Segmented,
    #[pyo3(name = "FULL_SWEEP")]
    FullSweep,
}

impl From<PyScanMode> for RustScanMode {
    fn from(mode: PyScanMode) -> Self {
        match mode {
            PyScanMode::Segmented => RustScanMode::Segmented,
            PyScanMode::FullSweep => RustScanMode::FullSweep,
        }
    }
}

#[pymethods]
impl PyScanMode {
    fn __repr__(&self) -> String {
        match self {
            PyScanMode::Segmented => "ScanMode.SEGMENTED".to_string(),
            PyScanMode::FullSweep => "ScanMode.FULL_SWEEP".to_string(),
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    #[getter]
    fn name(&self) -> &str {
        match self {
            PyScanMode::Segmented => "SEGMENTED",
            PyScanMode::FullSweep => "FULL_SWEEP",
        }
    }
}

/// A single scan line with its pixel coverage and mm-space endpoints.
///
/// Produced by :func:`generate_scan_lines`. Each line has a unique
/// index, start/end positions in mm, and the set of pixels it
/// intersects in the image.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.image.scan", name = "ScanLine", skip_from_py_object)]
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

#[gen_stub_pyfunction(
    python = r#"
    import numpy

    def find_mask_bounding_box(
        mask: numpy.ndarray,
    ) -> tuple[int, int, int, int] | None:
        """Find the bounding box of non-zero pixels in a binary mask.

        Scans the mask and returns the (y_min, y_max, x_min, x_max) of
        the smallest axis-aligned rectangle covering all non-zero pixels.

        :param mask: 2-D binary mask array.
        :returns: ``(y_min, y_max, x_min, x_max)`` pixel coordinates,
            or ``None`` if the mask is entirely zero.
        :complexity: O(h*w)
        """
"#,
    module = "raygeo.image.scan"
)]
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

#[gen_stub_pyfunction(
    python = r#"
    import numpy

    def find_segments(
        values: numpy.ndarray,
    ) -> list[tuple[int, int]]:
        """Find contiguous non-zero segments in a 1-D array.

        Returns a list of ``(start, end)`` index pairs covering every
        run of consecutive non-zero values.

        :param values: 1-D array of byte values.
        :returns: List of ``(start, end)`` index pairs.
        :complexity: O(n)
        """
"#,
    module = "raygeo.image.scan"
)]
#[pyfunction(name = "find_segments")]
fn py_find_segments(
    values: &Bound<'_, PyAny>,
) -> PyResult<Vec<(usize, usize)>> {
    let list: Vec<u8> = values.call_method0("tolist")?.extract()?;
    Ok(scan::find_segments(&list))
}

#[gen_stub_pyfunction(
    python = r#"
    def line_pixels(
        start: tuple[float, float],
        end: tuple[float, float],
        width: int,
        height: int,
    ) -> list[tuple[int, int]]:
        """Rasterise a line segment into pixel coordinates.

        Uses Bresenham's line algorithm to enumerate all integer pixel
        positions intersecting the line from *start* to *end*, clipped
        to the image dimensions ``(width, height)``.

        :param start: (x, y) start position in pixel coordinates.
        :param end: (x, y) end position in pixel coordinates.
        :param width: Image width in pixels.
        :param height: Image height in pixels.
        :returns: List of ``(x, y)`` pixel coordinates on the line.
        :complexity: O(n) where n = number of pixels on the line
        """
"#,
    module = "raygeo.image.scan"
)]
#[pyfunction(name = "line_pixels")]
fn py_line_pixels(
    start: (f64, f64),
    end: (f64, f64),
    width: i32,
    height: i32,
) -> Vec<(i32, i32)> {
    line_pixels(start, end, width, height)
}

#[gen_stub_pyfunction(
    python = r#"
    from raygeo.image.scan import ScanLine

    def generate_scan_lines(
        bbox: tuple[int, int, int, int],
        image_size: tuple[int, int],
        pixels_per_mm: tuple[float, float],
        line_interval_mm: float,
        direction_degrees: float = 0.0,
        offset_x_mm: float = 0.0,
        offset_y_mm: float = 0.0,
        global_center_mm: tuple[float, float] | None = None,
    ) -> list[ScanLine]:
        """Generate scan lines covering a bounding box.

        Creates a set of parallel scan lines at a given angle and
        spacing that cover the bounding box region. Each line is
        rasterised to pixels and stored as a :class:`ScanLine`.

        :param bbox: ``(y_min, y_max, x_min, x_max)`` of the region.
        :param image_size: ``(width, height)`` of the image in pixels.
        :param pixels_per_mm: ``(x, y)`` pixel density in px/mm.
        :param line_interval_mm: Spacing between scan lines in mm.
        :param direction_degrees: Scan direction angle in degrees.
        :param offset_x_mm: Global X offset in mm.
        :param offset_y_mm: Global Y offset in mm.
        :param global_center_mm: Optional rotation centre in mm;
            defaults to the bbox centre + offset.
        :returns: List of :class:`ScanLine` objects.
        :complexity: O(n * p) where n = number of lines, p = pixels per line
        """
"#,
    module = "raygeo.image.scan"
)]
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

#[gen_stub_pyfunction(
    python = r#"
    def generate_horizontal_scan_positions(
        y_min_px: int,
        y_max_px: int,
        height_px: int,
        pixels_per_mm: tuple[float, float],
        line_interval_mm: float,
        offset_y_mm: float,
    ) -> tuple[list[float], list[float]]:
        """Compute Y positions for horizontal scan lines.

        Given a vertical pixel range, computes the mm and pixel Y
        coordinates of evenly-spaced scan lines (aligned to a global
        grid defined by *line_interval_mm* and *offset_y_mm*).

        :param y_min_px: Minimum Y pixel coordinate.
        :param y_max_px: Maximum Y pixel coordinate.
        :param height_px: Image height in pixels.
        :param pixels_per_mm: ``(x, y)`` pixel density in px/mm.
        :param line_interval_mm: Spacing between scan lines in mm.
        :param offset_y_mm: Global Y offset in mm.
        :returns: ``(y_coords_mm, y_coords_px)`` tuple of Y positions.
        :complexity: O(n) where n = number of scan lines
        """
"#,
    module = "raygeo.image.scan"
)]
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

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def resample_rows(
        image: numpy.typing.NDArray[numpy.uint8],
        y_coords_px: numpy.ndarray,
    ) -> numpy.typing.NDArray[numpy.uint8]:
        """Resample image rows at arbitrary Y coordinates.

        Performs linear interpolation between adjacent rows to sample
        the image at the given (potentially fractional) Y positions.

        :param image: 2-D input image array.
        :param y_coords_px: 1-D array of Y pixel coordinates.
        :returns: 2-D array with shape ``(len(y_coords_px), width)``.
        :complexity: O(m * w) where m = output rows, w = image width
        """
"#,
    module = "raygeo.image.scan"
)]
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

#[gen_stub_pyfunction(
    python = r#"
    import numpy

    def downsample_power_values(
        power_values: numpy.ndarray,
        start_mm: tuple[float, float],
        end_mm: tuple[float, float],
        sample_interval_mm: float,
    ) -> tuple[numpy.ndarray, numpy.ndarray, numpy.ndarray]:
        """Downsample power values along a scan segment.

        If the sample interval is larger than the native pixel spacing,
        the power values are resampled by nearest-neighbour at the
        target spacing. Otherwise the original values are returned
        with their corresponding positions.

        :param power_values: 1-D array of byte power values.
        :param start_mm: ``(x, y)`` start position of the segment in mm.
        :param end_mm: ``(x, y)`` end position of the segment in mm.
        :param sample_interval_mm: Desired sample spacing in mm.
        :returns: ``(power, x_mm, y_mm)`` of downsampled values.
        :complexity: O(n) where n = number of power values
        """
"#,
    module = "raygeo.image.scan"
)]
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

#[gen_stub_pyfunction(
    python = r#"
    def extract_zero_power_segments(
        start: tuple[float, float, float],
        end: tuple[float, float, float],
        power_values: bytes,
    ) -> list[float]:
        """Extract zero-power segment endpoints from scanline power data.

        Finds contiguous runs of zero values in *power_values* and computes
        their 3D start/end points via linear interpolation along the
        scanline segment from *start* to *end*.

        :param start: (x, y, z) start position of the scanline in mm.
        :param end: (x, y, z) end position of the scanline in mm.
        :param power_values: Per-step power bytes.
        :returns: Flat list of ``[sx, sy, sz, ex, ey, ez, ...]`` segments.
        :complexity: O(n) where n = number of steps
        """
"#,
    module = "raygeo.image.scan"
)]
#[pyfunction(name = "extract_zero_power_segments")]
fn py_extract_zero_power_segments(
    start: (f64, f64, f64),
    end: (f64, f64, f64),
    power_values: &Bound<'_, PyAny>,
) -> PyResult<Vec<f32>> {
    let pv: Vec<u8> = if let Ok(b) = power_values.cast::<PyBytes>() {
        b.as_bytes().to_vec()
    } else {
        power_values.call_method0("tolist")?.extract()?
    };
    Ok(scan::extract_zero_power_segments(start, end, &pv))
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let scan_mod = PyModule::new(m.py(), "scan")?;

    scan_mod.add_class::<PyScanMode>()?;
    scan_mod.add_class::<PyScanLine>()?;
    scan_mod.add_function(wrap_pyfunction!(
        py_find_mask_bounding_box,
        scan_mod.clone()
    )?)?;
    scan_mod
        .add_function(wrap_pyfunction!(py_find_segments, scan_mod.clone())?)?;
    scan_mod
        .add_function(wrap_pyfunction!(py_line_pixels, scan_mod.clone())?)?;
    scan_mod.add_function(wrap_pyfunction!(
        py_generate_scan_lines,
        scan_mod.clone()
    )?)?;
    scan_mod.add_function(wrap_pyfunction!(
        py_generate_horizontal_scan_positions,
        scan_mod.clone()
    )?)?;
    scan_mod
        .add_function(wrap_pyfunction!(py_resample_rows, scan_mod.clone())?)?;
    scan_mod.add_function(wrap_pyfunction!(
        py_downsample_power_values,
        scan_mod.clone()
    )?)?;
    scan_mod.add_function(wrap_pyfunction!(
        py_extract_zero_power_segments,
        scan_mod.clone()
    )?)?;

    m.add_submodule(&scan_mod)?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.image.scan", &scan_mod)?;

    Ok(())
}
