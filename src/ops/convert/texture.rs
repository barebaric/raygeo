//! Convert Ops ScanLine commands into a 2D pixel power-map.
//!
//! Reads ScanLine commands from [`Ops`] and rasterizes them into a
//! `Vec<u8>` pixel buffer using Bresenham line drawing.

use crate::ops::container::Ops;
use crate::ops::convert::{EncodeCtx, EncodeOutput, Encoder};
use crate::ops::enums::CommandType;

/// Stamp a square brush of half-size ``radius_px`` centered on ``(cx, cy)``,
/// writing ``power`` (max-merged) into every covered pixel. Out-of-bounds
/// coverage is dropped (bounds-clamped), never wrapped.
fn stamp_square(
    buffer: &mut [u8],
    width: i32,
    height: i32,
    cx: i32,
    cy: i32,
    radius_px: i32,
    power: u8,
) {
    let x_lo = (cx - radius_px).max(0);
    let x_hi = (cx + radius_px).min(width - 1);
    let y_lo = (cy - radius_px).max(0);
    let y_hi = (cy + radius_px).min(height - 1);
    for ry in y_lo..=y_hi {
        let row_start = (ry * width) as usize;
        for xi in x_lo..=x_hi {
            let idx = row_start + xi as usize;
            if power > buffer[idx] {
                buffer[idx] = power;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn bresenham_line(
    buffer: &mut [u8],
    width: i32,
    height: i32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    power: u8,
    radius_px: i32,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let (mut x, mut y) = (x0, y0);

    loop {
        stamp_square(buffer, width, height, x, y, radius_px, power);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rasterize_horizontal(
    buffer: &mut [u8],
    iy: i32,
    height_px: i32,
    width_px: i32,
    x: f64,
    dx: f64,
    pwr: &[u8],
    radius_px: i32,
) -> bool {
    let mut has_content = false;
    if iy < 0 || iy >= height_px {
        return false;
    }
    let row_lo = (iy - radius_px).max(0);
    let row_hi = (iy + radius_px).min(height_px - 1);
    let mut cx = x;

    for &p in pwr {
        if p > 0 {
            let mut x0 = cx.round() as i32 - radius_px;
            let mut x1 = (cx + dx).round() as i32 + radius_px;
            if x0 > x1 {
                std::mem::swap(&mut x0, &mut x1);
            }
            if x0 < 0 {
                x0 = 0;
            }
            if x1 >= width_px {
                x1 = width_px - 1;
            }
            for ry in row_lo..=row_hi {
                let row_start = (ry * width_px) as usize;
                for xi in x0..=x1 {
                    let idx = row_start + xi as usize;
                    if p > buffer[idx] {
                        buffer[idx] = p;
                    }
                }
            }
            has_content = true;
        }
        cx += dx;
    }
    has_content
}

#[allow(clippy::too_many_arguments)]
fn rasterize_diagonal(
    buffer: &mut [u8],
    width_px: i32,
    height_px: i32,
    x: f64,
    y: f64,
    dx: f64,
    dy: f64,
    pwr: &[u8],
    radius_px: i32,
) -> bool {
    let mut has_content = false;
    let mut cx = x;
    let mut cy = y;

    for &p in pwr {
        if p > 0 {
            let psx = cx.round() as i32;
            let psy = cy.round() as i32;
            let pex = (cx + dx).round() as i32;
            let pey = (cy + dy).round() as i32;

            if psx == pex && psy == pey {
                stamp_square(
                    buffer, width_px, height_px, psx, psy, radius_px, p,
                );
                has_content = true;
            } else {
                bresenham_line(
                    buffer, width_px, height_px, psx, psy, pex, pey, p,
                    radius_px,
                );
                has_content = true;
            }
        }
        cx += dx;
        cy += dy;
    }
    has_content
}

impl Ops {
    /// Rasterize ScanLine commands into a 2D pixel power-map buffer.
    ///
    /// Iterates all scanline commands, converts their mm coordinates
    /// to pixel space, and returns a `Vec<u8>` where each pixel holds
    /// the maximum power value written to it.
    ///
    /// When `radius_px` is greater than zero, each rasterized sample is
    /// expanded to a square brush of side `2*radius_px + 1` (max-merged),
    /// which is equivalent to a square morphological dilation of the thin
    /// raster. Coverage is bounds-clamped at the texture edges.
    pub fn to_texture(
        &self,
        width_px: u32,
        height_px: u32,
        px_per_mm: (f64, f64),
        origin_mm: (f64, f64),
        radius_px: i32,
    ) -> Vec<u8> {
        let w = width_px as i32;
        let h = height_px as i32;
        let size = (w * h) as usize;
        if size == 0 {
            return Vec::new();
        }
        let radius_px = radius_px.max(0);

        let (ox, oy) = origin_mm;
        let (px_mm_x, px_mm_y) = px_per_mm;

        let mut buffer = vec![0u8; size];
        let mut current_pos = (0.0f64, 0.0f64, 0.0f64);

        for i in 0..self.len() {
            let ct = self.command_type(i);

            if ct == CommandType::MoveTo {
                let end = self.endpoint(i);
                current_pos = (end.x, end.y, end.z);
                continue;
            }

            if ct != CommandType::ScanLine {
                continue;
            }

            let end = self.endpoint(i);
            let power_values = self.scanline_data(i);
            let num_steps = power_values.len();
            if num_steps == 0 {
                current_pos = (end.x, end.y, end.z);
                continue;
            }

            let sx = (current_pos.0 - ox) * px_mm_x;
            let sy = h as f64 - (current_pos.1 - oy) * px_mm_y;
            let ex = (end.x - ox) * px_mm_x;
            let ey = h as f64 - (end.y - oy) * px_mm_y;

            let dx = (ex - sx) / num_steps as f64;
            let dy = (ey - sy) / num_steps as f64;

            if dy == 0.0 && dx != 0.0 {
                rasterize_horizontal(
                    &mut buffer,
                    sy.round() as i32,
                    h,
                    w,
                    sx,
                    dx,
                    &power_values,
                    radius_px,
                );
            } else if dx != 0.0 || dy != 0.0 {
                rasterize_diagonal(
                    &mut buffer,
                    w,
                    h,
                    sx,
                    sy,
                    dx,
                    dy,
                    &power_values,
                    radius_px,
                );
            }

            current_pos = (end.x, end.y, end.z);
        }

        buffer
    }
}

/// Spec for the texture encoder.
///
/// Carries the target texture dimensions and the mm→pixel mapping.
/// Calls [`Ops::to_texture`] on the upstream ops.
#[derive(Clone, Debug)]
pub struct TextureSpec {
    /// Texture width in pixels.
    pub width_px: u32,
    /// Texture height in pixels.
    pub height_px: u32,
    /// `(x, y)` resolution in pixels per millimetre.
    pub px_per_mm: (f64, f64),
    /// `(x, y)` origin offset in millimetres.
    pub origin_mm: (f64, f64),
}

impl Encoder for TextureSpec {
    fn encode(&self, ctx: &mut EncodeCtx<'_>) -> Result<EncodeOutput, String> {
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        ctx.callbacks.report_progress(0.0, "texture: encode");
        let buffer = ctx.ops.to_texture(
            self.width_px,
            self.height_px,
            self.px_per_mm,
            self.origin_mm,
            0,
        );
        ctx.callbacks.report_progress(1.0, "texture: done");
        Ok(EncodeOutput::Texture {
            power_texture: buffer,
            width_px: self.width_px,
            height_px: self.height_px,
        })
    }

    fn name(&self) -> &str {
        "texture"
    }
}
