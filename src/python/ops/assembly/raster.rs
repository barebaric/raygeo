use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::container::Ops;
use crate::ops::convert::image::ScanMode;
use crate::ops::cut::ToolPose;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::cut::part::PyPart;
use crate::types::Point3D;

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
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.raster", &m)?;

    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import raygeo

    def raster(
        part: raygeo.ops.cut.Part,
        image: numpy.ndarray,
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
    ) -> raygeo.ops.assembly.result.AssemblyResult:
        """Rasterise a part image into scan paths.

        Converts the grayscale or binary *image* into a sequence of
        scan-line toolpath commands suitable for laser engraving or
        similar raster operations.

        Three modes are supported:

        * ``"power_modulated"`` *(default)* — uses grayscale + alpha
          channels to produce power-modulated scan lines.
        * ``"mask_scan"`` — treats *image* as a binary mask and produces
          scan-line segments with constant power.  Also used for
          ``"dither"`` — the caller pre-ditheres the image and passes
          it as a binary mask.
        * ``"multi_pass"`` — decomposes the grayscale image into
          *num_depth_levels* layers, rasterising each at a progressive
          Z offset.

        When *cross_hatch* is True the scan is run twice — once at
        *angle* and once at *angle* + 90° — and the results are
        concatenated.

        :param part: Part providing pixel density and size metadata.
        :param image: 2-D grayscale (uint8) or binary numpy array.
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
        :returns: An :class:`AssemblyResult` with the raster path.
        :raises ValueError: If the mode is unknown or required data is
            missing.
        """
    "#,
    module = "raygeo.ops.assembly.raster"
)]
#[pyfunction(name = "raster")]
#[pyo3(signature = (
    part,
    image,
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
))]
#[allow(clippy::too_many_arguments)]
fn raster_py(
    py: Python<'_>,
    part: &PyPart,
    image: &Bound<'_, PyAny>,
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
) -> PyResult<PyAssemblyResult> {
    let pixels_per_mm = part.inner.pixels_per_mm.ok_or_else(|| {
        PyValueError::new_err("Part has no pixels_per_mm — required for raster")
    })?;

    let scan_mode_val = match scan_mode {
        "segmented" | "Segmented" => ScanMode::Segmented,
        "full_sweep" | "FullSweep" => ScanMode::FullSweep,
        other => {
            return Err(PyValueError::new_err(format!(
                "Unknown scan_mode '{}' — expected 'segmented' or \
                 'full_sweep'",
                other
            )));
        }
    };

    let (gray, h, w) = extract_flat_u8(py, image)?;

    // Build the list of scan angles (cross-hatch adds 90°).
    let mut angles = vec![angle];
    if cross_hatch {
        angles.push(angle + 90.0);
    }

    let mut combined = Ops::new();

    for &a in &angles {
        let pass = match mode {
            "power_modulated" => {
                let (alp, ah, aw) = match alpha {
                    Some(a_obj) => extract_flat_u8(py, a_obj)?,
                    None => (gray.clone(), h, w),
                };
                if ah != h || aw != w {
                    return Err(PyValueError::new_err(
                        "Alpha array dimensions must match image \
                         dimensions",
                    ));
                }
                Ops::from_power_modulated_image(
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
                    a,
                    scan_mode_val,
                )
            }
            "mask_scan" | "dither" => Ops::from_mask_scan(
                &gray,
                h,
                w,
                pixels_per_mm,
                offset_x_mm,
                offset_y_mm,
                line_interval_mm,
                max_power,
                a,
                scan_mode_val,
            ),
            "multi_pass" => Ops::from_multi_pass_image(
                &gray,
                h,
                w,
                pixels_per_mm,
                offset_x_mm,
                offset_y_mm,
                line_interval_mm,
                num_depth_levels,
                z_step_down,
                a,
                angle_increment,
                scan_mode_val,
            ),
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown mode '{}' — expected 'power_modulated', \
                     'mask_scan', or 'multi_pass'",
                    other
                )));
            }
        };
        combined.extend(&pass);
    }

    Ok(PyAssemblyResult::from_parts(
        combined,
        AssemblyMeta {
            start: ToolPose {
                pos: Point3D::ZERO,
                heading: 0.0,
            },
            end: ToolPose {
                pos: Point3D::ZERO,
                heading: 0.0,
            },
        },
        None,
        vec![],
    ))
}
