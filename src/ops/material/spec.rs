//! Fold input types: the stock, the entries, and the grid budget.

use crate::geo::matrix::Matrix;
use crate::geo::types::Polygon;

use crate::ops::material::MaterialEffect;

/// Placement of a raster power map in world mm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridSpec {
    /// World-mm coordinate of the grid's (0, 0) pixel corner.
    pub origin_mm: (f64, f64),
    /// Grid density in pixels per millimetre `(x, y)`.
    pub px_per_mm: (f64, f64),
    /// Grid size in pixels `(width, height)`.
    pub size_px: (usize, usize),
}

/// How a material responds to laser fluence.
///
/// The single point where laser physics is translated into the
/// removal-volume language CNC ops speak natively.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MaterialResponse {
    /// Raster fluence (J/cm²) at or above which the material is cut
    /// through. `None` means the material never cut-throughs from
    /// raster fluence alone.
    pub cut_fluence_threshold: Option<f32>,
}

/// Resolution budget for stock-grid outputs (surface map, and later
/// the depth field).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridBudget {
    /// Requested grid density in pixels per millimetre.
    pub px_per_mm: f64,
    /// Per-side pixel cap; `px_per_mm` is scaled down to fit.
    pub max_px: usize,
}

impl Default for GridBudget {
    fn default() -> Self {
        Self {
            px_per_mm: 50.0,
            max_px: 8192,
        }
    }
}

/// The stock a fold runs against.
///
/// Z semantics: a prismatic stock's top surface is `z = 0`, bottom
/// is `z = -thickness`.
#[derive(Clone, Debug)]
pub enum StockShape {
    /// A prism: 2D outline(s) extruded over the thickness.
    Prismatic {
        /// Stock outline polygons in world mm (outer rings and
        /// holes; holes must wind opposite to outers).
        polygons: Vec<Polygon>,
        /// Stock thickness in mm (positive).
        thickness: f64,
    },
    /// A cylinder: rotary stock, unrolled to a flat burn domain.
    ///
    /// Domain semantics: the axial coordinate maps to world x in
    /// `[0, length]`, the circumference (arc length) maps to world y
    /// in `[-pi * diameter / 2, pi * diameter / 2]`, centered on the
    /// machine origin. Laser ops are expressed in this unrolled
    /// space, so folding is 2D — the wrap onto the shell happens at
    /// render time via per-vertex power UVs.
    Cylinder {
        /// Workpiece diameter in mm (positive).
        diameter: f64,
        /// Axial length in mm (positive).
        length: f64,
    },
}

/// Physical laser parameters for the burn fluence model.
///
/// Carried through the assembly path so the burn emitter can convert
/// the 0–1 PWM power fraction into fluence (J/cm²):
/// `fluence = watts_at(power_fraction) / spot_area * dwell_time`,
/// where `dwell_time = spot_size_y / scan_speed`. All fields use the
/// toolpath-unit conventions: spot size in mm, speed in mm/s.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaserPhysics {
    /// Emission wavelength in nm (used by the shader's absorption
    /// lookup; the burn emitter itself is wavelength-agnostic).
    pub wavelength_nm: f64,
    /// Optical output power in watts at full power (S-value = max).
    pub max_power_watts: f64,
    /// Beam spot size `(x, y)` in mm. `x` is the kerf width, `y` the
    /// scanline spacing; their product is the spot area.
    pub spot_size_mm: (f64, f64),
    /// Scan speed in mm/s (the feed rate the assembler moves at).
    pub scan_speed_mm_per_s: f64,
}

impl Default for LaserPhysics {
    fn default() -> Self {
        // Neutral fallback: 1 W, 0.1 mm square spot, 100 mm/s. Produces
        // a small but non-zero fluence so unconfigured heads still
        // render a visible (if faint) burn rather than nothing.
        Self {
            wavelength_nm: 455.0,
            max_power_watts: 1.0,
            spot_size_mm: (0.1, 0.1),
            scan_speed_mm_per_s: 100.0,
        }
    }
}

impl LaserPhysics {
    /// Fluence (J/cm²) for a given 0–1 power fraction.
    ///
    /// `fluence = (max_power_watts * power_fraction) / spot_area_cm2
    /// * dwell_time_s`, where `spot_area_cm2 = spot_x * spot_y` (mm²
    /// → cm² via /100) and `dwell_time = spot_y / scan_speed`.
    pub fn fluence_at(&self, power_fraction: f64) -> f64 {
        let (sx, sy) = self.spot_size_mm;
        if sx <= 0.0 || sy <= 0.0 || self.scan_speed_mm_per_s <= 0.0 {
            return 0.0;
        }
        let spot_area_cm2 = (sx * sy) / 100.0;
        let dwell_s = sy / self.scan_speed_mm_per_s;
        let watts = self.max_power_watts * power_fraction.clamp(0.0, 1.0);
        watts * dwell_s / spot_area_cm2
    }
}

/// One compute node's contribution to a fold.
#[derive(Clone, Debug)]
pub struct FoldEntry {
    /// Node key of the source (for provenance).
    pub source_key: String,
    /// Workpiece-local → world-mm placement of the effects.
    pub placement: Matrix,
    /// The effects this entry contributes.
    pub effects: Vec<MaterialEffect>,
}

/// Full input to [`fold_effects`](super::fold::fold_effects).
#[derive(Clone, Debug)]
pub struct MaterialFoldSpec {
    /// The stock to fold against.
    pub stock: StockShape,
    /// Effect-bearing entries, in any order (the fold is
    /// order-independent and sorts provenance).
    pub entries: Vec<FoldEntry>,
    /// Grid budget for raster outputs.
    pub grid: GridBudget,
    /// Laser emission wavelength in nm. Carried through to the
    /// [`MaterialState`](super::state::MaterialState) so the renderer
    /// can look up the material's absorption coefficient for the band
    /// that produced the fluence. 0 means "unconfigured" (the renderer
    /// falls back to full absorption).
    pub wavelength_nm: f64,
    /// Optical output power in watts at full power. Carried through to
    /// the [`MaterialState`](super::state::MaterialState) for
    /// provenance; the fold itself does not use it (fluence is
    /// computed at emission time).
    pub max_power_watts: f64,
}

impl Default for MaterialFoldSpec {
    fn default() -> Self {
        Self {
            stock: StockShape::Prismatic {
                polygons: Vec::new(),
                thickness: 1.0,
            },
            entries: Vec::new(),
            grid: GridBudget::default(),
            wavelength_nm: 0.0,
            max_power_watts: 0.0,
        }
    }
}
