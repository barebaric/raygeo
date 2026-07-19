//! The `Part` type — unified workpiece + accumulated state for motion assembly.
//!
//! A `Part` describes the item being worked on.  It carries the
//! geometry (vector outlines) and/or metadata needed by assemblers,
//! plus a [`ClearedArea`] tracking what has already been cut.
//! No machine parameters, no step metadata — just the workpiece data
//! and its accumulated machining state.
//!
//! Assemblers accept `&mut Part` and mutate `part.cleared` as they work.

use crate::constants::{EPSILON_BOUNDARY, EPSILON_MERGE};
use crate::geo::algo::fitting::linearize_data;
use crate::geo::algo::topology::{
    split_inner_and_outer_contours, split_into_contours,
};
use crate::geo::shape::polygon::clean_polygon;
use crate::geo::Geometry;
use crate::types::{Point, Polygon};

use super::cleared_area::ClearedArea;
use super::image_source::ImageSource;
use super::stock_region::StockRegion;

/// Unified workpiece description shared by all assemblers.
///
/// Carries geometry, physical metadata, and a [`ClearedArea`] that
/// accumulates the cleared fragments as assemblers work the part.
///
/// Not `Clone`: an [`ImageSource`] is an opaque trait object. Callers
/// that previously cloned a `Part` should construct a fresh one or
/// borrow `&mut Part` instead.
pub struct Part {
    /// Vector geometry — the outline(s) of the part.
    ///
    /// May contain multiple closed contours (including holes).
    pub geometry: Option<Geometry>,

    /// Boundary and islands — cached extraction from geometry.
    /// Computed once at construction; never changes.
    pub stock_region: StockRegion,

    /// Physical size of the part in millimetres `(width, height)`.
    pub size_mm: (f64, f64),

    /// Pixel density `(x, y)` in pixels per millimetre.
    ///
    /// Required for raster operations; `None` for purely vector work.
    pub pixels_per_mm: Option<(f64, f64)>,

    /// Accumulated cleared-area state — what has been cut so far.
    ///
    /// Initialized from the part's boundary/islands at construction
    /// time; assemblers mutate this as they work.
    pub cleared: ClearedArea,

    /// Optional lazy source of pixel data for raster/shrinkwrap
    /// assemblers.
    ///
    /// Set by the stage before calling an assembler. The assembler
    /// pulls rows via [`ImageSource::read_slab`] (or
    /// [`ImageSource::read_all`] for full-buffer passes) instead of
    /// reading a separate image argument. `None` for vector-only
    /// work.
    pub image_source: Option<Box<dyn ImageSource>>,
}

impl std::fmt::Debug for Part {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let img = match &self.image_source {
            None => "None",
            Some(_) => "<ImageSource>",
        };
        f.debug_struct("Part")
            .field("geometry", &self.geometry)
            .field("stock_region", &self.stock_region)
            .field("size_mm", &self.size_mm)
            .field("pixels_per_mm", &self.pixels_per_mm)
            .field("cleared", &self.cleared)
            .field("image_source", &img)
            .finish()
    }
}

impl Part {
    /// Create a new `Part` from geometry and size.
    ///
    /// The `StockRegion` is extracted from `geometry` (empty if `None`).
    /// The `ClearedArea` starts empty.
    pub fn new(geometry: Option<Geometry>, size_mm: (f64, f64)) -> Self {
        let stock_region = match &geometry {
            Some(_) => {
                let (boundary, islands) =
                    Part::extract_boundary_from_geometry(geometry.as_ref());
                StockRegion::new(boundary.unwrap_or_default(), islands)
            }
            None => StockRegion::empty(),
        };
        Part {
            geometry,
            stock_region,
            size_mm,
            pixels_per_mm: None,
            cleared: ClearedArea::new(),
            image_source: None,
        }
    }

    /// Build a `Part` from a boundary polygon and optional islands.
    ///
    /// Constructs a `Geometry` containing the boundary as the first
    /// closed contour and each island as an additional contour, then
    /// wraps it in a `Part` with the given `size_mm`.
    /// The `StockRegion` is set directly from `boundary`/`islands`.
    pub fn from_polygons(
        boundary: &Polygon,
        islands: &[Polygon],
        size_mm: (f64, f64),
    ) -> Self {
        let mut geo = Geometry::new();
        if let Some(first) = boundary.first() {
            geo.move_to(first.x, first.y, 0.0);
            for p in boundary.iter().skip(1) {
                geo.line_to(p.x, p.y, 0.0);
            }
            geo.close_path();
        }
        for island in islands {
            if let Some(first) = island.first() {
                geo.move_to(first.x, first.y, 0.0);
                for p in island.iter().skip(1) {
                    geo.line_to(p.x, p.y, 0.0);
                }
                geo.close_path();
            }
        }
        let stock_region = StockRegion::new(boundary.clone(), islands.to_vec());
        Part {
            geometry: Some(geo),
            stock_region,
            size_mm,
            pixels_per_mm: None,
            cleared: ClearedArea::new(),
            image_source: None,
        }
    }

    /// Build a Part from polygons, pre-seeding the cleared area with
    /// `initial` fragments (e.g. a seed circle for adaptive clearing).
    pub fn from_polygons_initial(
        boundary: &Polygon,
        islands: &[Polygon],
        initial: &[Polygon],
        size_mm: (f64, f64),
    ) -> Self {
        let mut geo = Geometry::new();
        if let Some(first) = boundary.first() {
            geo.move_to(first.x, first.y, 0.0);
            for p in boundary.iter().skip(1) {
                geo.line_to(p.x, p.y, 0.0);
            }
            geo.close_path();
        }
        for island in islands {
            if let Some(first) = island.first() {
                geo.move_to(first.x, first.y, 0.0);
                for p in island.iter().skip(1) {
                    geo.line_to(p.x, p.y, 0.0);
                }
                geo.close_path();
            }
        }
        let stock_region = StockRegion::new(boundary.clone(), islands.to_vec());
        let cleared = ClearedArea::with_fragments(initial);
        Part {
            geometry: Some(geo),
            stock_region,
            size_mm,
            pixels_per_mm: None,
            cleared,
            image_source: None,
        }
    }

    /// Extract the outer boundary and island polygons from `self.geometry`.
    ///
    /// Returns `(boundary, islands)`.
    /// - `boundary` is the largest outer (CCW) contour, or `None`.
    /// - `islands` are all inner (CW) contours.
    pub fn extract_boundary(&self) -> (Option<Polygon>, Vec<Polygon>) {
        Self::extract_boundary_from_geometry(self.geometry.as_ref())
    }

    /// Replace `self.stock_region` with a new one built from the given
    /// boundary and islands.  Returns the old stock region so callers
    /// can restore it after an operation that temporarily scopes the region.
    pub fn replace_stock_region(
        &mut self,
        boundary: Polygon,
        islands: Vec<Polygon>,
    ) -> StockRegion {
        std::mem::replace(
            &mut self.stock_region,
            StockRegion::new(boundary, islands),
        )
    }

    /// Standalone boundary extraction from a geometry reference.
    fn extract_boundary_from_geometry(
        geo: Option<&Geometry>,
    ) -> (Option<Polygon>, Vec<Polygon>) {
        let geo = match geo {
            Some(g) => g,
            None => return (None, vec![]),
        };

        // Linearize once, then split.
        let mut linearized = geo.copy();
        if !linearized.data.is_empty() {
            linearized.data =
                linearize_data(&linearized.data, EPSILON_BOUNDARY);
        }
        let contours = split_into_contours(&linearized);
        if contours.is_empty() {
            return (None, vec![]);
        }

        let refs: Vec<&Geometry> = contours.iter().collect();
        let (inner_indices, outer_indices) =
            split_inner_and_outer_contours(&refs);

        // Convert indexed contours to polygons (without re-linearizing).
        let contour_to_poly = |i: usize| -> Option<Polygon> {
            let segs = contours[i].segments();
            for seg in &segs {
                if seg.len() < 3 {
                    continue;
                }
                let poly: Polygon =
                    seg.iter().map(|p| Point::new(p.x, p.y)).collect();
                if let Some(cleaned) =
                    clean_polygon(&poly, 0.01 * EPSILON_MERGE)
                {
                    return Some(cleaned);
                }
                if poly.len() >= 3 {
                    return Some(poly);
                }
            }
            None
        };

        let outers: Vec<Polygon> = outer_indices
            .iter()
            .filter_map(|&i| contour_to_poly(i))
            .collect();

        let islands: Vec<Polygon> = inner_indices
            .iter()
            .filter_map(|&i| contour_to_poly(i))
            .collect();

        // Pick the largest outer contour as the main boundary.
        let boundary = outers.into_iter().max_by(|a, b| {
            crate::utils::sort_f64(
                crate::geo::shape::polygon::get_polygon_area(a),
                crate::geo::shape::polygon::get_polygon_area(b),
            )
        });

        (boundary, islands)
    }
}
