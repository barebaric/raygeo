use crate::geo::shape::polygon::{
    get_polygon_bounds, get_polygon_group_bounds, translate_polygon,
};
use crate::types::{Polygon, Rect};

use super::collision::{any_overlap, is_contained};

const MIN_SLIDE: f64 = 0.01;
const MIN_STEP: f64 = 0.1;

/// Binary search for the maximum distance a part can slide in the negative
/// direction of `axis` without overlapping other parts or leaving the sheet.
pub fn find_max_slide(
    polys: &[Polygon],
    other_polys_list: &[Vec<Polygon>],
    sheet_bounds: Rect,
    sheet_poly: &Polygon,
    axis: &str,
    spacing: f64,
) -> f64 {
    let Rect(sheet_min_x, sheet_min_y, _, _) = sheet_bounds;

    let bounds = get_polygon_group_bounds(polys);
    let limit = if axis == "x" {
        sheet_min_x + spacing
    } else {
        sheet_min_y + spacing
    };
    let current_min = if axis == "x" { bounds.0 } else { bounds.1 };

    let max_slide = current_min - limit;
    if max_slide < MIN_SLIDE {
        return 0.0;
    }

    // Flatten other_polys_list for overlap checks
    let other_flat: Vec<Polygon> =
        other_polys_list.iter().flat_map(|g| g.clone()).collect();

    let mut best_slide = 0.0;
    let mut step = max_slide;

    while step > MIN_STEP {
        let test_slide = best_slide + step;
        if test_slide > max_slide {
            step /= 2.0;
            continue;
        }

        let test_polys: Vec<Polygon> = if axis == "x" {
            polys
                .iter()
                .map(|p| translate_polygon(p, -test_slide, 0.0))
                .collect()
        } else {
            polys
                .iter()
                .map(|p| translate_polygon(p, 0.0, -test_slide))
                .collect()
        };

        // Check overlap with other parts
        let overlaps = test_polys
            .iter()
            .any(|tp| any_overlap(tp, &other_flat, 1.0));

        if overlaps {
            step /= 2.0;
            continue;
        }

        // Check containment within sheet
        if !is_contained(&test_polys, sheet_poly) {
            step /= 2.0;
            continue;
        }

        best_slide = test_slide;
    }

    best_slide
}

/// Apply gravity sliding to a set of placements.
///
/// Iterates Y and X passes up to 10 times until no movement occurs.
/// Returns a `(dx, dy)` adjustment for each input group in order.
pub fn apply_gravity(
    placement_groups: &[Vec<Polygon>],
    sheet_poly: &Polygon,
    spacing: f64,
) -> Vec<(f64, f64)> {
    let n = placement_groups.len();
    if n < 2 {
        return vec![(0.0, 0.0); n];
    }

    let sheet_bounds = get_polygon_bounds(sheet_poly);
    let mut groups: Vec<Vec<Polygon>> = placement_groups.to_vec();
    let mut adjustments: Vec<(f64, f64)> = vec![(0.0, 0.0); n];

    for _ in 0..10 {
        let mut any_moved = false;

        // Sort by Y (min Y first) — slide down
        let mut y_order: Vec<usize> = (0..n).collect();
        y_order.sort_by(|&a, &b| {
            let ba = get_polygon_group_bounds(&groups[a]);
            let bb = get_polygon_group_bounds(&groups[b]);
            ba.1.partial_cmp(&bb.1).unwrap_or(std::cmp::Ordering::Equal)
        });

        for &idx in &y_order {
            let others: Vec<Vec<Polygon>> = groups
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != idx)
                .map(|(_, g)| g.clone())
                .collect();

            let dy = find_max_slide(
                &groups[idx],
                &others,
                sheet_bounds,
                sheet_poly,
                "y",
                spacing,
            );
            if dy > MIN_SLIDE {
                groups[idx] = groups[idx]
                    .iter()
                    .map(|p| translate_polygon(p, 0.0, -dy))
                    .collect();
                adjustments[idx].1 -= dy;
                any_moved = true;
            }
        }

        // Sort by X (min X first) — slide left
        let mut x_order: Vec<usize> = (0..n).collect();
        x_order.sort_by(|&a, &b| {
            let ba = get_polygon_group_bounds(&groups[a]);
            let bb = get_polygon_group_bounds(&groups[b]);
            ba.0.partial_cmp(&bb.0).unwrap_or(std::cmp::Ordering::Equal)
        });

        for &idx in &x_order {
            let others: Vec<Vec<Polygon>> = groups
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != idx)
                .map(|(_, g)| g.clone())
                .collect();

            let dx = find_max_slide(
                &groups[idx],
                &others,
                sheet_bounds,
                sheet_poly,
                "x",
                spacing,
            );
            if dx > MIN_SLIDE {
                groups[idx] = groups[idx]
                    .iter()
                    .map(|p| translate_polygon(p, -dx, 0.0))
                    .collect();
                adjustments[idx].0 -= dx;
                any_moved = true;
            }
        }

        if !any_moved {
            break;
        }
    }

    adjustments
}
