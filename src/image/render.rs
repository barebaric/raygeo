//! Rasterise geometry into a pixel buffer for preview rendering.
//!
//! # Coordinate system
//!
//! Input geometry is in **mm space, Y‑up** (origin bottom‑left).
//! The output buffer is **Y‑down** (row 0 = top of image).
//! The caller chooses the output resolution via `dpi`.

use crate::geo::geometry::Geometry;
use crate::geo::shape::bezier::get_bezier_point_at;
use crate::types::{Command, Point, Point3D};

/// Options for [`geometry_to_image`].
pub struct RenderOptions {
    /// Output resolution (dots per inch).  Default 96.
    pub dpi: f64,
    /// Background colour as packed RGBA.  Default white (0xFF_FF_FF_FF).
    pub bg_color: u32,
    /// Fill colour for filled polygons as packed RGBA.  Default light gray
    /// (0xD0_D0_D0_FF).
    pub fill_color: u32,
    /// Stroke colour as packed RGBA.  Default black (0x00_00_00_FF).
    pub stroke_color: u32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            dpi: 96.0,
            bg_color: u32::from_ne_bytes([255, 255, 255, 255]),
            fill_color: u32::from_ne_bytes([208, 208, 208, 255]),
            stroke_color: u32::from_ne_bytes([0, 0, 0, 255]),
        }
    }
}

/// Rasterise vector geometry into an RGBA pixel buffer.
///
/// * `strokes` — wire‑frame paths (anti‑aliased lines).
/// * `fills` — closed polygons rendered with scan‑line fill.
/// * `size_mm` — physical size `(width, height)` in millimetres.
/// * `opts` — render options (DPI, colours).
///
/// Returns `(buffer, height, width)` where `buffer` is a flat row‑major
/// RGBA `Vec<u8>` of shape `(height, width, 4)`.
pub fn geometry_to_image(
    strokes: &Geometry,
    fills: &Geometry,
    size_mm: (f64, f64),
    opts: &RenderOptions,
) -> (Vec<u8>, usize, usize) {
    let (w_mm, h_mm) = size_mm;
    let scale = opts.dpi / 25.4;
    let w_px = (w_mm * scale).ceil() as usize;
    let h_px = (h_mm * scale).ceil() as usize;
    if w_px == 0 || h_px == 0 {
        return (vec![], 0, 0);
    }

    let mut buf = vec![0u8; w_px * h_px * 4];

    // Fill background
    let bg = opts.bg_color.to_le_bytes();
    for px in buf.chunks_exact_mut(4) {
        px.copy_from_slice(&bg);
    }

    // ── Fill closed polygons (scan‑line) ───────────────────────────
    let fill_col = opts.fill_color.to_le_bytes();
    if !fills.data.is_empty() {
        let edges = collect_edges(fills, scale, h_mm);
        render_fills(&mut buf, w_px, h_px, &edges, &fill_col);
    }

    // ── Stroke lines (Wu anti‑aliased, consistent 2‑px spread) ────
    let stroke_col = opts.stroke_color.to_le_bytes();
    if !strokes.data.is_empty() {
        render_strokes(&mut buf, w_px, h_px, strokes, scale, h_mm, &stroke_col);
    }

    (buf, h_px, w_px)
}

// ── Edge collection (for scan‑line fill) ─────────────────────────

struct Edge {
    /// y of the upper endpoint (smaller image‑Y)
    y_upper: f64,
    /// y of the lower endpoint (larger image‑Y)
    y_lower: f64,
    /// x at the upper endpoint
    x_upper: f64,
    /// inverse slope: dx/dy
    inv_slope: f64,
}

/// Collect edges from closed‑polygon commands.
/// Only `Move` / `Line` commands that form closed contours are kept.
fn collect_edges(geo: &Geometry, scale: f64, h_mm: f64) -> Vec<Edge> {
    let mut edges = Vec::new();
    let mut start = Point3D::ZERO;
    let mut prev = Point3D::ZERO;
    let mut has_move = false;

    for cmd in &geo.data {
        match cmd {
            Command::Move { end } => {
                if has_move
                    && ((prev.x - start.x).abs() > 1e-9
                        || (prev.y - start.y).abs() > 1e-9)
                {
                    push_edge(&mut edges, prev, start, scale, h_mm);
                }
                start = *end;
                prev = *end;
                has_move = true;
            }
            Command::Line { end } => {
                if has_move {
                    push_edge(&mut edges, prev, *end, scale, h_mm);
                    prev = *end;
                }
            }
            Command::Arc { end, .. } | Command::Bezier { end, .. } => {
                if has_move {
                    push_edge(&mut edges, prev, *end, scale, h_mm);
                    prev = *end;
                }
            }
        }
    }
    if has_move
        && ((prev.x - start.x).abs() > 1e-9 || (prev.y - start.y).abs() > 1e-9)
    {
        push_edge(&mut edges, prev, start, scale, h_mm);
    }

    edges
}

fn push_edge(
    edges: &mut Vec<Edge>,
    from: Point3D,
    to: Point3D,
    scale: f64,
    h_mm: f64,
) {
    let (x1, y1) = (from.x * scale, (h_mm - from.y) * scale);
    let (x2, y2) = (to.x * scale, (h_mm - to.y) * scale);
    let dy = y2 - y1;
    if dy.abs() < 0.5 {
        return;
    }
    if dy > 0.0 {
        edges.push(Edge {
            y_upper: y1,
            y_lower: y2,
            x_upper: x1,
            inv_slope: (x2 - x1) / dy,
        });
    } else {
        edges.push(Edge {
            y_upper: y2,
            y_lower: y1,
            x_upper: x2,
            inv_slope: (x2 - x1) / dy,
        });
    }
}

// ── Scan‑line fill ──────────────────────────────────────────────

fn render_fills(
    buf: &mut [u8],
    w_px: usize,
    h_px: usize,
    edges: &[Edge],
    color: &[u8; 4],
) {
    if edges.is_empty() {
        return;
    }

    let h = h_px as f64;
    for py in 0..h_px {
        let y = py as f64 + 0.5;
        if y < 0.0 || y >= h {
            continue;
        }

        let mut xs: Vec<f64> = Vec::new();
        for e in edges {
            if y >= e.y_upper && y < e.y_lower {
                let x = e.x_upper + e.inv_slope * (y - e.y_upper);
                xs.push(x);
            }
        }
        if xs.len() < 2 {
            continue;
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());

        for chunk in xs.chunks(2) {
            if chunk.len() < 2 {
                break;
            }
            let x0 = chunk[0].max(0.0).min(w_px as f64 - 1.0);
            let x1 = chunk[1].max(0.0).min(w_px as f64 - 1.0);
            let x_start = x0.ceil() as usize;
            let x_end = x1.floor() as usize;
            for px in x_start..=x_end {
                let idx = (py * w_px + px) * 4;
                if idx + 4 <= buf.len() {
                    buf[idx..idx + 4].copy_from_slice(color);
                }
            }
        }
    }
}

// ── Wu anti‑aliased lines ──────────────────────────────────────

fn blend_pixel(buf: &mut [u8], idx: usize, color: &[u8; 4], coverage: f64) {
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

fn plot(
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

fn fpart(x: f64) -> f64 {
    x - x.floor()
}

fn rfpart(x: f64) -> f64 {
    1.0 - fpart(x)
}

/// Xiaolin Wu anti‑aliased line — always draws two pixels per step,
/// giving consistent 2‑pixel visual width for all lines.
#[allow(clippy::too_many_arguments)]
fn draw_line_aa(
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

fn render_strokes(
    buf: &mut [u8],
    w_px: usize,
    h_px: usize,
    geo: &Geometry,
    scale: f64,
    h_mm: f64,
    color: &[u8; 4],
) {
    let mut prev: Option<Point3D> = None;
    for cmd in &geo.data {
        match cmd {
            Command::Move { end } => {
                prev = Some(*end);
            }
            Command::Line { end } => {
                if let Some(p) = prev {
                    let (x1, y1) = (p.x * scale, (h_mm - p.y) * scale);
                    let (x2, y2) = (end.x * scale, (h_mm - end.y) * scale);
                    draw_line_aa(buf, w_px, h_px, x1, y1, x2, y2, color);
                }
                prev = Some(*end);
            }
            Command::Bezier {
                end,
                control1,
                control2,
            } => {
                if let Some(p) = prev {
                    let p0 = Point::new(p.x, p.y);
                    let c1 = Point::new(control1.x, control1.y);
                    let c2 = Point::new(control2.x, control2.y);
                    let p1 = Point::new(end.x, end.y);
                    let est_len = p0.distance(p1)
                        + (c1.distance(p0) + c2.distance(p1)) * 0.5;
                    let steps = (est_len * scale).ceil().max(2.0) as u32;
                    let mut prev_pt = p0;
                    for i in 1..=steps {
                        let t = i as f64 / steps as f64;
                        let pt = get_bezier_point_at(p0, c1, c2, p1, t);
                        let (x1, y1) =
                            (prev_pt.x * scale, (h_mm - prev_pt.y) * scale);
                        let (x2, y2) = (pt.x * scale, (h_mm - pt.y) * scale);
                        draw_line_aa(buf, w_px, h_px, x1, y1, x2, y2, color);
                        prev_pt = pt;
                    }
                }
                prev = Some(*end);
            }
            Command::Arc { end, .. } => {
                if let Some(p) = prev {
                    let (x1, y1) = (p.x * scale, (h_mm - p.y) * scale);
                    let (x2, y2) = (end.x * scale, (h_mm - end.y) * scale);
                    draw_line_aa(buf, w_px, h_px, x1, y1, x2, y2, color);
                }
                prev = Some(*end);
            }
        }
    }
}
