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

/// How a material responds to laser power.
///
/// The single point where laser physics is translated into the
/// removal-volume language CNC ops speak natively.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MaterialResponse {
    /// Raster power (0–255) at or above which the material is cut
    /// through. `None` means the material never cut-throughs from
    /// raster power alone.
    pub cut_power_threshold: Option<u8>,
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
}
