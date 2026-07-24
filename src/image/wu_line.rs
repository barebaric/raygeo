//! Xiaolin Wu anti‑aliased line drawing primitives.
//!
//! These operate on raw RGBA8 pixel buffers and are shared between
//! :mod:`crate::image::render` and callers in higher crate layers.

/// Blend a single RGBA pixel with coverage-based alpha.
pub fn blend_pixel(buf: &mut [u8], idx: usize, color: &[u8; 4], coverage: f64) {
    if coverage <= 0.0 || idx + 4 > buf.len() {
        return;
    }
    if coverage >= 1.0 {
        buf[idx..idx + 4].copy_from_slice(color);
        return;
    }
    let inv = 1.0 - coverage;
    buf[idx] = (buf[idx] as f64 * inv + color[0] as f64 * coverage) as u8;
    buf[idx + 1] =
        (buf[idx + 1] as f64 * inv + color[1] as f64 * coverage) as u8;
    buf[idx + 2] =
        (buf[idx + 2] as f64 * inv + color[2] as f64 * coverage) as u8;
    buf[idx + 3] =
        (buf[idx + 3] as f64 * inv + color[3] as f64 * coverage) as u8;
}

/// Plot a coverage-weighted pixel at a fractional ``(x, y)`` position.
pub fn plot(
    buf: &mut [u8],
    w_px: isize,
    h_px: isize,
    x: f64,
    y: f64,
    color: &[u8; 4],
    coverage: f64,
) {
    let xi = x.floor() as isize;
    let yi = y.floor() as isize;
    if xi >= 0 && xi < w_px && yi >= 0 && yi < h_px {
        let idx = (yi * w_px + xi) as usize * 4;
        blend_pixel(buf, idx, color, coverage);
    }
}

/// Fractional part of ``x`` (``x - floor(x)``).
pub fn fpart(x: f64) -> f64 {
    x - x.floor()
}

/// ``1.0 - fpart(x)``.
pub fn rfpart(x: f64) -> f64 {
    1.0 - fpart(x)
}

/// Xiaolin Wu anti‑aliased line — draws a 2‑px wide AA stroke between
/// two pixel‑space points.
#[allow(clippy::too_many_arguments)]
pub fn draw_line_aa(
    buf: &mut [u8],
    w_px: usize,
    h_px: usize,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    color: &[u8; 4],
) {
    let w = w_px as isize;
    let h = h_px as isize;

    if (x1 - x2).abs() < 1e-9 && (y1 - y2).abs() < 1e-9 {
        plot(buf, w, h, x1, y1, color, 1.0);
        return;
    }

    let steep = (y2 - y1).abs() > (x2 - x1).abs();
    let (x1, y1, x2, y2) = if steep {
        (y1, x1, y2, x2)
    } else {
        (x1, y1, x2, y2)
    };
    let (x1, y1, x2, y2) = if x1 > x2 {
        (x2, y2, x1, y1)
    } else {
        (x1, y1, x2, y2)
    };

    let dx = x2 - x1;
    let dy = y2 - y1;
    let gradient = if dx.abs() > 1e-9 { dy / dx } else { 0.0 };

    // ---- first endpoint ----
    let x_end = (x1 + 0.5).floor();
    let y_end = y1 + gradient * (x_end - x1);
    let x_gap = rfpart(x1 + 0.5);
    let x_pxl1 = x_end as isize;
    let y_pxl1 = y_end.floor() as isize;
    if steep {
        plot(
            buf,
            w,
            h,
            y_pxl1 as f64,
            x_pxl1 as f64,
            color,
            rfpart(y_end) * x_gap,
        );
        plot(
            buf,
            w,
            h,
            (y_pxl1 + 1) as f64,
            x_pxl1 as f64,
            color,
            fpart(y_end) * x_gap,
        );
    } else {
        plot(
            buf,
            w,
            h,
            x_pxl1 as f64,
            y_pxl1 as f64,
            color,
            rfpart(y_end) * x_gap,
        );
        plot(
            buf,
            w,
            h,
            x_pxl1 as f64,
            (y_pxl1 + 1) as f64,
            color,
            fpart(y_end) * x_gap,
        );
    }
    let mut intery = y_end + gradient;

    // ---- second endpoint ----
    let x_end = (x2 + 0.5).floor();
    let y_end = y2 + gradient * (x_end - x2);
    let x_gap = rfpart(x2 + 0.5);
    let x_pxl2 = x_end as isize;
    let y_pxl2 = y_end.floor() as isize;
    if steep {
        plot(
            buf,
            w,
            h,
            y_pxl2 as f64,
            x_pxl2 as f64,
            color,
            rfpart(y_end) * x_gap,
        );
        plot(
            buf,
            w,
            h,
            (y_pxl2 + 1) as f64,
            x_pxl2 as f64,
            color,
            fpart(y_end) * x_gap,
        );
    } else {
        plot(
            buf,
            w,
            h,
            x_pxl2 as f64,
            y_pxl2 as f64,
            color,
            rfpart(y_end) * x_gap,
        );
        plot(
            buf,
            w,
            h,
            x_pxl2 as f64,
            (y_pxl2 + 1) as f64,
            color,
            fpart(y_end) * x_gap,
        );
    }

    // ---- main loop ----
    for x in (x_pxl1 + 1)..x_pxl2 {
        if steep {
            let yi = intery.floor() as isize;
            plot(buf, w, h, yi as f64, x as f64, color, rfpart(intery));
            plot(buf, w, h, (yi + 1) as f64, x as f64, color, fpart(intery));
        } else {
            let yi = intery.floor() as isize;
            plot(buf, w, h, x as f64, yi as f64, color, rfpart(intery));
            plot(buf, w, h, x as f64, (yi + 1) as f64, color, fpart(intery));
        }
        intery += gradient;
    }
}
