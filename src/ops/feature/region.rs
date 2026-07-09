use crate::geo::algo::polylabel::find_largest_circle;
use crate::geo::shape::polygon::{
    get_polygon_area, get_polygon_centroid, get_polygons_group_difference,
};
use crate::ops::feature::narrow::{analyze_pocket, NarrowAnalysisOptions};
use crate::types::{Point, Polygon};
use prof_macros::prof;

/// A disconnected wide sub-region of a pocket.
///
/// Wide sub-regions are the areas of the pocket that are wide enough for
/// the tool — they are separated by narrow/slot/unreachable passages.
#[derive(Clone, Debug)]
pub struct Region {
    /// Polygon of the wide sub-region.
    pub polygon: Polygon,
    /// Area of the sub-region in mm².
    pub area: f64,
    /// Entry point (centre of the largest inscribed circle).
    pub entry_pt: Point,
    /// Radius of the largest inscribed circle in mm.
    pub r_max: f64,
}

/// Detect disconnected wide sub-regions of a pocket.
///
/// The pocket boundary (with optional islands) is analyzed for narrow
/// passages.  The passages act as barriers that separate the pocket into
/// disconnected wide sub-regions.  Each sub-region is returned with its
/// largest inscribed circle (entry point and radius).
///
/// Results are sorted by area descending (largest region first).
/// Returns an empty `Vec` when the pocket is entirely consumed by passages.
#[prof]
pub fn find_regions(
    boundary: &Polygon,
    islands: &[Polygon],
    tool_radius: f64,
    tolerance: f64,
) -> Vec<Region> {
    let options = NarrowAnalysisOptions {
        tool_radius,
        tolerance,
        min_slot_width: 0.0,
    };

    let narrow_regions = match analyze_pocket(boundary, islands, &options) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let narrow_polygons: Vec<Polygon> =
        narrow_regions.into_iter().map(|r| r.polygon).collect();

    let wide_polygons = get_polygons_group_difference(
        std::slice::from_ref(boundary),
        &narrow_polygons,
    );

    let mut regions: Vec<Region> = wide_polygons
        .into_iter()
        .map(|polygon| {
            let area = get_polygon_area(&polygon);
            let (entry_pt, r_max) = find_largest_circle(&polygon, islands, 0.1)
                .unwrap_or_else(|| (get_polygon_centroid(&polygon), 0.0));
            Region {
                polygon,
                area,
                entry_pt,
                r_max,
            }
        })
        .filter(|r| r.r_max >= tool_radius)
        .collect();

    regions.sort_by(|a, b| {
        b.area
            .partial_cmp(&a.area)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    regions
}
