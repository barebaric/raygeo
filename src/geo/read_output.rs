//! ReadOutput: the geometry a file reader extracts from a document.
//!
//! Every reader in this crate (SVG, G-code, DXF) answers the same
//! question: which outlines does this file draw, which layer is each
//! outline on, how large is it, and what did the reader have to skip
//! while finding out. `ReadOutput` is that answer.
//!
//! It deliberately contains only facts about the file. Layout decisions
//! (where the document lands in the work area, how it is scaled to fit)
//! belong to the embedding application, which receives these fields and
//! applies its own normalisation.

use crate::geo::geometry::Geometry;
use crate::geo::types::Rect;

/// The geometry and metadata a reader extracted from one document.
#[derive(Clone, Debug, Default)]
pub struct ReadOutput {
    /// One geometry per layer, in document order.
    ///
    /// Readers that do not support layers report a single entry whose
    /// name is the format's default layer. The empty string is used for
    /// unnamed or single-layer content.
    pub layers: Vec<(String, Geometry)>,
    /// Bounding rectangle of all drawn content, in native document units.
    ///
    /// A document with no drawable content reports a zero-sized rect.
    pub bounds: Rect,
    /// Multiplier converting native document units to millimetres.
    ///
    /// Readers whose format is always metric report `1.0`.
    pub units_to_mm: f64,
    /// Whether the native Y axis points down (SVG, images) rather than up
    /// (DXF, G-code).
    pub is_y_down: bool,
    /// Range of Z values spanned by the drawn content, in native units.
    ///
    /// `(0.0, 0.0)` for planar documents. Values outside this range may
    /// come from entities the reader skipped.
    pub z_range: (f64, f64),
    /// Human-readable notes about content that was skipped or approximated.
    ///
    /// Readers never fail on a single unreadable entity: they record an
    /// issue and carry on, so a mostly-good file still imports.
    pub issues: Vec<String>,
}

impl ReadOutput {
    /// Creates an empty output for a reader that found nothing to draw.
    pub fn empty(units_to_mm: f64, is_y_down: bool) -> Self {
        ReadOutput {
            layers: Vec::new(),
            bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            units_to_mm,
            is_y_down,
            z_range: (0.0, 0.0),
            issues: Vec::new(),
        }
    }

    /// True when the document contains no drawn content at all.
    pub fn is_empty(&self) -> bool {
        self.layers.iter().all(|(_, geo)| geo.is_empty())
    }

    /// True when the content spans more than one Z level.
    ///
    /// `tolerance` is in native units; drawings carry small rounding
    /// noise, so an exact zero test would report variation on flat files.
    pub fn has_z_variation(&self, tolerance: f64) -> bool {
        let (min_z, max_z) = self.z_range;
        (max_z - min_z) > tolerance
    }

    /// The bounds of a single layer, or `None` when it draws nothing.
    pub fn layer_bounds(&self, index: usize) -> Option<Rect> {
        let (_, geo) = self.layers.get(index)?;
        if geo.is_empty() {
            return None;
        }
        Some(geo.rect())
    }
}
