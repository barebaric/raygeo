//! Raster assembler: rasterise a part image into scan paths.
//!
//! Pure-Rust core. The Python `raster` pyfunction in
//! `crate::python::ops::assembly::raster` is a thin wrapper that calls
//! [`assemble_raster`] and packs the result into a
//! [`PyAssemblyResult`](crate::python::ops::assembly::result::PyAssemblyResult).
//!
//! The [`RasterSpec`] struct implements the [`Assembler`] trait so
//! callers can dispatch to it without knowing the concrete parameter
//! set.

use crate::error::{RaygeoError, RaygeoResult};
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::{AssembleCtx, Assembler};
use crate::ops::container::Ops;
use crate::ops::convert::image::ScanMode;
use crate::ops::enums::{RasterMode, SectionType};
use crate::ops::part::ImageSource;
use crate::ops::types::ToolPose;
use crate::types::Point3D;

/// Spec for the raster assembler.
///
/// Mirrors the parameter list of [`assemble_raster`]. The optional
/// `alpha` buffer (same dimensions as the image) is carried in the
/// spec because it is per-call data that the assembler reads
/// alongside the image. Held as `Box<dyn Assembler>` by callers that
/// drive the trait.
#[derive(Clone, Debug)]
pub struct RasterSpec {
    pub mode: String,
    pub line_interval_mm: f64,
    pub sample_interval_mm: f64,
    pub min_power: f64,
    pub max_power: f64,
    pub step_power: f64,
    pub num_power_levels: usize,
    pub angle: f64,
    pub offset_x_mm: f64,
    pub offset_y_mm: f64,
    pub scan_mode: String,
    pub cross_hatch: bool,
    pub num_depth_levels: usize,
    pub z_step_down: f64,
    pub angle_increment: f64,
    /// Compensates for the physical width of the laser spot by
    /// delaying laser-on and advancing laser-off by this distance
    /// at each end of every continuous engraved run.
    pub dot_width_correction_mm: f64,
    /// Optional alpha mask buffer (row-major u8, same dimensions as
    /// the image). Required for `power_modulated` mode when the image
    /// is not pre-masked; `None` means the grayscale image is used as
    /// its own alpha.
    pub alpha: Option<Vec<u8>>,
}

impl Assembler for RasterSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        ctx.callbacks.report_progress(0.0, "raster: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let image_src = ctx.image_source.ok_or_else(|| {
            "Part has no image — set part.image before calling raster"
                .to_string()
        })?;
        let pixels_per_mm = ctx.pixels_per_mm.ok_or_else(|| {
            "Part has no pixels_per_mm — required for raster".to_string()
        })?;
        let (ops, meta) = assemble_raster(
            image_src,
            pixels_per_mm,
            self.alpha.as_deref(),
            &self.mode,
            self.line_interval_mm,
            self.sample_interval_mm,
            self.min_power,
            self.max_power,
            self.step_power,
            self.num_power_levels,
            self.angle,
            self.offset_x_mm,
            self.offset_y_mm,
            &self.scan_mode,
            self.cross_hatch,
            self.num_depth_levels,
            self.z_step_down,
            self.angle_increment,
            self.dot_width_correction_mm,
        )
        .map_err(|e| e.to_string())?;
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        ctx.trace.append_ops(&ops);
        ctx.callbacks.report_progress(1.0, "raster: done");
        Ok(meta)
    }

    fn is_scalable(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "raster"
    }

    fn boxed_clone(&self) -> Box<dyn Assembler> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Rasterise a part image into scan paths.
///
/// Reads pixels from `image_src`, converts them into scan-line
/// toolpath commands according to `mode`, and returns the result as
/// an `(Ops, AssemblyMeta)` pair.
///
/// Three modes are supported:
///
/// * `"power_modulated"` — uses grayscale + alpha channels to produce
///   power-modulated scan lines.
/// * `"mask_scan"` / `"dither"` — treats the image as a binary mask
///   and produces scan-line segments with constant power.
/// * `"multi_pass"` — decomposes the grayscale image into
///   `num_depth_levels` layers, rasterising each at a progressive Z
///   offset.
#[allow(clippy::too_many_arguments)]
pub fn assemble_raster(
    image_src: &dyn ImageSource,
    pixels_per_mm: (f64, f64),
    alpha: Option<&[u8]>,
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
) -> RaygeoResult<(Ops, AssemblyMeta)> {
    let (w, h) = image_src.dimensions();
    let gray = image_src.read_all().ok_or_else(|| {
        RaygeoError::ContourError(
            "Part's image source cannot materialise a full buffer — \
             raster requires an in-memory image"
                .to_string(),
        )
    })?;
    let (h, w) = (h as usize, w as usize);

    let scan_mode_val = match scan_mode {
        "segmented" | "Segmented" => ScanMode::Segmented,
        "full_sweep" | "FullSweep" => ScanMode::FullSweep,
        other => {
            return Err(RaygeoError::ContourError(format!(
                "Unknown scan_mode '{}' — expected 'segmented' or \
                 'full_sweep'",
                other
            )));
        }
    };

    let mut angles = vec![angle];
    if cross_hatch {
        angles.push(angle + 90.0);
    }

    let raster_mode = match mode {
        "power_modulated" => Some(RasterMode::VariablePower),
        "mask_scan" | "dither" => Some(RasterMode::ConstantPower),
        "multi_pass" => Some(RasterMode::DepthMap),
        _ => None,
    };

    let mut combined = Ops::new();

    for &a in &angles {
        let pass = match mode {
            "power_modulated" => {
                let (alp, ah, aw) = match alpha {
                    Some(arr) => (arr.to_vec(), h, w),
                    None => (gray.clone(), h, w),
                };
                if ah != h || aw != w {
                    return Err(RaygeoError::ContourError(
                        "Alpha array dimensions must match image \
                         dimensions"
                            .to_string(),
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
                    dot_width_correction_mm,
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
                step_power,
                a,
                scan_mode_val,
                dot_width_correction_mm,
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
                return Err(RaygeoError::ContourError(format!(
                    "Unknown mode '{}' — expected 'power_modulated', \
                     'mask_scan', or 'multi_pass'",
                    other
                )));
            }
        };

        if let Some(rm) = raster_mode {
            let mut wrapped = Ops::new();
            wrapped
                .ops_section_start(SectionType::RasterFill, "raster", Some(rm))
                .map_err(|e| RaygeoError::ContourError(e.to_string()))?;
            wrapped.extend(&pass);
            wrapped
                .ops_section_end(SectionType::RasterFill, Some(rm))
                .map_err(|e| RaygeoError::ContourError(e.to_string()))?;
            combined.extend(&wrapped);
        } else {
            combined.extend(&pass);
        }
    }

    let meta = AssemblyMeta {
        start: ToolPose {
            pos: Point3D::ZERO,
            heading: 0.0,
        },
        end: ToolPose {
            pos: Point3D::ZERO,
            heading: 0.0,
        },
    };
    Ok((combined, meta))
}
