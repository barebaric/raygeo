//! Burn-effect construction for assemblers.
//!
//! Turns assembled `Ops` (raster scanlines) or cut-footprint polygons
//! (vector outlines) into `MaterialEffect::Raster` fluence maps: the
//! surface-burn input the fold max-reduces into
//! [`MaterialState::surface_map`](super::state::MaterialState).
//!
//! Row convention: fluence buffers are bottom-up — row 0 is the minimum
//! world y — matching the fold's `RasterView` sampling.

use crate::compressed_array::CompressedArray;
use crate::geo::shape::polygon::get_polygon_group_bounds;
use crate::geo::types::Polygon;
use crate::ops::container::Ops;
use crate::ops::material::spec::{GridSpec, LaserPhysics};

/// Per-side pixel cap for assembler-emitted burn grids. The fold
/// re-samples onto its own stock grid anyway, so this only bounds the
/// interchange buffer.
pub(crate) const BURN_MAX_PX: usize = 4096;

/// Density of outline (vector-cut) burn grids, in px/mm.
pub(crate) const OUTLINE_PX_PER_MM: f64 = 20.0;

/// Half-width of the burn line drawn along polygon outlines, in px.
pub(crate) const OUTLINE_THICKNESS_PX: i32 = 1;

/// Rasterize a raster assembler's `Ops` (scanlines) into a burn
/// fluence map covering the part area `(0, 0)`–`size_mm` at the part's
/// own density (isotropically clamped to `max_px` per side).
///
/// The `Ops` carry the 0–255 PWM power fraction; the [`LaserPhysics`]
/// converts each non-zero pixel to fluence (J/cm²) via
/// [`LaserPhysics::fluence_at`].
///
/// Returns `None` when the buffer would be empty or contains no
/// non-zero fluence, so assemblers skip emission for empty output.
pub(crate) fn scanline_fluence_map(
    ops: &Ops,
    size_mm: (f64, f64),
    px_per_mm: (f64, f64),
    laser: &LaserPhysics,
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

    // Convert the 0–255 PWM power map to a float32 fluence map. The
    // conversion is per-pixel and runs once per assembler output, so
    // the cost is negligible relative to the rasterization above.
    let mut fluence: Vec<f32> = Vec::with_capacity(buffer.len());
    let mut any_nonzero = false;
    for &pwm in &buffer {
        if pwm == 0 {
            fluence.push(0.0);
            continue;
        }
        let fraction = pwm as f64 / 255.0;
        let f = laser.fluence_at(fraction) as f32;
        if f > 0.0 {
            any_nonzero = true;
        }
        fluence.push(f);
    }
    if !any_nonzero {
        return None;
    }
    let grid = GridSpec {
        origin_mm: (0.0, 0.0),
        px_per_mm: (ppm, ppm),
        size_px: (w_px as usize, h_px as usize),
    };
    Some((
        CompressedArray::from_vec_f32_with_shape(
            fluence,
            vec![h_px as usize, w_px as usize],
        ),
        grid,
    ))
}

/// Rasterize polygon outlines into a thin burn fluence map.
///
/// The grid covers the polygons' bounds at `px_per_mm` (clamped to
/// `max_px` per side); each edge is traced by incremental sampling
/// and stamped with a square brush of half-width `thickness_px`
/// (max-merged), so a cut line reads as a charred kerf line.
///
/// `power_fraction` (0–1) scales the laser's full-power fluence so a
/// low-power step produces a faint line, not a full char.
///
/// Returns `None` for empty input, zero power, or an all-zero buffer.
pub(crate) fn outline_fluence_map(
    polygons: &[Polygon],
    px_per_mm: f64,
    thickness_px: i32,
    laser: &LaserPhysics,
    power_fraction: f64,
) -> Option<(CompressedArray, GridSpec)> {
    if polygons.is_empty() || power_fraction <= 0.0 {
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

    // The outline carries the step's actual fluence, computed by the
    // identical physics as scanline burns: the laser's full-power
    // fluence scaled by the power fraction. A slow full-power cut
    // deposits more fluence than a fast raster pass, and the char
    // curve accounts for the wider range. A low-power step falls below
    // the char threshold and renders no burn.
    let line_fluence = laser.fluence_at(power_fraction) as f32;

    let mut buffer = vec![0f32; w_px * h_px];
    let (ox, oy) = (bounds.min.x, bounds.min.y);
    let mut drew = false;
    for polygon in polygons {
        let n = polygon.len();
        for i in 0..n {
            let p0 = polygon[i];
            let p1 = polygon[(i + 1) % n];
            drew |= stamp_edge_fluence(
                &mut buffer,
                w_px,
                h_px,
                ((p0.x - ox) * ppm, (p0.y - oy) * ppm),
                ((p1.x - ox) * ppm, (p1.y - oy) * ppm),
                thickness_px,
                line_fluence,
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
    Some((
        CompressedArray::from_vec_f32_with_shape(buffer, vec![h_px, w_px]),
        grid,
    ))
}

/// Trace one edge in grid space and stamp it with a square brush at
/// `fluence` (max-merged). Returns whether any pixel was written.
fn stamp_edge_fluence(
    buffer: &mut [f32],
    w_px: usize,
    h_px: usize,
    from: (f64, f64),
    to: (f64, f64),
    thickness_px: i32,
    fluence: f32,
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
                if fluence > buffer[idx] {
                    buffer[idx] = fluence;
                    drew = true;
                }
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
