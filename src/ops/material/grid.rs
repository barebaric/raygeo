//! Grid helpers for the fold: budgeted resolution and raster
//! sampling.

use crate::compressed_array::CompressedArray;
use crate::geo::types::Rect;

use super::spec::{GridBudget, GridSpec};

/// Resolve the stock grid for raster outputs: the stock AABB at the
/// requested density, with `px_per_mm` scaled down so each side
/// respects the budget cap.
pub(crate) fn resolve_grid(bounds: &Rect, budget: &GridBudget) -> GridSpec {
    let x0 = bounds.min.x;
    let y0 = bounds.min.y;
    let x1 = bounds.max.x;
    let y1 = bounds.max.y;
    let w_mm = (x1 - x0).max(0.0);
    let h_mm = (y1 - y0).max(0.0);
    let mut ppm = budget.px_per_mm.max(f64::MIN_POSITIVE);
    let cap = budget.max_px.max(1) as f64;
    if w_mm * ppm > cap {
        ppm = cap / w_mm;
    }
    if h_mm * ppm > cap {
        ppm = ppm.min(cap / h_mm);
    }
    let w_px = ((w_mm * ppm).ceil() as usize).max(1);
    let h_px = ((h_mm * ppm).ceil() as usize).max(1);
    GridSpec {
        origin_mm: (x0, y0),
        px_per_mm: (ppm, ppm),
        size_px: (w_px, h_px),
    }
}

/// A decompressed raster power map ready for sampling.
pub(crate) struct RasterView<'a> {
    data: Vec<u8>,
    grid: &'a GridSpec,
}

impl<'a> RasterView<'a> {
    /// Decompress a raster effect's power map once for repeated
    /// sampling. Returns `None` if decompression fails.
    pub(crate) fn new(
        power: &'a CompressedArray,
        grid: &'a GridSpec,
    ) -> Option<Self> {
        let data = power.decompress_to_vec_u8().ok()?;
        Some(Self { data, grid })
    }

    /// Nearest-neighbour sample at a world point; 0 outside the grid.
    pub(crate) fn sample(&self, world: (f64, f64)) -> u8 {
        let (w, h) = self.grid.size_px;
        let px = ((world.0 - self.grid.origin_mm.0) * self.grid.px_per_mm.0)
            .floor() as isize;
        let py = ((world.1 - self.grid.origin_mm.1) * self.grid.px_per_mm.1)
            .floor() as isize;
        if px < 0 || py < 0 || px >= w as isize || py >= h as isize {
            return 0;
        }
        self.data[py as usize * w + px as usize]
    }
}
