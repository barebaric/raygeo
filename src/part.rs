//! The `Part` type — unified input for motion assembly.
//!
//! A `Part` describes the item being worked on.  It carries the
//! geometry (vector outlines) and/or metadata needed by assemblers.
//! No machine parameters, no step metadata — just the workpiece data.
//!
//! Assemblers accept `&Part` and internally extract what they need
//! (boundary polygons, islands, size, pixel density).

use crate::constants::EPSILON_MERGE;
use crate::geo::algo::fitting::linearize_data;
use crate::geo::algo::topology::{
    split_inner_and_outer_contours, split_into_contours,
};
use crate::geo::shape::polygon::clean_polygon;
use crate::geo::Geometry;
use crate::types::{Point, Polygon};

/// Unified workpiece description shared by all assemblers.
///
/// Every assembler that currently accepts raw polygon data
/// (`boundary`, `islands`, …) will grow an overload that accepts
/// `&Part` instead.  The assembler internally converts from `Part`
/// to whatever it needs.
#[derive(Clone, Debug)]
pub struct Part {
    /// Vector geometry — the outline(s) of the part.
    ///
    /// May contain multiple closed contours (including holes).
    pub geometry: Option<Geometry>,

    /// Physical size of the part in millimetres `(width, height)`.
    pub size_mm: (f64, f64),

    /// Pixel density `(x, y)` in pixels per millimetre.
    ///
    /// Required for raster operations; `None` for purely vector work.
    pub pixels_per_mm: Option<(f64, f64)>,
}

impl Part {
    /// Create a new `Part` from geometry and size.
    pub fn new(geometry: Option<Geometry>, size_mm: (f64, f64)) -> Self {
        Part {
            geometry,
            size_mm,
            pixels_per_mm: None,
        }
    }

    /// Build a `Part` from a boundary polygon and optional islands.
    ///
    /// Constructs a `Geometry` containing the boundary as the first
    /// closed contour and each island as an additional contour, then
    /// wraps it in a `Part` with the given `size_mm`.
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
        Part {
            geometry: Some(geo),
            size_mm,
            pixels_per_mm: None,
        }
    }

    /// Extract the outer boundary and island polygons from `self.geometry`.
    ///
    /// Returns `(boundary, islands)`.
    /// - `boundary` is the largest outer (CCW) contour, or `None`.
    /// - `islands` are all inner (CW) contours.
    pub fn extract_boundary(&self) -> (Option<Polygon>, Vec<Polygon>) {
        let geo = match &self.geometry {
            Some(g) => g,
            None => return (None, vec![]),
        };

        // Linearize once, then split.
        let mut linearized = geo.copy();
        if !linearized.data.is_empty() {
            linearized.data = linearize_data(&linearized.data, EPSILON_MERGE);
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
            crate::geo::shape::polygon::get_polygon_area(a)
                .partial_cmp(&crate::geo::shape::polygon::get_polygon_area(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        (boundary, islands)
    }
}
