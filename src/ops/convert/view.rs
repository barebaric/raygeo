//! View rendering: rasterise an :class:`Ops` object into a
//! pre-multiplied ARGB32 bitmap on the rayon thread pool.
//!
//! # Coordinate system
//!
//! Ops positions are stored in **mm space, Y‑up** (origin bottom‑left).
//! The output buffer is **Y‑down** (row 0 = top of image).  The
//! transform from mm space to pixel space is:
//!
//! ```text
//! px_x = (x_mm - bbox_min_x_mm) * ppm_x
//! px_y = (bbox_max_y_mm - y_mm) * ppm_y   // Y flip
//! ```
//!
//! The caller supplies ``render_bbox`` — the mm-space area the output
//! bitmap should cover.  Any padding the caller desires must be folded
//! into this bbox before calling; the renderer adds no implicit
//! margin.

use rayon::prelude::*;

use super::vertex_arrays::VertexArrays;
use super::{EncodeCtx, EncodeOutput, Encoder};
use crate::geo::types::Point3D;
use crate::image::wu_line::draw_line_aa;
use crate::ops::container::Ops;

/// All colour fields are in **pre-multiplied ARGB32** (byte order
/// ``[B, G, R, A]`` on little-endian, matching Cairo's
/// ``FORMAT_ARGB32``).  Callers must pass values in this format;
/// raygeo performs no colour-space conversion.
///
/// All colours are resolved by the caller before the call — raygeo has
/// no notion of lasers, layers, or themes.
#[derive(Clone, Debug)]
pub struct ViewSpec {
    /// Pixels per millimetre in ``(x, y)`` order.
    pub pixels_per_mm: (f64, f64),
    /// If ``true`` the renderer also rasterises ``MoveTo`` (travel)
    /// segments.  When ``false`` they are skipped entirely.
    pub show_travel_moves: bool,
    /// The mm-space area the output bitmap covers, as
    /// ``(min_x, min_y, max_x, max_y)``.  The caller computes this
    /// (typically the union of geometry and texture extents, expanded
    /// by any desired padding) and passes it here.
    pub render_bbox: (f64, f64, f64, f64),
    /// Maximum supported side in pixels.  If the bbox would exceed
    /// this in either axis the ppm is scaled down so the bitmap
    /// just fits.
    pub max_dimension_px: u32,
    /// Maximum supported total pixel count.  If the bbox would exceed
    /// this the ppm is scaled down uniformly.
    pub max_total_pixels: u64,
    /// Stroke colour for cut / powered segments at full power.
    /// **Pre-multiplied ARGB32** (``[B×α, G×α, R×α, A]``).
    pub cut_color: [u8; 4],
    /// Stroke colour for travel segments.  **Pre-multiplied ARGB32.**
    pub travel_color: [u8; 4],
    /// Stroke colour for zero‑power cutting segments.
    /// **Pre-multiplied ARGB32.**
    pub zero_power_color: [u8; 4],
    /// 256-entry lookup table mapping a quantised power (0..=255) to
    /// its colour.  Index 0 corresponds to power 0.0, index 255
    /// to power 1.0.  **Pre-multiplied ARGB32 entries.**
    pub cut_lut: [[u8; 4]; 256],
    /// 256-entry lookup table mapping a quantised engrave power
    /// (0..=255) to its premultiplied ARGB32 colour.  Used for
    /// texture / scanline rendering.  The alpha channel is remapped
    /// via ``alpha' = alpha * 0.5 + 0.5`` during rendering to give
    /// low-power pixels a minimum opacity.
    pub engrave_lut: [[u8; 4]; 256],
}

impl Default for ViewSpec {
    fn default() -> Self {
        Self {
            pixels_per_mm: (8.0, 8.0),
            show_travel_moves: true,
            render_bbox: (0.0, 0.0, 0.0, 0.0),
            max_dimension_px: 8192,
            max_total_pixels: 8192u64 * 8192,
            // Pre-multiplied ARGB32 black.
            cut_color: [0, 0, 0, 255],
            // Pre-multiplied ARGB32 yellow.
            travel_color: [0, 255, 255, 255],
            // Pre-multiplied ARGB32 gray.
            zero_power_color: [128, 128, 128, 255],
            cut_lut: [[0, 0, 0, 255]; 256],
            engrave_lut: [[0, 0, 0, 0]; 256],
        }
    }
}

impl Encoder for ViewSpec {
    fn encode(&self, ctx: &mut EncodeCtx<'_>) -> Result<EncodeOutput, String> {
        let va = ctx.ops.to_vertex_arrays();
        match render_vertex_arrays_to_output(ctx.ops, &va, self) {
            Some(output) => Ok(output),
            None => Err("no geometry to render".to_string()),
        }
    }

    fn name(&self) -> &str {
        "view"
    }
}

fn render_vertex_arrays_to_output(
    ops: &Ops,
    va: &VertexArrays,
    spec: &ViewSpec,
) -> Option<EncodeOutput> {
    let result = render_vertex_arrays(ops, va, spec)?;
    Some(EncodeOutput::View {
        buffer: result.buffer,
        width: result.width,
        height: result.height,
        bbox_mm: result.bbox_mm,
        effective_ppm: result.effective_ppm,
    })
}

// ──────────────────────────────────────────────────────────────────
// Public entry points (free functions)
// ──────────────────────────────────────────────────────────────────

/// Output of :func:`render_ops`.
#[derive(Debug)]
pub struct RenderResult {
    /// Flat row‑major RGBA8 bytes of shape ``height * width * 4``.
    pub buffer: Vec<u8>,
    pub width: usize,
    pub height: usize,
    /// ``bbox`` in mm: ``(min_x, min_y, max_x, max_y)``.
    pub bbox_mm: (f64, f64, f64, f64),
    /// Effective pixels/mm applied by the renderer after clamping.
    pub effective_ppm: (f64, f64),
}

/// Rasterise a single :class:`Ops` object into an ARGB32 bitmap.
///
/// The bitmap covers ``spec.render_bbox`` at ``spec.pixels_per_mm``,
/// clamped to ``max_dimension_px`` / ``max_total_pixels``.  Texture
/// (scanline) data is rendered first, then vertex strokes on top.
///
/// Returns ``None`` when the bbox is degenerate (zero area).
pub fn render_ops(ops: &Ops, spec: &ViewSpec) -> Option<RenderResult> {
    let va = ops.to_vertex_arrays();
    render_vertex_arrays(ops, &va, spec)
}

/// Rasterise a batch of :class:`Ops` objects in parallel on the rayon
/// pool.
///
/// ``specs`` must already contain fully resolved ``ViewSpec`` values —
/// raygeo does not attempt to colour-share across workpieces.
///
/// The position of each entry in the returned :exc:`Vec` matches the
/// position of the corresponding input.  Individual items that have no
/// geometry to draw produce :data:`None` in their slot.
pub fn render_ops_batch<'a, I>(items: I) -> Vec<Option<RenderResult>>
where
    I: IntoParallelIterator<Item = (&'a Ops, &'a ViewSpec)> + Send,
{
    items
        .into_par_iter()
        .map(|(ops, spec)| render_ops(ops, spec))
        .collect()
}

/// Render an :class:`Ops` chunk into a caller-provided bitmap.
///
/// Unlike :func:`render_ops`, this does not allocate a buffer — it
/// writes directly into *bitmap* (a flat ``&mut [u8]`` of shape
/// ``h_px * w_px * 4``).  The *view_bbox* ``(min_x, min_y, max_x,
/// max_y)`` defines the mm-space area the bitmap covers; the effective
/// ppm is derived from the bitmap dimensions and the bbox.
///
/// Texture (scanline) data is rendered first, then vertex strokes on
/// top.
pub fn render_ops_into(
    ops: &Ops,
    spec: &ViewSpec,
    bitmap: &mut [u8],
    w_px: usize,
    h_px: usize,
    view_bbox: (f64, f64, f64, f64),
) -> bool {
    let va = ops.to_vertex_arrays();
    let (min_x, min_y, max_x, max_y) = view_bbox;
    let w_mm = (max_x - min_x).max(0.0);
    let h_mm = (max_y - min_y).max(0.0);
    if w_mm < 1e-9 || h_mm < 1e-9 {
        return false;
    }
    let eff_ppm_x = w_px as f64 / w_mm;
    let eff_ppm_y = h_px as f64 / h_mm;

    let to_px = |p: Point3D| -> (f64, f64) {
        let x = (p.x - min_x) * eff_ppm_x;
        let y = (max_y - p.y) * eff_ppm_y;
        (x, y)
    };

    draw_texture(
        ops,
        bitmap,
        w_px,
        h_px,
        min_x,
        min_y,
        eff_ppm_x,
        eff_ppm_y,
        &spec.engrave_lut,
    );

    if spec.show_travel_moves {
        draw_segments(
            bitmap,
            w_px,
            h_px,
            &va.travel_vertices,
            &spec.travel_color,
            to_px,
        );
        draw_segments(
            bitmap,
            w_px,
            h_px,
            &va.zero_power_vertices,
            &spec.zero_power_color,
            to_px,
        );
    }

    draw_powered(
        bitmap,
        w_px,
        h_px,
        &va.powered_vertices,
        &va.powered_colors,
        &spec.cut_lut,
        to_px,
    );
    true
}

// ──────────────────────────────────────────────────────────────────
// Implementation
// ──────────────────────────────────────────────────────────────────
fn render_vertex_arrays(
    ops: &Ops,
    va: &VertexArrays,
    spec: &ViewSpec,
) -> Option<RenderResult> {
    let (min_x, min_y, max_x, max_y) = spec.render_bbox;
    let w_mm = (max_x - min_x).max(0.0);
    let h_mm = (max_y - min_y).max(0.0);
    if w_mm < 1e-9 && h_mm < 1e-9 {
        return None;
    }

    let (w_px, h_px, eff_ppm_x, eff_ppm_y) =
        compute_dimensions(w_mm, h_mm, spec)?;
    if w_px == 0 || h_px == 0 {
        return None;
    }

    let mut buf = vec![0u8; w_px * h_px * 4];

    let to_px = |p: Point3D| -> (f64, f64) {
        let x = (p.x - min_x) * eff_ppm_x;
        let y = (max_y - p.y) * eff_ppm_y;
        (x, y)
    };

    // Texture first, strokes on top.
    draw_texture(
        ops,
        &mut buf,
        w_px,
        h_px,
        min_x,
        min_y,
        eff_ppm_x,
        eff_ppm_y,
        &spec.engrave_lut,
    );

    if spec.show_travel_moves {
        draw_segments(
            &mut buf,
            w_px,
            h_px,
            &va.travel_vertices,
            &spec.travel_color,
            to_px,
        );
        draw_segments(
            &mut buf,
            w_px,
            h_px,
            &va.zero_power_vertices,
            &spec.zero_power_color,
            to_px,
        );
    }

    draw_powered(
        &mut buf,
        w_px,
        h_px,
        &va.powered_vertices,
        &va.powered_colors,
        &spec.cut_lut,
        to_px,
    );

    Some(RenderResult {
        buffer: buf,
        width: w_px,
        height: h_px,
        bbox_mm: (min_x, min_y, max_x, max_y),
        effective_ppm: (eff_ppm_x, eff_ppm_y),
    })
}

fn compute_dimensions(
    w_mm: f64,
    h_mm: f64,
    spec: &ViewSpec,
) -> Option<(usize, usize, f64, f64)> {
    let (ppm_x, ppm_y) = spec.pixels_per_mm;

    let requested_w = (w_mm * ppm_x).round().max(1.0) as i64;
    let requested_h = (h_mm * ppm_y).round().max(1.0) as i64;

    let max_dim = spec.max_dimension_px as i64;
    let mut w_px = requested_w.min(max_dim).max(0) as usize;
    let mut h_px = requested_h.min(max_dim).max(0) as usize;

    let max_total = spec.max_total_pixels as usize;
    if w_px > 0 && h_px > 0 && (w_px * h_px) > max_total {
        let scale = (max_total as f64 / (w_px * h_px) as f64).sqrt();
        w_px = ((w_px as f64) * scale).max(1.0) as usize;
        h_px = ((h_px as f64) * scale).max(1.0) as usize;
    }

    if w_px == 0 || h_px == 0 {
        return None;
    }

    let eff_ppm_x = w_px as f64 / w_mm;
    let eff_ppm_y = h_px as f64 / h_mm;
    Some((w_px, h_px, eff_ppm_x, eff_ppm_y))
}

// ──────────────────────────────────────────────────────────────────
// Texture rendering
// ──────────────────────────────────────────────────────────────────

/// Rasterise scanline commands into the output bitmap, applying the
/// engrave LUT.
///
/// Iterates all ``ScanLine`` commands in *ops*, rasterises them into a
/// power map at the effective ppm, then converts each power value to
/// a premultiplied ARGB32 colour via *engrave_lut* with an alpha remap
/// (``alpha' = alpha * 0.5 + 0.5``) and blends it into the bitmap.
#[allow(clippy::too_many_arguments)]
fn draw_texture(
    ops: &Ops,
    buf: &mut [u8],
    w_px: usize,
    h_px: usize,
    origin_x_mm: f64,
    origin_y_mm: f64,
    eff_ppm_x: f64,
    eff_ppm_y: f64,
    engrave_lut: &[[u8; 4]; 256],
) {
    // Precompute the remapped alpha LUT: for each power level 0..255,
    // produce the final premultiplied ARGB32 colour with
    // alpha' = clip(alpha * 0.5 + 0.5, 0, 255).
    let mut remapped = [[0u8; 4]; 256];
    for i in 0..256 {
        let alpha_f = engrave_lut[i][3] as f64;
        let remapped_alpha = (alpha_f * 0.5 + 127.5).clamp(0.0, 255.0);
        let a = remapped_alpha as u8;
        // Re-premultiply: scale B, G, R by the new alpha / old alpha.
        let old_a = engrave_lut[i][3] as f64;
        let scale = if old_a > 0.0 {
            remapped_alpha / old_a
        } else {
            0.0
        };
        remapped[i] = [
            (engrave_lut[i][0] as f64 * scale).clamp(0.0, 255.0) as u8,
            (engrave_lut[i][1] as f64 * scale).clamp(0.0, 255.0) as u8,
            (engrave_lut[i][2] as f64 * scale).clamp(0.0, 255.0) as u8,
            a,
        ];
    }

    // Rasterise scanlines into a power map.
    let power_map = ops.to_texture(
        w_px as u32,
        h_px as u32,
        (eff_ppm_x, eff_ppm_y),
        (origin_x_mm, origin_y_mm),
        0,
    );

    if power_map.is_empty() {
        return;
    }

    // Blit the power map into the bitmap via the remapped LUT.
    for (px, &power) in power_map.iter().enumerate() {
        if power == 0 {
            continue;
        }
        let color = &remapped[power as usize];
        let buf_idx = px * 4;
        if buf_idx + 4 > buf.len() {
            break;
        }
        // Source-over blend with premultiplied alpha.
        let src_a = color[3] as f64 / 255.0;
        let inv_a = 1.0 - src_a;
        let dst = &mut buf[buf_idx..buf_idx + 4];
        dst[0] = (dst[0] as f64 * inv_a + color[0] as f64) as u8;
        dst[1] = (dst[1] as f64 * inv_a + color[1] as f64) as u8;
        dst[2] = (dst[2] as f64 * inv_a + color[2] as f64) as u8;
        dst[3] = (dst[3] as f64 * inv_a + color[3] as f64) as u8;
    }
}

// ──────────────────────────────────────────────────────────────────
// Segment iteration helpers
// ──────────────────────────────────────────────────────────────────

fn draw_segments<F>(
    buf: &mut [u8],
    w_px: usize,
    h_px: usize,
    vertices: &[f32],
    color: &[u8; 4],
    to_px: F,
) where
    F: Fn(Point3D) -> (f64, f64),
{
    let mut i = 0;
    while i + 5 < vertices.len() {
        let sx = vertices[i] as f64;
        let sy = vertices[i + 1] as f64;
        let sz = vertices[i + 2] as f64;
        let ex = vertices[i + 3] as f64;
        let ey = vertices[i + 4] as f64;
        let ez = vertices[i + 5] as f64;
        let (x1, y1) = to_px(Point3D::new(sx, sy, sz));
        let (x2, y2) = to_px(Point3D::new(ex, ey, ez));
        draw_line_aa(buf, w_px, h_px, x1, y1, x2, y2, color);
        i += 6;
    }
}

/// Draw powered segments using the segment's per-segment power as a
/// LUT index:
///
///   - The first vertex's R channel of each powered-colors segment
///     encodes the segment's power as a grayscale value in ``[0, 1]``.
///   - We quantise it to a :data:`u8` index into ``cut_lut``.
///   - The LUT entry's alpha is remapped via
///     ``alpha' = clip(alpha * 0.5 + 0.5)`` — this gives low-power
///     segments a guaranteed-minimum opacity rather than vanishing
///     entirely.
fn draw_powered<F>(
    buf: &mut [u8],
    w_px: usize,
    h_px: usize,
    vertices: &[f32],
    colors: &[f32],
    cut_lut: &[[u8; 4]; 256],
    to_px: F,
) where
    F: Fn(Point3D) -> (f64, f64),
{
    // Vertices: 6 floats per segment (3 per vertex × 2 vertices).
    // Colors:   8 floats per segment (4 per vertex × 2 vertices).
    let mut v_i = 0;
    let mut c_i = 0;
    while v_i + 5 < vertices.len() && c_i + 7 < colors.len() {
        let power_gray = colors[c_i] as f64;
        let idx = (power_gray * 255.0).clamp(0.0, 255.0) as usize;
        let lut_entry = &cut_lut[idx];

        // Remap alpha: alpha' = alpha * 0.5 + 0.5
        let alpha_f = lut_entry[3] as f64;
        let remapped_alpha = (alpha_f * 0.5 + 127.5).clamp(0.0, 255.0);
        let old_a = alpha_f;
        let scale = if old_a > 0.0 {
            remapped_alpha / old_a
        } else {
            0.0
        };
        let color: [u8; 4] = [
            (lut_entry[0] as f64 * scale).clamp(0.0, 255.0) as u8,
            (lut_entry[1] as f64 * scale).clamp(0.0, 255.0) as u8,
            (lut_entry[2] as f64 * scale).clamp(0.0, 255.0) as u8,
            remapped_alpha as u8,
        ];

        let sx = vertices[v_i] as f64;
        let sy = vertices[v_i + 1] as f64;
        let sz = vertices[v_i + 2] as f64;
        let ex = vertices[v_i + 3] as f64;
        let ey = vertices[v_i + 4] as f64;
        let ez = vertices[v_i + 5] as f64;
        let (x1, y1) = to_px(Point3D::new(sx, sy, sz));
        let (x2, y2) = to_px(Point3D::new(ex, ey, ez));
        draw_line_aa(buf, w_px, h_px, x1, y1, x2, y2, &color);
        v_i += 6;
        c_i += 8;
    }
}
