//! Renderer-facing constraint symbol drawing.
//!
//! The constraint `draw` methods replay their visual representation
//! onto a canvas through the [`CairoLike`] interface: a subset of the
//! cairo context API.  All layout decisions (marker positions, sizes,
//! offsets, label placement, status colors) are made here in core;
//! label glyphs are generated through the same text stack the text
//! box entity uses, so no renderer-side text layout engine is
//! involved.

use crate::geo::shape::text::{
    get_font_metrics, get_text_width, text_to_geometry, FontConfig,
};
use crate::geo::types::Command;

/// Validation status of a constraint, mirroring the reference
/// `ConstraintStatus` enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstraintStatus {
    Valid,
    ExpressionBased,
    Error,
    Conflicting,
}

impl ConstraintStatus {
    /// Resolves a status from its reference enum ordinal (the
    /// reference `ConstraintStatus` enum numbers members from 1:
    /// valid, expression-based, error, conflicting).
    pub fn from_ordinal(value: u8) -> ConstraintStatus {
        match value {
            2 => ConstraintStatus::ExpressionBased,
            3 => ConstraintStatus::Error,
            4 => ConstraintStatus::Conflicting,
            _ => ConstraintStatus::Valid,
        }
    }
}

/// Style inputs shared by all `draw` methods.
#[derive(Clone, Copy, Debug)]
pub struct DrawStyle {
    /// Validation status driving colors.
    pub status: ConstraintStatus,
    /// Whether the constraint is currently selected.
    pub is_selected: bool,
    /// Whether the constraint is currently hovered.
    pub is_hovered: bool,
    /// Radius of the rendered sketch points, in screen pixels.
    pub point_radius: f64,
}

impl Default for DrawStyle {
    fn default() -> Self {
        DrawStyle {
            status: ConstraintStatus::Valid,
            is_selected: false,
            is_hovered: false,
            point_radius: 5.0,
        }
    }
}

/// One point as seen by the symbol layout: position and fixed flag.
#[derive(Clone, Copy, Debug)]
pub struct DrawPoint {
    pub x: f64,
    pub y: f64,
    pub fixed: bool,
}

/// Read view over the caller's sketch document, addressed by point
/// id.  The symbol layout reads live coordinates through this
/// protocol instead of a snapshot, mirroring how the solver reads
/// coordinates through [`crate::sketch::constraint::PointContext`].
pub trait DrawContext {
    /// Returns the `(x, y)` position of a point id.
    fn get_point(&self, id: u64) -> Option<DrawPoint>;

    /// Returns the entity ids of the document.
    fn entity_ids(&self) -> Vec<u64>;

    /// Returns the type name of an entity id.
    fn entity_type(&self, id: u64) -> Option<std::borrow::Cow<'static, str>>;

    /// Returns the point ids of an entity id.
    fn entity_point_ids(&self, id: u64) -> Option<Vec<u64>>;

    /// Whether the point is one of a text box frame's corner points.
    fn point_in_text_box_frame(&self, point_id: u64) -> bool;
}

/// The subset of the cairo context API the constraint symbols need.
///
/// The Python binding maps this onto the caller's pycairo context
/// (or any duck-typed stand-in such as a test double).
pub trait CairoLike {
    fn save(&self);
    fn restore(&self);
    fn set_line_width(&self, width: f64);
    fn set_dash(&self, on: f64, off: f64);
    fn set_line_cap_round(&self);
    fn set_line_join_round(&self);
    fn set_source_rgb(&self, r: f64, g: f64, b: f64);
    fn set_source_rgba(&self, r: f64, g: f64, b: f64, a: f64);
    fn move_to(&self, x: f64, y: f64);
    fn line_to(&self, x: f64, y: f64);
    fn arc(
        &self,
        x: f64,
        y: f64,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
    );
    fn arc_negative(
        &self,
        x: f64,
        y: f64,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
    );
    fn rectangle(&self, x: f64, y: f64, width: f64, height: f64);
    fn fill(&self);
    fn stroke(&self);
    fn stroke_preserve(&self);
    fn new_sub_path(&self);
    fn new_path(&self);
    /// Returns the current line width (cairo's `get_line_width`).
    fn get_line_width(&self) -> f64;
    fn translate(&self, x: f64, y: f64);
    fn rotate(&self, angle: f64);
}

/// Screen-space parameter of the horizontal/vertical markers along
/// the segment.
pub const MARKER_T: f64 = 0.2;

/// Half-length of the horizontal/vertical marker strokes.
pub(crate) const MARKER_SIZE: f64 = 8.0;

/// Screen-space offset of the horizontal/vertical markers from the
/// segment line.
pub(crate) const MARKER_OFFSET: f64 = 10.0;

/// Screen-space distance of the radius/diameter labels from the arc.
pub const LABEL_DIST_SCREEN: f64 = 20.0;

/// Screen-space distance of the equal-length symbol from the segment.
pub const EQUAL_OFFSET_SCREEN: f64 = 15.0;

/// Screen-space offset of the aspect-ratio icon from the junction.
pub const ICON_OFFSET: f64 = 18.0;

/// Half-size of the aspect-ratio icon bracket.
pub(crate) const ICON_SIZE: f64 = 16.0;

/// Screen-space offset of the tangent symbol from the tangent point.
pub(crate) const TANGENT_OFFSET: f64 = 15.0;

/// Screen-space radius of the tangent symbol circle.
pub(crate) const TANGENT_RADIUS: f64 = 6.0;

/// Label font height, in screen pixels (the reference uses a 12 px
/// absolute label font).
const LABEL_FONT_PX: f64 = 12.0;

/// Equality-symbol font height, in screen pixels.
const EQUAL_FONT_PX: f64 = 16.0;

/// Padding around a constraint label box, in screen pixels.
const LABEL_PAD: f64 = 4.0;

/// Points per inch used by the text stack.
const PT_TO_MM: f64 = 25.4 / 72.0;

/// Computes the screen-space scale from a `to_screen` mapping, using
/// the unit model vector.  Falls back to 1.0 for degenerate mappings.
pub fn screen_scale(to_screen: &dyn Fn((f64, f64)) -> (f64, f64)) -> f64 {
    let p0 = to_screen((0.0, 0.0));
    let p1 = to_screen((1.0, 0.0));
    let scale = (p1.0 - p0.0).hypot(p1.1 - p0.1);
    if scale < 1e-9 {
        1.0
    } else {
        scale
    }
}

/// Formats a value with one decimal place, matching the reference
/// label formatting.
pub fn format_value(value: f64) -> String {
    format!("{:.1}", value)
}

/// Builds the label font config for the given screen-pixel height at
/// the given screen scale.
fn label_config(font_px: f64, scale: f64) -> FontConfig {
    FontConfig::new("sans-serif", font_px / (scale * PT_TO_MM))
}

/// Returns the status-driven drawing color (hover yellow, conflicting
/// or error red, expression orange, valid green).
pub fn status_color(style: &DrawStyle) -> (f64, f64, f64) {
    match style.status {
        ConstraintStatus::Error | ConstraintStatus::Conflicting => {
            (1.0, 0.2, 0.2)
        }
        ConstraintStatus::ExpressionBased => (1.0, 0.6, 0.0),
        _ => {
            if style.is_hovered {
                (1.0, 0.8, 0.0)
            } else {
                (0.0, 0.6, 0.0)
            }
        }
    }
}

/// Sets the status-driven stroke color (hover yellow, conflicting or
/// error red, expression orange, valid green).
pub fn draw_status_color(ctx: &dyn CairoLike, style: &DrawStyle) {
    let (r, g, b) = status_color(style);
    ctx.set_source_rgb(r, g, b);
}

/// Strokes the current path with a semi-transparent selection
/// highlight, mirroring the reference underlay.
pub fn draw_selection_underlay(ctx: &dyn CairoLike) {
    ctx.save();
    ctx.set_source_rgba(0.2, 0.6, 1.0, 0.4);
    ctx.set_line_width(ctx.get_line_width() * 3.0);
    ctx.stroke_preserve();
    ctx.restore();
}

/// Strokes the current path with a semi-transparent conflict
/// highlight when the constraint is conflicting.
pub fn draw_conflict_underlay(ctx: &dyn CairoLike, style: &DrawStyle) {
    if style.status != ConstraintStatus::Conflicting {
        return;
    }
    ctx.save();
    ctx.set_source_rgba(1.0, 0.2, 0.2, 0.5);
    ctx.set_line_width(ctx.get_line_width() * 3.5);
    ctx.stroke_preserve();
    ctx.restore();
}

/// Returns the label metrics `(width, ascent, descent)` in screen
/// pixels for the given text and font height.
fn label_metrics(text: &str, font_px: f64, scale: f64) -> (f64, f64, f64) {
    let config = label_config(font_px, scale);
    let fallback = |rect: crate::geo::types::Rect| {
        (
            (rect.max.x - rect.min.x) * scale,
            rect.max.y * scale,
            -rect.min.y * scale,
        )
    };
    let Some(geo) = text_to_geometry(text, &config) else {
        return (0.0, 0.0, 0.0);
    };
    let width_mm = get_text_width(text, &config).unwrap_or(0.0);
    if let Some((ascent, descent, _)) = get_font_metrics(&config) {
        (width_mm * scale, ascent * scale, -descent * scale)
    } else {
        fallback(geo.rect())
    }
}

/// Strokes the glyph outlines of `text`, centered on the given
/// screen position, through the raygeo text stack.
fn draw_glyphs(
    ctx: &dyn CairoLike,
    text: &str,
    font_px: f64,
    cx: f64,
    cy: f64,
    scale: f64,
) {
    let config = label_config(font_px, scale);
    let Some(geo) = text_to_geometry(text, &config) else {
        return;
    };
    let (_, ascent_px, descent_px) = label_metrics(text, font_px, scale);
    let baseline_y = cy - (ascent_px - descent_px) / 2.0 + ascent_px;
    for command in geo.data() {
        match command {
            Command::Move { end } => {
                ctx.move_to(cx + end.x * scale, baseline_y - end.y * scale);
            }
            Command::Line { end } => {
                ctx.line_to(cx + end.x * scale, baseline_y - end.y * scale);
            }
            _ => {}
        }
    }
    ctx.stroke();
}

/// Draws a label: status-colored background box and centered text at
/// the given screen position, mirroring the reference label
/// rendering.
pub fn draw_label(
    ctx: &dyn CairoLike,
    text: &str,
    cx: f64,
    cy: f64,
    style: &DrawStyle,
    scale: f64,
) {
    let (_, ascent_px, descent_px) = label_metrics(text, LABEL_FONT_PX, scale);
    let width_px = label_metrics(text, LABEL_FONT_PX, scale).0;
    let height_px = ascent_px + descent_px;

    let (bg_r, bg_g, bg_b, bg_a) = if style.is_selected {
        (0.2, 0.6, 1.0, 0.4)
    } else if style.is_hovered {
        (1.0, 0.95, 0.85, 0.9)
    } else if style.status == ConstraintStatus::Conflicting {
        (1.0, 0.6, 0.6, 0.9)
    } else if style.status == ConstraintStatus::Error {
        (1.0, 0.8, 0.8, 0.9)
    } else if style.status == ConstraintStatus::ExpressionBased {
        (1.0, 0.9, 0.7, 0.9)
    } else {
        (1.0, 1.0, 1.0, 0.8)
    };
    ctx.set_source_rgba(bg_r, bg_g, bg_b, bg_a);
    ctx.rectangle(
        cx - width_px / 2.0 - LABEL_PAD,
        cy - height_px / 2.0 - LABEL_PAD,
        width_px + 2.0 * LABEL_PAD,
        height_px + 2.0 * LABEL_PAD,
    );
    ctx.fill();
    ctx.new_path();

    let (fg_r, fg_g, fg_b) = match style.status {
        ConstraintStatus::Error | ConstraintStatus::Conflicting => {
            (0.8, 0.0, 0.0)
        }
        _ => (0.0, 0.0, 0.5),
    };
    ctx.set_source_rgb(fg_r, fg_g, fg_b);
    draw_glyphs(ctx, text, LABEL_FONT_PX, cx, cy, scale);
}

/// Draws the equality symbol: selection/conflict disc fills and the
/// "=" glyphs at the given screen position.
pub(crate) fn draw_equal_symbol(
    ctx: &dyn CairoLike,
    sx: f64,
    sy: f64,
    style: &DrawStyle,
    scale: f64,
) {
    ctx.save();
    if style.is_selected {
        ctx.set_source_rgba(0.2, 0.6, 1.0, 0.4);
        ctx.arc(sx, sy, 10.0, 0.0, 2.0 * std::f64::consts::PI);
        ctx.fill();
    }
    if style.status == ConstraintStatus::Conflicting {
        ctx.set_source_rgba(1.0, 0.2, 0.2, 0.5);
        ctx.arc(sx, sy, 12.0, 0.0, 2.0 * std::f64::consts::PI);
        ctx.fill();
    }
    draw_status_color(ctx, style);
    draw_glyphs(ctx, "=", EQUAL_FONT_PX, sx, sy, scale);
    ctx.restore();
}

/// Draws the point marker circle shared by the coincident and
/// point-on-line/curve constraints.
pub(crate) fn draw_point_marker(
    ctx: &dyn CairoLike,
    sx: f64,
    sy: f64,
    style: &DrawStyle,
) {
    ctx.save();
    ctx.set_line_width(1.5);
    ctx.new_sub_path();
    ctx.arc(
        sx,
        sy,
        style.point_radius + 4.0,
        0.0,
        2.0 * std::f64::consts::PI,
    );
    if style.is_selected {
        draw_selection_underlay(ctx);
    }
    draw_conflict_underlay(ctx, style);
    draw_status_color(ctx, style);
    ctx.stroke();
    ctx.restore();
}

/// Draws the two symmetry arrow heads along the segment direction,
/// mirroring the reference helper's translate/rotate pairs.
pub(crate) fn draw_symmetry_arrows(
    ctx: &dyn CairoLike,
    s1: (f64, f64),
    s2: (f64, f64),
) {
    let mx = (s1.0 + s2.0) / 2.0;
    let my = (s1.1 + s2.1) / 2.0;
    let angle = (s2.1 - s1.1).atan2(s2.0 - s1.0);
    let offset = 12.0;
    let lx = mx - offset * angle.cos();
    let ly = my - offset * angle.sin();

    ctx.save();
    ctx.translate(lx, ly);
    ctx.rotate(angle);
    ctx.move_to(-3.0, -4.0);
    ctx.line_to(3.0, 0.0);
    ctx.line_to(-3.0, 4.0);
    ctx.restore();

    let rx = mx + offset * angle.cos();
    let ry = my + offset * angle.sin();
    ctx.save();
    ctx.translate(rx, ry);
    ctx.rotate(angle);
    ctx.move_to(3.0, -4.0);
    ctx.line_to(-3.0, 0.0);
    ctx.line_to(3.0, 4.0);
    ctx.restore();
}
