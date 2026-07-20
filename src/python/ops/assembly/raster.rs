use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::ops::assembly::raster::{
    assemble_raster, RasterSpec as CoreRasterSpec,
};
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::part::part::PyPart;

/// Extract a flat u8 buffer from a numpy array.
fn extract_flat_u8(
    py: Python<'_>,
    obj: &Bound<'_, PyAny>,
) -> PyResult<(Vec<u8>, usize, usize)> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (obj,))?;
    let shape: (usize, usize) = arr.getattr("shape")?.extract()?;
    let flat: Vec<u8> = arr
        .call_method("astype", ("uint8",), None)?
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;
    Ok((flat, shape.0, shape.1))
}

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "raster")?;
    m.add_function(pyo3::wrap_pyfunction!(raster_py, m.clone())?)?;
    m.add_class::<PyRasterSpec>()?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.raster", &m)?;

    Ok(())
}

/// Parameters for the ``raster`` assembler.
///
/// Construct with ``RasterSpec(mode, line_interval_mm, ...)``. The
/// optional ``alpha`` buffer is set via the ``alpha`` keyword. Wrap
/// in an :class:`~raygeo.ops.assembly.Assembler` instance to drive
/// the `Assembler` trait.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.assembly.raster",
    name = "RasterSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyRasterSpec {
    #[pyo3(get)]
    pub mode: String,
    #[pyo3(get)]
    pub line_interval_mm: f64,
    #[pyo3(get)]
    pub sample_interval_mm: f64,
    #[pyo3(get)]
    pub min_power: f64,
    #[pyo3(get)]
    pub max_power: f64,
    #[pyo3(get)]
    pub step_power: f64,
    #[pyo3(get)]
    pub num_power_levels: usize,
    #[pyo3(get)]
    pub angle: f64,
    #[pyo3(get)]
    pub offset_x_mm: f64,
    #[pyo3(get)]
    pub offset_y_mm: f64,
    #[pyo3(get)]
    pub scan_mode: String,
    #[pyo3(get)]
    pub cross_hatch: bool,
    #[pyo3(get)]
    pub num_depth_levels: usize,
    #[pyo3(get)]
    pub z_step_down: f64,
    #[pyo3(get)]
    pub angle_increment: f64,
    /// Compensates for the physical width of the laser spot by
    /// delaying laser-on and advancing laser-off by this distance
    /// at each end of every continuous engraved run.
    #[pyo3(get)]
    pub dot_width_correction_mm: f64,
    /// Optional alpha mask buffer (row-major u8). Not exposed as a
    /// Python getter because it is a raw buffer; set via the
    /// constructor keyword.
    pub alpha: Option<Vec<u8>>,
}

impl PyRasterSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreRasterSpec {
        CoreRasterSpec {
            mode: self.mode,
            line_interval_mm: self.line_interval_mm,
            sample_interval_mm: self.sample_interval_mm,
            min_power: self.min_power,
            max_power: self.max_power,
            step_power: self.step_power,
            num_power_levels: self.num_power_levels,
            angle: self.angle,
            offset_x_mm: self.offset_x_mm,
            offset_y_mm: self.offset_y_mm,
            scan_mode: self.scan_mode,
            cross_hatch: self.cross_hatch,
            num_depth_levels: self.num_depth_levels,
            z_step_down: self.z_step_down,
            angle_increment: self.angle_increment,
            dot_width_correction_mm: self.dot_width_correction_mm,
            alpha: self.alpha,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyRasterSpec {
    #[new]
    #[pyo3(signature = (
        mode = "power_modulated",
        line_interval_mm = 0.1,
        sample_interval_mm = 0.05,
        min_power = 0.0,
        max_power = 1.0,
        step_power = 0.1,
        num_power_levels = 10,
        angle = 0.0,
        offset_x_mm = 0.0,
        offset_y_mm = 0.0,
        scan_mode = "segmented",
        cross_hatch = false,
        num_depth_levels = 5,
        z_step_down = 0.0,
        angle_increment = 0.0,
        dot_width_correction_mm = 0.0,
        alpha = None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        mode: &str,
        line_interval_mm: f64,
        sample_interval_mm: f64,
        min_power: f64,
        max_power: f64,
        step_power: f64,
        num_power_levels: usize,
        angle: f64,
        offset_x_mm: f64,
        offset_y_mm: f64,
        scan_mode: &str,
        cross_hatch: bool,
        num_depth_levels: usize,
        z_step_down: f64,
        angle_increment: f64,
        dot_width_correction_mm: f64,
        alpha: Option<Vec<u8>>,
    ) -> Self {
        PyRasterSpec {
            mode: mode.to_string(),
            line_interval_mm,
            sample_interval_mm,
            min_power,
            max_power,
            step_power,
            num_power_levels,
            angle,
            offset_x_mm,
            offset_y_mm,
            scan_mode: scan_mode.to_string(),
            cross_hatch,
            num_depth_levels,
            z_step_down,
            angle_increment,
            dot_width_correction_mm,
            alpha,
        }
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import raygeo

    def raster(
        part: raygeo.ops.part.Part,
        alpha: numpy.ndarray | None = None,
        mode: str = "power_modulated",
        line_interval_mm: float = 0.1,
        sample_interval_mm: float = 0.05,
        min_power: float = 0.0,
        max_power: float = 1.0,
        step_power: float = 0.1,
        num_power_levels: int = 10,
        angle: float = 0.0,
        offset_x_mm: float = 0.0,
        offset_y_mm: float = 0.0,
        scan_mode: str = "segmented",
        cross_hatch: bool = False,
        num_depth_levels: int = 5,
        z_step_down: float = 0.0,
        angle_increment: float = 0.0,
        dot_width_correction_mm: float = 0.0,
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Rasterise a part image into scan paths.

        Reads the pixel image from ``part.image`` (a 2-D uint8 numpy
        array) and converts it into scan-line toolpath commands.

        Three modes are supported:

        * ``"power_modulated"`` *(default)* — uses grayscale + alpha
          channels to produce power-modulated scan lines.
        * ``"mask_scan"`` — treats the image as a binary mask and
          produces scan-line segments with constant power.  Also used
          for ``"dither"`` — the caller pre-ditheres the image and
          stores it on ``part.image`` as a binary mask.
        * ``"multi_pass"`` — decomposes the grayscale image into
          *num_depth_levels* layers, rasterising each at a progressive
          Z offset.

        When *cross_hatch* is True the scan is run twice — once at
        *angle* and once at *angle* + 90° — and the results are
        concatenated.

        :param part: Part providing pixel density, size metadata, and
            the image buffer (``part.image``).
        :param alpha: Optional 2-D alpha mask (uint8). Required for
            ``power_modulated`` mode when the image is not pre-masked.
        :param mode: ``"power_modulated"``, ``"mask_scan"``, or
            ``"multi_pass"``.
        :param line_interval_mm: Spacing between scan lines in mm.
        :param sample_interval_mm: Power sampling interval along a scan
            line in mm (power_modulated only).
        :param min_power: Minimum laser power (0–1).
        :param max_power: Maximum laser power (0–1).
        :param step_power: Power step per level.
        :param num_power_levels: Number of discrete power levels.
        :param angle: Scan angle in degrees.
        :param offset_x_mm: Global X offset in mm.
        :param offset_y_mm: Global Y offset in mm.
        :param scan_mode: ``"segmented"`` or ``"full_sweep"``.
        :param cross_hatch: If True, add a second pass at angle + 90°
            (default False).
        :param num_depth_levels: Number of depth layers (multi_pass only,
            default 5).
        :param z_step_down: Z decrement per depth layer in mm
            (multi_pass only, default 0.0).
        :param angle_increment: Angle added per depth layer in degrees
            (multi_pass only, default 0.0).
        :param dot_width_correction_mm: Shortens laser firing by this
            distance at each end of every engraved run, to compensate
            for the physical width of the laser spot. Geometry is
            unaffected. ``power_modulated``/``mask_scan``/``dither``
            only.
        :returns: An :class:`AssemblyResult` with the raster path.
        :raises ValueError: If the mode is unknown, required data is
            missing, or ``part.image`` is None.
        """
    "#,
    module = "raygeo.ops.assembly.raster"
)]
#[allow(clippy::too_many_arguments)]
#[pyfunction(name = "raster")]
#[pyo3(signature = (
    part,
    alpha = None,
    mode = "power_modulated",
    line_interval_mm = 0.1,
    sample_interval_mm = 0.05,
    min_power = 0.0,
    max_power = 1.0,
    step_power = 0.1,
    num_power_levels = 10,
    angle = 0.0,
    offset_x_mm = 0.0,
    offset_y_mm = 0.0,
    scan_mode = "segmented",
    cross_hatch = false,
    num_depth_levels = 5,
    z_step_down = 0.0,
    angle_increment = 0.0,
    dot_width_correction_mm = 0.0,
))]
fn raster_py(
    py: Python<'_>,
    part: &PyPart,
    alpha: Option<&Bound<'_, PyAny>>,
    mode: &str,
    line_interval_mm: f64,
    sample_interval_mm: f64,
    min_power: f64,
    max_power: f64,
    step_power: f64,
    num_power_levels: usize,
    angle: f64,
    offset_x_mm: f64,
    offset_y_mm: f64,
    scan_mode: &str,
    cross_hatch: bool,
    num_depth_levels: usize,
    z_step_down: f64,
    angle_increment: f64,
    dot_width_correction_mm: f64,
) -> PyResult<PyAssemblyResult> {
    let pixels_per_mm = part.inner.pixels_per_mm.ok_or_else(|| {
        PyValueError::new_err("Part has no pixels_per_mm — required for raster")
    })?;

    let image_src = part.inner.image_source.as_ref().ok_or_else(|| {
        PyValueError::new_err(
            "Part has no image — set part.image before calling raster",
        )
    })?;

    let alpha_buf = match alpha {
        Some(a_obj) => {
            let (flat, ah, aw) = extract_flat_u8(py, a_obj)?;
            let (w, h) = image_src.dimensions();
            let (h, w) = (h as usize, w as usize);
            if ah != h || aw != w {
                return Err(PyValueError::new_err(
                    "Alpha array dimensions must match image \
                     dimensions",
                ));
            }
            Some(flat)
        }
        None => None,
    };

    let (ops, meta) = assemble_raster(
        image_src.as_ref(),
        pixels_per_mm,
        alpha_buf.as_deref(),
        mode,
        line_interval_mm,
        sample_interval_mm,
        min_power,
        max_power,
        step_power,
        num_power_levels,
        angle,
        offset_x_mm,
        offset_y_mm,
        scan_mode,
        cross_hatch,
        num_depth_levels,
        z_step_down,
        angle_increment,
        dot_width_correction_mm,
    )?;
    Ok(PyAssemblyResult::from_parts(ops, meta, None, vec![]))
}
