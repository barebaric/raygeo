use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pyfunction,
    gen_stub_pymethods,
};

use crate::ops::assembly::raster::rasterize::{
    rasterize_mask_lines, rasterize_mask_scan, rasterize_multi_pass,
    rasterize_power_modulation, ScanMode as RustScanMode,
};
use crate::ops::assembly::raster::scan::{
    self, downsample_power_values, find_mask_bounding_box,
    generate_horizontal_scan_positions, generate_scan_lines, line_pixels,
    resample_rows,
};
use crate::python::ops::container::PyOps;

/// Scan mode for raster operations.
///
/// ``SEGMENTED`` skips zero-power gaps within a scan line.
/// ``FULL_SWEEP`` emits the full line with power values (zeros included).
#[gen_stub_pyclass_enum]
#[pyclass(module = "raygeo.ops.raster", name = "ScanMode", from_py_object)]
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
    /// Create a new ScanLine.
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

    /// The index of this scan line (used to determine alternating direction).
    #[getter]
    fn index(&self) -> i64 {
        self.index
    }

    /// Start position of the scan line in mm space.
    #[getter]
    fn start_mm(&self) -> (f64, f64) {
        self.start_mm
    }

    /// End position of the scan line in mm space.
    #[getter]
    fn end_mm(&self) -> (f64, f64) {
        self.end_mm
    }

    /// Pixel coordinates covered by this scan line.
    #[getter]
    fn pixels(&self) -> Vec<(i32, i32)> {
        self.pixels.clone()
    }

    /// Spacing between scan lines in mm.
    #[getter]
    fn line_interval_mm(&self) -> f64 {
        self.line_interval_mm
    }

    /// Compute the length of this scan line in mm.
    ///
    /// :returns: Total path length in mm.
    /// :complexity: O(1)
    fn length_mm(&self) -> f64 {
        let dx = self.end_mm.0 - self.start_mm.0;
        let dy = self.end_mm.1 - self.start_mm.1;
        (dx * dx + dy * dy).sqrt()
    }

    /// Normalised direction vector from start to end in mm space.
    ///
    /// :returns: ``(dx, dy)`` unit vector.
    /// :complexity: O(1)
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

    /// Convert pixel coordinates to mm space, projected onto this scan line.
    ///
    /// :param px: X pixel coordinate.
    /// :param py: Y pixel coordinate.
    /// :param pixels_per_mm: ``(x, y)`` pixel density in px/mm.
    /// :returns: ``(x, y)`` position in mm, projected onto the scan line.
    /// :complexity: O(1)
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
    module = "raygeo.ops.raster"
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
    module = "raygeo.ops.raster"
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
    module = "raygeo.ops.raster"
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
    from raygeo.ops.raster import ScanLine

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
    module = "raygeo.ops.raster"
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
    module = "raygeo.ops.raster"
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
    module = "raygeo.ops.raster"
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
    module = "raygeo.ops.raster"
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
    module = "raygeo.ops.raster"
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

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing
    from raygeo.ops import Ops
    from raygeo.ops.raster import ScanMode

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
        scan_mode: ScanMode = ScanMode.SEGMENTED,
    ) -> ops.Ops:
        """Rasterise a grayscale image with power-modulated scans.

        Samples the image along scan lines and computes per-pixel power
        values from the grayscale intensity and alpha channel, then
        emits move-to/scan-to commands with the modulated power.

        :param gray_image: 2-D grayscale image (0 = black, 255 = white).
        :param alpha: 2-D alpha mask (0 = transparent/no emission).
        :param pixels_per_mm: ``(x, y)`` pixel density in px/mm.
        :param offset_x_mm: Global X offset in mm.
        :param offset_y_mm: Global Y offset in mm.
        :param line_interval_mm: Spacing between scan lines in mm.
        :param sample_interval_mm: Output sample spacing in mm.
        :param min_power: Minimum power fraction (for white pixels).
        :param max_power: Maximum power fraction (for black pixels).
        :param step_power: Global power multiplier.
        :param num_power_levels: Number of quantised power levels.
        :param angle: Scan angle in degrees.
        :param scan_mode: ``ScanMode.SEGMENTED`` or ``ScanMode.FULL_SWEEP``.
        :returns: An :class:`~raygeo.ops.Ops` container.
        :complexity: O(h * w + n * p) where h, w = image dimensions, n = scan lines, p = pixels per line
        """
"#,
    module = "raygeo.ops.raster"
)]
#[pyfunction(name = "rasterize_power_modulation")]
#[pyo3(signature = (gray_image, alpha, pixels_per_mm, offset_x_mm, offset_y_mm, line_interval_mm, sample_interval_mm, min_power=0.0, max_power=1.0, step_power=1.0, num_power_levels=256, angle=0.0, scan_mode=PyScanMode::Segmented))]
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
    scan_mode: PyScanMode,
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
        scan_mode.into(),
    );
    Ok(PyOps { inner: ops })
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing
    from raygeo.ops import Ops
    from raygeo.ops.raster import ScanMode

    def rasterize_mask_scan(
        mask: numpy.typing.NDArray[numpy.uint8],
        pixels_per_mm: tuple[float, float],
        offset_x_mm: float,
        offset_y_mm: float,
        line_interval_mm: float,
        step_power: float = 1.0,
        angle: float = 0.0,
        scan_mode: ScanMode = ScanMode.SEGMENTED,
    ) -> ops.Ops:
        """Rasterise a binary mask into scan-to commands.

        Generates scan lines covering the mask's bounding box, samples
        the mask along each line, and emits move-to/scan-to commands
        for each non-zero segment (or the full sweep).

        :param mask: 2-D binary mask array.
        :param pixels_per_mm: ``(x, y)`` pixel density in px/mm.
        :param offset_x_mm: Global X offset in mm.
        :param offset_y_mm: Global Y offset in mm.
        :param line_interval_mm: Spacing between scan lines in mm.
        :param step_power: Power value (0-1) for exposed pixels.
        :param angle: Scan angle in degrees.
        :param scan_mode: ``ScanMode.SEGMENTED`` or ``ScanMode.FULL_SWEEP``.
        :returns: An :class:`~raygeo.ops.Ops` container.
        :complexity: O(h * w + n * p) where h, w = image dimensions, n = scan lines, p = pixels per line
        """
"#,
    module = "raygeo.ops.raster"
)]
#[pyfunction(name = "rasterize_mask_scan")]
#[pyo3(signature = (mask, pixels_per_mm, offset_x_mm, offset_y_mm, line_interval_mm, step_power=1.0, angle=0.0, scan_mode=PyScanMode::Segmented))]
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
    scan_mode: PyScanMode,
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
        scan_mode.into(),
    );
    Ok(PyOps { inner: ops })
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing
    from raygeo.ops import Ops
    from raygeo.ops.raster import ScanMode

    def rasterize_mask_lines(
        mask: numpy.typing.NDArray[numpy.uint8],
        pixels_per_mm: tuple[float, float],
        offset_x_mm: float,
        offset_y_mm: float,
        line_interval_mm: float,
        z: float = 0.0,
        angle: float = 0.0,
        scan_mode: ScanMode = ScanMode.SEGMENTED,
    ) -> ops.Ops:
        """Rasterise a binary mask into line-to commands (no power).

        Similar to :func:`rasterize_mask_scan` but emits move-to/line-to
        commands with a Z offset instead of scan-to with power values.
        Useful for simple contour or hatch patterns.

        :param mask: 2-D binary mask array.
        :param pixels_per_mm: ``(x, y)`` pixel density in px/mm.
        :param offset_x_mm: Global X offset in mm.
        :param offset_y_mm: Global Y offset in mm.
        :param line_interval_mm: Spacing between scan lines in mm.
        :param z: Z offset for the lines in mm.
        :param angle: Scan angle in degrees.
        :param scan_mode: ``ScanMode.SEGMENTED`` or ``ScanMode.FULL_SWEEP``.
        :returns: An :class:`~raygeo.ops.Ops` container.
        :complexity: O(h * w + n * p) where h, w = image dimensions, n = scan lines, p = pixels per line
        """
"#,
    module = "raygeo.ops.raster"
)]
#[pyfunction(name = "rasterize_mask_lines")]
#[pyo3(signature = (mask, pixels_per_mm, offset_x_mm, offset_y_mm, line_interval_mm, z=0.0, angle=0.0, scan_mode=PyScanMode::Segmented))]
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
    scan_mode: PyScanMode,
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
        scan_mode.into(),
    );
    Ok(PyOps { inner: ops })
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing
    from raygeo.ops import Ops
    from raygeo.ops.raster import ScanMode

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
        scan_mode: ScanMode = ScanMode.SEGMENTED,
    ) -> ops.Ops:
        """Rasterise a grayscale image as multiple Z-depth passes.

        Decomposes the grayscale image into *num_depth_levels* layers
        by depth-slicing, then rasterises each layer with a progressive
        Z offset and optional per-pass angle increment.

        :param gray_image: 2-D grayscale image (0 = black, 255 = white).
        :param pixels_per_mm: ``(x, y)`` pixel density in px/mm.
        :param offset_x_mm: Global X offset in mm.
        :param offset_y_mm: Global Y offset in mm.
        :param line_interval_mm: Spacing between scan lines in mm.
        :param num_depth_levels: Number of depth layers to produce.
        :param z_step_down: Z decrement per depth layer in mm.
        :param angle: Initial scan angle in degrees.
        :param angle_increment: Angle added per depth layer in degrees.
        :param scan_mode: ``ScanMode.SEGMENTED`` or ``ScanMode.FULL_SWEEP``.
        :returns: An :class:`~raygeo.ops.Ops` container.
        :complexity: O(d * (h * w + n * p)) where d = depth levels, h, w = image dims, n = scan lines, p = pixels per line
        """
"#,
    module = "raygeo.ops.raster"
)]
#[pyfunction(name = "rasterize_multi_pass")]
#[pyo3(signature = (gray_image, pixels_per_mm, offset_x_mm, offset_y_mm, line_interval_mm, num_depth_levels, z_step_down, angle=0.0, angle_increment=0.0, scan_mode=PyScanMode::Segmented))]
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
    scan_mode: PyScanMode,
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
        scan_mode.into(),
    );
    Ok(PyOps { inner: ops })
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let raster_mod = PyModule::new(m.py(), "raster")?;

    raster_mod.add_class::<PyScanMode>()?;
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
        py_extract_zero_power_segments,
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
