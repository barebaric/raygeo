//! Burn-effect construction for assemblers.
//!
//! Turns assembled `Ops` (raster scanlines) or cut-footprint polygons
//! (vector outlines) into `MaterialEffect::Raster` power maps: the
//! surface-burn input the fold max-reduces into
//! [`MaterialState::surface_map`](super::state::MaterialState).
//!
//! Row convention: power buffers are bottom-up — row 0 is the minimum
//! world y — matching the fold's `RasterView` sampling.

use crate::compressed_array::CompressedArray;
use crate::geo::shape::polygon::get_polygon_group_bounds;
use crate::geo::types::Polygon;
use crate::ops::container::Ops;
use crate::ops::material::spec::GridSpec;

/// Per-side pixel cap for assembler-emitted burn grids. The fold
/// re-samples onto its own stock grid anyway, so this only bounds the
/// interchange buffer.
pub(crate) const BURN_MAX_PX: usize = 4096;

/// Density of outline (vector-cut) burn grids, in px/mm.
pub(crate) const OUTLINE_PX_PER_MM: f64 = 20.0;

/// Half-width of the burn line drawn along polygon outlines, in px.
pub(crate) const OUTLINE_THICKNESS_PX: i32 = 1;

/// Rasterize a raster assembler's `Ops` (scanlines) into a burn power
/// map covering the part area `(0, 0)`–`size_mm` at the part's own
/// density (isotropically clamped to `max_px` per side).
///
/// Returns `None` when the buffer would be empty or contains no
/// non-zero power, so assemblers skip emission for empty output.
pub(crate) fn scanline_power_map(
    ops: &Ops,
    size_mm: (f64, f64),
    px_per_mm: (f64, f64),
) -> Option<(CompressedArray, GridSpec)> {
    let (w_mm, h_mm) = size_mm;
    if w_mm <= 0.0 || h_mm <= 0.0 {
        return None;
    }
    let ppm = px_per_mm.0.min(px_per_mm.1).max(f64::MIN_POSITIVE);
    let ppm = ppm
        .min(BURN_MAX_PX as f64 / w_mm)
        .min(BURN_MAX_PX as f64 / h_mm);
    let w_px = ((w_mm * ppm).ceil() as u32).max(1);
    let h_px = ((h_mm * ppm).ceil() as u32).max(1);

    let mut buffer = ops.to_texture(w_px, h_px, (ppm, ppm), (0.0, 0.0), 0);
    if buffer.is_empty() {
        return None;
    }
    flip_rows(&mut buffer, w_px as usize);
    if buffer.iter().all(|&v| v == 0) {
        return None;
    }
    let grid = GridSpec {
        origin_mm: (0.0, 0.0),
        px_per_mm: (ppm, ppm),
        size_px: (w_px as usize, h_px as usize),
    };
    Some((
        CompressedArray::from_vec_u8(
            buffer,
            vec![h_px as usize, w_px as usize],
        ),
        grid,
    ))
}

/// Rasterize polygon outlines into a thin full-power burn map.
///
/// The grid covers the polygons' bounds at `px_per_mm` (clamped to
/// `max_px` per side); each edge is traced by incremental sampling
/// and stamped with a square brush of half-width `thickness_px`
/// (max-merged), so a cut line reads as a charred kerf line.
///
/// Returns `None` for empty input or an all-zero buffer.
pub(crate) fn outline_power_map(
    polygons: &[Polygon],
    px_per_mm: f64,
    thickness_px: i32,
) -> Option<(CompressedArray, GridSpec)> {
    if polygons.is_empty() {
        return None;
    }
    let bounds = get_polygon_group_bounds(polygons);
    let w_mm = (bounds.max.x - bounds.min.x).max(0.0);
    let h_mm = (bounds.max.y - bounds.min.y).max(0.0);
    let ppm = px_per_mm
        .max(f64::MIN_POSITIVE)
        .min(BURN_MAX_PX as f64 / w_mm.max(f64::MIN_POSITIVE))
        .min(BURN_MAX_PX as f64 / h_mm.max(f64::MIN_POSITIVE));
    let w_px = ((w_mm * ppm).ceil() as usize).max(1);
    let h_px = ((h_mm * ppm).ceil() as usize).max(1);

    let mut buffer = vec![0u8; w_px * h_px];
    let (ox, oy) = (bounds.min.x, bounds.min.y);
    let mut drew = false;
    for polygon in polygons {
        let n = polygon.len();
        for i in 0..n {
            let p0 = polygon[i];
            let p1 = polygon[(i + 1) % n];
            drew |= stamp_edge(
                &mut buffer,
                w_px,
                h_px,
                ((p0.x - ox) * ppm, (p0.y - oy) * ppm),
                ((p1.x - ox) * ppm, (p1.y - oy) * ppm),
                thickness_px,
            );
        }
    }
    if !drew {
        return None;
    }
    let grid = GridSpec {
        origin_mm: (ox, oy),
        px_per_mm: (ppm, ppm),
        size_px: (w_px, h_px),
    };
    Some((CompressedArray::from_vec_u8(buffer, vec![h_px, w_px]), grid))
}

/// Trace one edge in grid space and stamp it with a square brush.
/// Returns whether any pixel was written.
fn stamp_edge(
    buffer: &mut [u8],
    w_px: usize,
    h_px: usize,
    from: (f64, f64),
    to: (f64, f64),
    thickness_px: i32,
) -> bool {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-9 {
        return false;
    }
    // Sample at ~half-pixel steps so fast grid walks cannot skip pixels.
    let steps = ((len * 2.0).ceil() as usize).max(1);
    let mut drew = false;
    for s in 0..=steps {
        let t = s as f64 / steps as f64;
        let cx = from.0 + dx * t;
        let cy = from.1 + dy * t;
        let px = cx.floor() as i32;
        let py = cy.floor() as i32;
        for oy in -thickness_px..=thickness_px {
            for ox in -thickness_px..=thickness_px {
                let x = px + ox;
                let y = py + oy;
                if x < 0 || y < 0 || x >= w_px as i32 || y >= h_px as i32 {
                    continue;
                }
                let idx = y as usize * w_px + x as usize;
                buffer[idx] = 255;
                drew = true;
            }
        }
    }
    drew
}

/// Flip buffer rows in place from top-down (image) order to the
/// bottom-up (world-y) order the fold's `RasterView` expects.
fn flip_rows(buffer: &mut [u8], w_px: usize) {
    let n = buffer.len();
    if w_px == 0 || !n.is_multiple_of(w_px) {
        return;
    }
    let h = n / w_px;
    let mut row = vec![0u8; w_px];
    for r in 0..h / 2 {
        let (a, b) = (r * w_px, (h - 1 - r) * w_px);
        row.copy_from_slice(&buffer[a..a + w_px]);
        let (top, bottom) = buffer.split_at_mut(b);
        top[a..a + w_px].copy_from_slice(&bottom[..w_px]);
        bottom[..w_px].copy_from_slice(&row);
    }
}
