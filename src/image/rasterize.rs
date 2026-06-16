use crate::ops::container::Ops;
use crate::ops::enums::CommandType;

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
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let (mut x, mut y) = (x0, y0);

    loop {
        if x >= 0 && x < width && y >= 0 && y < height {
            let idx = (y * width + x) as usize;
            if power > buffer[idx] {
                buffer[idx] = power;
            }
        }
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
) -> bool {
    let mut has_content = false;
    if iy < 0 || iy >= height_px {
        return false;
    }
    let row_start = (iy * width_px) as usize;
    let mut cx = x;

    for &p in pwr {
        if p > 0 {
            let mut x0 = cx.round() as i32;
            let mut x1 = (cx + dx).round() as i32;
            if x0 > x1 {
                std::mem::swap(&mut x0, &mut x1);
            }
            if x0 < 0 {
                x0 = 0;
            }
            if x1 >= width_px {
                x1 = width_px - 1;
            }
            for xi in x0..=x1 {
                let idx = row_start + xi as usize;
                if p > buffer[idx] {
                    buffer[idx] = p;
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
                if psx >= 0 && psx < width_px && psy >= 0 && psy < height_px {
                    let idx = (psy * width_px + psx) as usize;
                    if p > buffer[idx] {
                        buffer[idx] = p;
                        has_content = true;
                    }
                }
            } else {
                bresenham_line(
                    buffer, width_px, height_px, psx, psy, pex, pey, p,
                );
                has_content = true;
            }
        }
        cx += dx;
        cy += dy;
    }
    has_content
}

pub fn rasterize_scanlines(
    ops: &Ops,
    width_px: u32,
    height_px: u32,
    px_per_mm: (f64, f64),
    origin_mm: (f64, f64),
) -> Vec<u8> {
    let w = width_px as i32;
    let h = height_px as i32;
    let size = (w * h) as usize;
    if size == 0 {
        return Vec::new();
    }

    let (ox, oy) = origin_mm;
    let (px_mm_x, px_mm_y) = px_per_mm;

    let mut buffer = vec![0u8; size];
    let mut current_pos = (0.0f64, 0.0f64, 0.0f64);

    for i in 0..ops.len() {
        let ct = ops.command_type(i);

        if ct == CommandType::MoveTo {
            let end = ops.endpoint(i);
            current_pos = (end.x, end.y, end.z);
            continue;
        }

        if ct != CommandType::ScanLine {
            continue;
        }

        let end = ops.endpoint(i);
        let power_values = ops.scanline_data(i);
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
            );
        }

        current_pos = (end.x, end.y, end.z);
    }

    buffer
}
