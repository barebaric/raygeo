//! Text-to-geometry: convert text strings to vector paths.
//!
//! This module wraps the [`swash`] font shaper to rasterise glyph outlines
//! into the project's [`Geometry`] type.
//!
//! # Coordinate system
//!
//! The origin `(0, 0)` sits at the **baseline start** of the text.
//! **Y increases upward** (positive → ascender region, negative →
//! descender region).  This matches standard typographic convention.
//!
//! All measurements are returned in **millimetres** (1 pt = 25.4 / 72 mm).
//! System fonts are discovered via [`fontdb`]; bold/italic selection uses
//! the OS font database where available.
//!
//! # Example
//!
//! ```rust
//! use raygeo::geo::shape::text::{FontConfig, text_to_geometry};
//!
//! let cfg = FontConfig::new("sans-serif", 12.0);
//! if let Some(geo) = text_to_geometry("Hello", &cfg) {
//!     println!("Geometry has {} commands", geo.len());
//! }
//! ```

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use swash::scale::ScaleContext;
use swash::zeno;
use swash::zeno::PathData as _;
use swash::FontRef;

use crate::geo::geometry::Geometry;

/// Font configuration for text-to-geometry conversion.
///
/// Controls the font family, size (in points), and style (bold/italic).
/// Build with [`FontConfig::new`] and chain the builder methods:
///
/// ```
/// use raygeo::geo::shape::text::FontConfig;
/// let cfg = FontConfig::new("Arial", 14.0).bold(true).italic(false);
/// ```
#[derive(Clone, Debug)]
pub struct FontConfig {
    /// Font family name (e.g. "sans-serif", "Arial", "Noto Sans").
    pub family: String,
    /// Font size in points (1 point = 1/72 inch).
    pub size: f64,
    /// Whether to use a bold weight.
    pub bold: bool,
    /// Whether to use an italic/slanted style.
    pub italic: bool,
}

impl FontConfig {
    /// Create a new [`FontConfig`] with the given family and size.
    ///
    /// Bold and italic are disabled by default — use [`bold`](Self::bold) and
    /// [`italic`](Self::italic) to enable them.
    pub fn new(family: &str, size: f64) -> Self {
        FontConfig {
            family: family.to_string(),
            size,
            bold: false,
            italic: false,
        }
    }

    /// Enable or disable bold weight.
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    /// Enable or disable italic / slanted style.
    pub fn italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }
}

/// Global lazily-initialised font database.
///
/// Loads system fonts once and caches the result for the lifetime of
/// the process.  When DejaVu fonts are available they are used as the
/// preferred generic-family fallback; otherwise the database relies on
/// its own cross-platform defaults, ensuring correct behaviour on
/// operating systems (such as Windows) that do not ship DejaVu fonts.
fn font_database() -> &'static fontdb::Database {
    static DB: OnceLock<fontdb::Database> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();

        // Prefer DejaVu fonts when they are present, but do not override
        // generic-family resolution when they are absent – this keeps the
        // build reproducible on Linux/macOS while not breaking on Windows.
        if db
            .query(&fontdb::Query {
                families: &[fontdb::Family::Name("DejaVu Sans")],
                weight: fontdb::Weight::NORMAL,
                style: fontdb::Style::Normal,
                ..Default::default()
            })
            .is_some()
        {
            db.set_sans_serif_family("DejaVu Sans");
        }
        if db
            .query(&fontdb::Query {
                families: &[fontdb::Family::Name("DejaVu Serif")],
                weight: fontdb::Weight::NORMAL,
                style: fontdb::Style::Normal,
                ..Default::default()
            })
            .is_some()
        {
            db.set_serif_family("DejaVu Serif");
        }

        db
    })
}

/// Cached font data + face index keyed by fontdb face ID.
struct FontCacheEntry {
    data: Vec<u8>,
    index: u32,
}

/// Cache of raw font data + face index, keyed by face ID.
/// Avoids re-reading font files from disk on every call.
fn cached_font_data(id: fontdb::ID) -> Option<FontCacheEntry> {
    static CACHE: OnceLock<Mutex<HashMap<fontdb::ID, FontCacheEntry>>> =
        OnceLock::new();
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap();
    if let Some(entry) = cache.get(&id) {
        return Some(FontCacheEntry {
            data: entry.data.clone(),
            index: entry.index,
        });
    }
    let db = font_database();
    let result = db.with_face_data(id, |data, face_index| FontCacheEntry {
        data: data.to_vec(),
        index: face_index,
    });
    if let Some(entry) = result {
        cache.insert(
            id,
            FontCacheEntry {
                data: entry.data.clone(),
                index: entry.index,
            },
        );
        Some(entry)
    } else {
        None
    }
}

/// Look up a font face ID matching the given [`FontConfig`].
///
/// Tries the requested family name first, then falls back to generic
/// families (sans-serif, serif, monospace), and finally returns the
/// first available face as a last resort.
fn find_face_id(config: &FontConfig) -> Option<fontdb::ID> {
    let db = font_database();
    let weight = if config.bold {
        fontdb::Weight::BOLD
    } else {
        fontdb::Weight::NORMAL
    };
    let style = if config.italic {
        fontdb::Style::Italic
    } else {
        fontdb::Style::Normal
    };
    // First try the family name directly, then try generic fallback,
    // then fall back to any available font.
    let families: &[fontdb::Family] = &[
        fontdb::Family::Name(&config.family),
        fontdb::Family::SansSerif,
        fontdb::Family::Serif,
        fontdb::Family::Monospace,
    ];
    for family in families {
        let query = fontdb::Query {
            families: &[*family],
            weight,
            style,
            ..Default::default()
        };
        if let Some(id) = db.query(&query) {
            return Some(id);
        }
    }
    // Last resort: return the first available font face.
    db.faces().next().map(|info| info.id)
}

/// Shape a text string and return its glyph outlines as a [`Geometry`].
///
/// The origin `(0, 0)` of the returned geometry is the **baseline start**
/// of the text.  **Y increases upward** (positive → ascenders, negative →
/// descenders).  Each glyph's outline is decomposed into move-to,
/// line-to, quadratic/cubic Bézier commands that are appended to a
/// single [`Geometry`] in left-to-right order.
///
/// # Algorithm
///
/// 1. Look up the font face via [`find_face_id`].
/// 2. Build a [`swash::scale::Scaler`] at `size × internal_scale` ppem
///    (the internal scale improves contour fidelity before the final
///    down-scale).
/// 3. For each character, map via the font's charmapping table, skip
///    unmapped glyphs (still advancing their width), and scale the
///    outline through [`GeoPathBuilder`].
/// 4. Scale the whole geometry back by `1 / internal_scale`, then
///    convert from points to millimetres.
///
/// Returns `None` when the font face cannot be found.
pub fn text_to_geometry(text: &str, config: &FontConfig) -> Option<Geometry> {
    let face_id = find_face_id(config)?;
    let entry = cached_font_data(face_id)?;
    let font = FontRef::from_index(&entry.data, entry.index as usize)?;

    // Size in points -> pixels per em (72 DPI).
    let internal_scale = 4.0;
    let ppem = (config.size * internal_scale) as f32;

    let mut geo = Geometry::new();
    let mut scale_ctx = ScaleContext::new();
    let mut scaler = scale_ctx.builder(font).size(ppem).hint(false).build();

    let charmap = font.charmap();
    let font_metrics = font.metrics(&[]);
    let upem = font_metrics.units_per_em as f32;
    let glyph_metrics = font.glyph_metrics(&[]);

    // Pixel position of cursor, in the scaled pixel space (ppem).
    let mut cursor_px: f32 = 0.0;

    for ch in text.chars() {
        let gid: u16 = charmap.map(ch);
        if gid == 0 {
            // Unmapped glyph — still advance by its width.
            let advance_fu = glyph_metrics.advance_width(gid);
            cursor_px += advance_fu / upem * ppem;
            continue;
        }

        if let Some(outline) = scaler.scale_outline(gid) {
            let verbs = outline.verbs();
            let pts = outline.points();
            if !verbs.is_empty() && !pts.is_empty() {
                let mut builder = GeoPathBuilder::new(&mut geo, cursor_px, 0.0);
                (pts, verbs).copy_to(&mut builder);
            }
        }

        let advance_fu = glyph_metrics.advance_width(gid);
        cursor_px += advance_fu / upem * ppem;
    }

    // Scale from the internal resolution back to points.
    let inv = 1.0 / internal_scale;
    geo.transform_2d(inv, 0.0, 0.0, inv, 0.0, 0.0);

    // Convert from points to mm.
    let pt_to_mm = 25.4 / 72.0;
    geo.transform_2d(pt_to_mm, 0.0, 0.0, pt_to_mm, 0.0, 0.0);

    Some(geo)
}

/// Return the advance width of `text` in mm.
///
/// This is the logical layout width — it sums each glyph's advance
/// width and applies the same pt→mm transform as [`text_to_geometry`].
/// The result matches the X-extent that a text layout engine would
/// reserve, which may differ from the visual bounding box of the
/// generated geometry (side bearings, overhanging glyphs, etc.).
///
/// Returns `None` when the font face cannot be found.
pub fn get_text_width(text: &str, config: &FontConfig) -> Option<f64> {
    let face_id = find_face_id(config)?;
    let entry = cached_font_data(face_id)?;
    let font = FontRef::from_index(&entry.data, entry.index as usize)?;
    let font_metrics = font.metrics(&[]);
    let upem = font_metrics.units_per_em as f32;
    let glyph_metrics = font.glyph_metrics(&[]);
    let charmap = font.charmap();

    let pt_to_mm = 25.4 / 72.0;
    let mut total_mm: f32 = 0.0;
    for ch in text.chars() {
        let gid: u16 = charmap.map(ch);
        let advance_fu = glyph_metrics.advance_width(gid);
        total_mm += advance_fu / upem * config.size as f32 * pt_to_mm as f32;
    }
    Some(total_mm as f64)
}

/// Return font metrics as `(ascent, descent, height)` in mm.
///
/// Y follows the standard typographic convention: **positive is upward**
/// from the baseline.
///
/// - **ascent** — distance from the baseline to the top of ascenders (≥ 0)
/// - **descent** — distance from the baseline to the bottom of
///   descenders (always **≤ 0**, i.e. below the baseline)
/// - **height** — total vertical extent (`ascent - descent`)
///
/// Returns `None` when the font face cannot be found.
pub fn get_font_metrics(config: &FontConfig) -> Option<(f64, f64, f64)> {
    let face_id = find_face_id(config)?;
    let entry = cached_font_data(face_id)?;
    let font = FontRef::from_index(&entry.data, entry.index as usize)?;
    let m = font.metrics(&[]);
    let upem = m.units_per_em as f32;
    let pt_to_mm = 25.4 / 72.0;
    let scale = config.size as f32 / upem * pt_to_mm as f32;
    let ascent = (m.ascent * scale) as f64;
    let descent = -(m.descent * scale) as f64;
    let height = ascent - descent;
    Some((ascent, descent, height))
}

/// Return the X-position of the cursor at character `index` within `text` (mm).
///
/// X increases to the right along the baseline (positive → downstream).
/// `index = 0` is before the first character, `text.len()` is after the
/// last.  The result is the cumulative advance width of all characters
/// before `index`.
///
/// Returns `None` when the font face cannot be found.
pub fn get_text_position(
    text: &str,
    index: usize,
    config: &FontConfig,
) -> Option<f64> {
    if index == 0 {
        return Some(0.0);
    }
    let face_id = find_face_id(config)?;
    let entry = cached_font_data(face_id)?;
    let font = FontRef::from_index(&entry.data, entry.index as usize)?;
    let font_metrics = font.metrics(&[]);
    let upem = font_metrics.units_per_em as f32;
    let glyph_metrics = font.glyph_metrics(&[]);
    let charmap = font.charmap();

    let pt_to_mm = 25.4 / 72.0;
    let mut pos_mm: f32 = 0.0;
    for (i, ch) in text.chars().enumerate() {
        if i >= index {
            break;
        }
        let gid: u16 = charmap.map(ch);
        let advance_fu = glyph_metrics.advance_width(gid);
        pos_mm += advance_fu / upem * config.size as f32 * pt_to_mm as f32;
    }
    Some(pos_mm as f64)
}

/// Bridge between [`swash::zeno::PathBuilder`] and [`Geometry`].
///
/// Translates glyph outline commands (move, line, quad, cubic) into
/// the project's 3-D `Geometry` format, offsetting by `(origin_x, origin_y)`
/// so that each glyph sits at the correct cursor position.
struct GeoPathBuilder<'a> {
    geo: &'a mut Geometry,
    origin_x: f32,
    origin_y: f32,
    current_point: zeno::Point,
}

impl<'a> GeoPathBuilder<'a> {
    /// Create a new builder that writes into `geo`, offset by
    /// `(origin_x, origin_y)`.
    fn new(geo: &'a mut Geometry, origin_x: f32, origin_y: f32) -> Self {
        GeoPathBuilder {
            geo,
            origin_x,
            origin_y,
            current_point: zeno::Point::new(0.0, 0.0),
        }
    }
}

impl<'a> zeno::PathBuilder for GeoPathBuilder<'a> {
    /// Return the current pen position.
    fn current_point(&self) -> zeno::Point {
        self.current_point
    }

    /// Start a new sub-path at `to` (offset by origin).
    fn move_to(&mut self, to: impl Into<zeno::Point>) -> &mut Self {
        let p: zeno::Point = to.into();
        let x = p.x + self.origin_x;
        let y = p.y + self.origin_y;
        self.current_point = zeno::Point::new(x, y);
        self.geo.move_to(x as f64, y as f64, 0.0);
        self
    }

    /// Add a straight line from the current point to `to`.
    fn line_to(&mut self, to: impl Into<zeno::Point>) -> &mut Self {
        let p: zeno::Point = to.into();
        let x = p.x + self.origin_x;
        let y = p.y + self.origin_y;
        self.current_point = zeno::Point::new(x, y);
        self.geo.line_to(x as f64, y as f64, 0.0);
        self
    }

    /// Add a cubic Bézier curve from the current point to `to`
    /// with control points `c1` and `c2`.
    fn curve_to(
        &mut self,
        c1: impl Into<zeno::Point>,
        c2: impl Into<zeno::Point>,
        to: impl Into<zeno::Point>,
    ) -> &mut Self {
        let c1: zeno::Point = c1.into();
        let c2: zeno::Point = c2.into();
        let p: zeno::Point = to.into();
        let c1x = c1.x + self.origin_x;
        let c1y = c1.y + self.origin_y;
        let c2x = c2.x + self.origin_x;
        let c2y = c2.y + self.origin_y;
        let px = p.x + self.origin_x;
        let py = p.y + self.origin_y;
        self.current_point = zeno::Point::new(px, py);
        self.geo.bezier_to(
            crate::types::Point3D::new(c1x as f64, c1y as f64, 0.0),
            crate::types::Point3D::new(c2x as f64, c2y as f64, 0.0),
            crate::types::Point3D::new(px as f64, py as f64, 0.0),
        );
        self
    }

    /// Add a quadratic Bézier curve from the current point to `to`
    /// with control point `c`.
    ///
    /// The quadratic is elevated to a cubic before appending so the
    /// underlying [`Geometry`] only stores one curve type.
    fn quad_to(
        &mut self,
        c: impl Into<zeno::Point>,
        to: impl Into<zeno::Point>,
    ) -> &mut Self {
        let c: zeno::Point = c.into();
        let p: zeno::Point = to.into();
        let ctrl = zeno::Point::new(c.x + self.origin_x, c.y + self.origin_y);
        let end = zeno::Point::new(p.x + self.origin_x, p.y + self.origin_y);
        let c0x = (self.current_point.x + 2.0 * ctrl.x) / 3.0;
        let c0y = (self.current_point.y + 2.0 * ctrl.y) / 3.0;
        let c1x = (2.0 * ctrl.x + end.x) / 3.0;
        let c1y = (2.0 * ctrl.y + end.y) / 3.0;
        self.current_point = end;
        self.geo.bezier_to(
            crate::types::Point3D::new(c0x as f64, c0y as f64, 0.0),
            crate::types::Point3D::new(c1x as f64, c1y as f64, 0.0),
            crate::types::Point3D::new(end.x as f64, end.y as f64, 0.0),
        );
        self
    }

    /// Close the current sub-path with a line back to the most recent
    /// [`move_to`](Self::move_to) point.
    fn close(&mut self) -> &mut Self {
        self.geo.close_path();
        self
    }
}
