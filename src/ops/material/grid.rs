//! Grid helpers for the fold: budgeted resolution, raster sampling,
//! and per-vertex burn UVs.

use crate::compressed_array::CompressedArray;
use crate::geo::types::Rect;

use super::spec::{GridBudget, GridSpec};

/// Map flat XYZ positions onto a burn power grid as normalized UVs.
///
/// `power_uv = ((xy - origin) * px_per_mm) / size_px`, so a vertex at
/// the grid's origin corner maps to `(0, 0)` and one at the far corner
/// maps to `(1, 1)`. Values outside the grid clamp at the sampler's
/// `GL_CLAMP_TO_EDGE`; the grid is expected to cover the stock AABB.
///
/// `positions` is the mesh's flat per-vertex XYZ array (length `3N`);
/// the result is a flat XY array of length `2N`, index-aligned with
/// the vertices.
pub fn compute_power_uvs(positions: &[f32], grid: &GridSpec) -> Vec<f32> {
    let (ox, oy) = grid.origin_mm;
    let (ppm_x, ppm_y) = grid.px_per_mm;
    let w = grid.size_px.0.max(1) as f64;
    let h = grid.size_px.1.max(1) as f64;
    let mut out = Vec::with_capacity(positions.len() / 3 * 2);
    for chunk in positions.chunks_exact(3) {
        out.push(((chunk[0] as f64 - ox) * ppm_x / w) as f32);
        out.push(((chunk[1] as f64 - oy) * ppm_y / h) as f32);
    }
    out
}

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

/// A decompressed raster fluence map ready for sampling.
pub(crate) struct RasterView<'a> {
    data: Vec<f32>,
    grid: &'a GridSpec,
}

impl<'a> RasterView<'a> {
    /// Decompress a raster effect's fluence map once for repeated
    /// sampling. Returns `None` if decompression fails.
    pub(crate) fn new(
        fluence: &'a CompressedArray,
        grid: &'a GridSpec,
    ) -> Option<Self> {
        let data = fluence.decompress_to_vec_f32().ok()?;
        Some(Self { data, grid })
    }

    /// Nearest-neighbour sample at a world point; 0 outside the grid.
    pub(crate) fn sample(&self, world: (f64, f64)) -> f32 {
        let (w, h) = self.grid.size_px;
        let px = ((world.0 - self.grid.origin_mm.0) * self.grid.px_per_mm.0)
            .floor() as isize;
        let py = ((world.1 - self.grid.origin_mm.1) * self.grid.px_per_mm.1)
            .floor() as isize;
        if px < 0 || py < 0 || px >= w as isize || py >= h as isize {
            return 0.0;
        }
        self.data[py as usize * w + px as usize]
    }
}
