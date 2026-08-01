use crate::geo::algo::polylabel::find_largest_circle;
use crate::geo::shape::polygon::{
    get_polygon_area, get_polygon_centroid, get_polygon_signed_area,
    get_polygons_group_difference, is_point_in_polygon, offset_polygon,
    JoinStyle,
};
use crate::geo::types::{Point, Polygon};
use crate::ops::feature::narrow::{
    analyze_pocket, NarrowAnalysisOptions, NarrowRegion,
};
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

/// Build the narrow-analysis options used for region detection.
fn build_narrow_options(
    tool_radius: f64,
    tolerance: f64,
) -> NarrowAnalysisOptions {
    NarrowAnalysisOptions {
        tool_radius,
        tolerance,
        min_slot_width: 0.0,
    }
}

/// Dilate each passage by a small epsilon so adjacent passage patches
/// touch and form a continuous barrier, then chain them with the islands
/// into the full barrier set that is subtracted from the boundary.
///
/// Without the dilation, a passage that consists of several disjoint
/// convex-hull patches would leave thin connections between the wide lobes
/// it separates.
fn collect_barriers(
    narrow_regions: &[NarrowRegion],
    islands: &[Polygon],
) -> Vec<Polygon> {
    narrow_regions
        .iter()
        .flat_map(|r| offset_polygon(&r.polygon, 0.1, JoinStyle::Round))
        .chain(islands.iter().cloned())
        .collect()
}

/// Split a polygon-difference result into shells (solid pieces) and holes
/// (negative rings that `get_polygons_group_difference` carved out of the
/// boundary: islands and passages).
fn split_shells_holes(
    wide_polygons: &[Polygon],
) -> (Vec<Polygon>, Vec<Polygon>) {
    let shells: Vec<Polygon> = wide_polygons
        .iter()
        .filter(|p| p.len() >= 3 && get_polygon_signed_area(p) > 0.0)
        .cloned()
        .collect();
    let holes: Vec<Polygon> = wide_polygons
        .iter()
        .filter(|p| p.len() >= 3 && get_polygon_signed_area(p) <= 0.0)
        .cloned()
        .collect();
    (shells, holes)
}

/// Collect the holes that belong to a shell: any hole whose first vertex
/// lies inside the shell.
fn associate_holes(shell: &Polygon, holes: &[Polygon]) -> Vec<Polygon> {
    holes
        .iter()
        .filter(|h| {
            h.first()
                .map(|&p| is_point_in_polygon(p, shell))
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

/// Turn a shell (with its associated holes) into a `Region`, or `None` when
/// the shell cannot fit the tool.
fn shell_to_region(
    shell: &Polygon,
    associated_holes: &[Polygon],
    tool_radius: f64,
) -> Option<Region> {
    let area = get_polygon_area(shell);
    let (entry_pt, r_max) = find_largest_circle(shell, associated_holes, 0.1)
        .unwrap_or_else(|| (get_polygon_centroid(shell), 0.0));
    if r_max >= tool_radius {
        Some(Region {
            polygon: shell.clone(),
            area,
            entry_pt,
            r_max,
        })
    } else {
        None
    }
}

/// Detect disconnected wide sub-regions of a pocket.
///
/// Every narrow / slot / unreachable passage (`analyze_pocket`) acts as a
/// hard barrier that splits the pocket: the passages (dilated by a small
/// epsilon so adjacent patches join into continuous walls) and the islands
/// are subtracted from the boundary, and each resulting connected component
/// is one wide region.  This is what `adaptive_clearing` can seed and clear
/// independently — it must never have to navigate across a passage.
///
/// Results are sorted by area descending (largest region first).
/// Returns an empty `Vec` when no tool-reachable wide area remains.
#[prof]
pub fn find_regions(
    boundary: &Polygon,
    islands: &[Polygon],
    tool_radius: f64,
    tolerance: f64,
) -> Vec<Region> {
    let options = build_narrow_options(tool_radius, tolerance);

    let narrow_regions = match analyze_pocket(boundary, islands, &options) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    // Every barrier (passages + islands) is subtracted from the boundary.
    let barriers = collect_barriers(&narrow_regions, islands);
    let wide_polygons = get_polygons_group_difference(
        std::slice::from_ref(boundary),
        &barriers,
    );

    let (shells, holes) = split_shells_holes(&wide_polygons);

    let mut regions: Vec<Region> = Vec::new();
    for shell in shells {
        let associated_holes = associate_holes(&shell, &holes);
        if let Some(region) =
            shell_to_region(&shell, &associated_holes, tool_radius)
        {
            regions.push(region);
        }
    }

    regions.sort_by(|a, b| {
        b.area
            .partial_cmp(&a.area)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    regions
}
