//! Alpha composite multiple ARGB32 bitmaps (pre-multiplied) into a
//! target buffer with nearest-neighbour sampling and per-view scaling
//! and positioning.
//!
//! # Coordinate system
//!
//! Both source and target buffers are **Y‑down** (row 0 = top of image).
//! The caller supplies the destination position in target pixel
//! coordinates and scale factors.

/// A single source view to composite into the target.
pub struct ViewInput<'a> {
    /// Source ARGB32 bitmap (pre-multiplied).
    pub bitmap: &'a [u8],
    /// Source width in pixels.
    pub src_w: u32,
    /// Source height in pixels.
    pub src_h: u32,
    /// Destination X position (top‑left of source in target pixels).
    pub dst_x: f64,
    /// Destination Y position (top‑left of source in target pixels).
    pub dst_y: f64,
    /// Horizontal scale factor (src → dst pixels).
    pub scale_x: f64,
    /// Vertical scale factor (src → dst pixels).
    pub scale_y: f64,
}

/// Composite multiple views into a target ARGB32 buffer.
///
/// *target* is assumed zero-initialised.
pub fn composite_views_into(
    target: &mut [u8],
    target_w: u32,
    target_h: u32,
    sources: &[ViewInput<'_>],
) {
    for src in sources {
        composite_one(target, target_w, target_h, src);
    }
}

fn composite_one(
    target: &mut [u8],
    target_w: u32,
    target_h: u32,
    src: &ViewInput<'_>,
) {
    let tw = target_w as i64;
    let th = target_h as i64;

    let left = (src.dst_x.floor() as i64).max(0).min(tw);
    let right = ((src.dst_x + src.src_w as f64 * src.scale_x).ceil() as i64)
        .max(0)
        .min(tw);
    let top = (src.dst_y.floor() as i64).max(0).min(th);
    let bottom = ((src.dst_y + src.src_h as f64 * src.scale_y).ceil() as i64)
        .max(0)
        .min(th);

    if left >= right || top >= bottom {
        return;
    }

    let stride = tw * 4;

    for dy in top..bottom {
        let sy = ((dy as f64 - src.dst_y + 0.5) / src.scale_y - 0.5)
            .round()
            .clamp(0.0, (src.src_h - 1) as f64) as u32;
        let src_row_off = sy as usize * src.src_w as usize * 4;

        for dx in left..right {
            let sx = ((dx as f64 - src.dst_x + 0.5) / src.scale_x - 0.5)
                .round()
                .clamp(0.0, (src.src_w - 1) as f64) as u32;

            let src_off = src_row_off + sx as usize * 4;
            let sa = src.bitmap[src_off + 3] as u32;

            if sa == 0 {
                continue;
            }

            let dst_off = dy as usize * stride as usize + dx as usize * 4;

            if sa == 255 {
                target[dst_off..dst_off + 4]
                    .copy_from_slice(&src.bitmap[src_off..src_off + 4]);
            } else {
                let inv_a = 255 - sa;
                target[dst_off] = ((src.bitmap[src_off] as u32)
                    + (target[dst_off] as u32 * inv_a + 127) / 255)
                    as u8;
                target[dst_off + 1] = ((src.bitmap[src_off + 1] as u32)
                    + (target[dst_off + 1] as u32 * inv_a + 127) / 255)
                    as u8;
                target[dst_off + 2] = ((src.bitmap[src_off + 2] as u32)
                    + (target[dst_off + 2] as u32 * inv_a + 127) / 255)
                    as u8;
                target[dst_off + 3] = ((src.bitmap[src_off + 3] as u32)
                    + (target[dst_off + 3] as u32 * inv_a + 127) / 255)
                    as u8;
            }
        }
    }
}
