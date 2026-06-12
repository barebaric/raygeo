use crate::geo::shape::polygon::{
    is_point_in_polygon, rotate_polygon, translate_polygon,
};
use crate::types::Polygon;

use super::collision::{any_overlap, any_overlap_with_grid};
use super::spatial_grid::SpatialGrid;

/// A single placed part result.
#[derive(Clone, Debug)]
pub struct PlacedPart {
    pub part_index: usize,
    pub rotation_index: usize,
    pub position: (f64, f64),
    pub polygons: Vec<Polygon>,
}

/// Result of nesting parts into a sheet.
#[derive(Clone, Debug)]
pub struct NestResult {
    pub placements: Vec<PlacedPart>,
    pub sheet_index: usize,
    pub unused_part_indices: Vec<usize>,
}

/// Configuration for placement search.
#[derive(Clone, Debug)]
pub struct PlacementConfig {
    pub spacing: f64,
    pub scale: i64,
    pub min_area: f64,
}

impl Default for PlacementConfig {
    fn default() -> Self {
        PlacementConfig {
            spacing: 1.0,
            scale: 10_000_000,
            min_area: 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Candidate generation
// ---------------------------------------------------------------------------

/// Generate edge-aligned placement candidates around already-placed parts.
///
/// For each placed part (group of polygons) produces 8 positions:
/// right-bottom, right-top, left-bottom, left-top, above-left, above-right,
/// below-left, below-right — placing the new part adjacent without overlap.
pub fn generate_perimeter_candidates(
    placed_groups: &[Vec<Polygon>],
    part_bounds: (f64, f64, f64, f64),
    spacing: f64,
) -> Vec<(f64, f64)> {
    let (p_min_x, p_min_y, p_max_x, p_max_y) = part_bounds;
    let mut candidates = Vec::with_capacity(placed_groups.len() * 8);

    for group in placed_groups {
        if group.is_empty() {
            continue;
        }
        let b = crate::geo::shape::polygon::get_polygon_group_bounds(group);
        let (pb_min_x, pb_min_y, pb_max_x, pb_max_y) = b;

        candidates.push((pb_max_x + spacing - p_min_x, pb_min_y - p_min_y));
        candidates.push((pb_max_x + spacing - p_min_x, pb_max_y - p_max_y));
        candidates.push((pb_min_x - spacing - p_max_x, pb_min_y - p_min_y));
        candidates.push((pb_min_x - spacing - p_max_x, pb_max_y - p_max_y));
        candidates.push((pb_min_x - p_min_x, pb_max_y + spacing - p_min_y));
        candidates.push((pb_max_x - p_max_x, pb_max_y + spacing - p_min_y));
        candidates.push((pb_min_x - p_min_x, pb_min_y - spacing - p_max_y));
        candidates.push((pb_max_x - p_max_x, pb_min_y - spacing - p_max_y));
    }

    candidates
}

/// Generate candidate positions by sweeping bottom-to-top, left-to-right
/// within the IFP bounding box.
pub fn generate_bottom_left_candidates(
    ifp_bounds: (f64, f64, f64, f64),
    part_bounds: (f64, f64, f64, f64),
    spacing: f64,
) -> Vec<(f64, f64)> {
    let pw = part_bounds.2 - part_bounds.0;
    let ph = part_bounds.3 - part_bounds.1;
    let sx = (pw + spacing).max(spacing);
    let sy = (ph + spacing).max(spacing);
    let mut cand = Vec::new();
    let mut y = ifp_bounds.1;
    while y + ph <= ifp_bounds.3 + 1e-6 {
        let mut x = ifp_bounds.0;
        while x + pw <= ifp_bounds.2 + 1e-6 {
            cand.push((x, y));
            x += sx;
        }
        y += sy;
    }
    cand
}

/// Generate a uniform grid of candidate positions within the IFP bounding box.
pub fn generate_grid_candidates(
    ifp_bounds: (f64, f64, f64, f64),
    _part_bounds: (f64, f64, f64, f64),
    spacing: f64,
) -> Vec<(f64, f64)> {
    let step = spacing.max(1.0);
    let mut cand = Vec::new();
    let mut y = ifp_bounds.1;
    while y <= ifp_bounds.3 + 1e-6 {
        let mut x = ifp_bounds.0;
        while x <= ifp_bounds.2 + 1e-6 {
            cand.push((x, y));
            x += step;
        }
        y += step;
    }
    cand
}

/// Remove candidates that are closer than `min_dist` to each other.
///
/// Uses a spatial-grid approach for O(n) average-case filtering.
pub fn filter_candidates_multi_resolution(
    candidates: &[(f64, f64)],
    _ifp_bounds: (f64, f64, f64, f64),
    min_dist: f64,
) -> Vec<(f64, f64)> {
    if candidates.is_empty() || min_dist <= 0.0 {
        return candidates.to_vec();
    }
    let mut grid: std::collections::HashMap<(i32, i32), (f64, f64)> =
        std::collections::HashMap::new();
    let mut result = Vec::new();
    for &(x, y) in candidates {
        let cx = (x / min_dist).floor() as i32;
        let cy = (y / min_dist).floor() as i32;
        let mut keep = true;
        'neighbors: for dx in -1..=1i32 {
            for dy in -1..=1i32 {
                if let Some(&(px, py)) = grid.get(&(cx + dx, cy + dy)) {
                    let d2 = (x - px).powi(2) + (y - py).powi(2);
                    if d2 < min_dist * min_dist {
                        keep = false;
                        break 'neighbors;
                    }
                }
            }
        }
        if keep {
            grid.insert((cx, cy), (x, y));
            result.push((x, y));
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Position search
// ---------------------------------------------------------------------------

/// Find the first valid placement position for a part inside the IFP.
///
/// Strategy: bottom-left candidates first, then grid, then perimeter
/// (if other parts have already been placed).
pub fn find_valid_position(
    ifp_polygons: &[Polygon],
    part_polygons: &[Polygon],
    placed_polygons: &[Polygon],
    config: &PlacementConfig,
    spacing: f64,
) -> Option<(f64, f64)> {
    if ifp_polygons.is_empty() || part_polygons.is_empty() {
        return None;
    }
    let part_bounds =
        crate::geo::shape::polygon::get_polygon_group_bounds(part_polygons);
    let ifp_bounds =
        crate::geo::shape::polygon::get_polygon_group_bounds(ifp_polygons);

    let mut candidates =
        generate_bottom_left_candidates(ifp_bounds, part_bounds, spacing);
    if candidates.is_empty() {
        candidates = generate_grid_candidates(ifp_bounds, part_bounds, spacing);
    }
    if !placed_polygons.is_empty() {
        let perim_groups: Vec<Vec<Polygon>> =
            placed_polygons.iter().map(|p| vec![p.clone()]).collect();
        candidates.extend(generate_perimeter_candidates(
            &perim_groups,
            part_bounds,
            spacing,
        ));
    }
    candidates = filter_candidates_multi_resolution(
        &candidates,
        ifp_bounds,
        spacing * 0.5,
    );

    for &(x, y) in &candidates {
        let in_ifp = ifp_polygons
            .iter()
            .any(|ifp| is_point_in_polygon((x, y), ifp));
        if !in_ifp {
            continue;
        }
        let dx = x - part_bounds.0;
        let dy = y - part_bounds.1;
        let translated: Vec<Polygon> = part_polygons
            .iter()
            .map(|p| translate_polygon(p, dx, dy))
            .collect();
        if translated
            .iter()
            .any(|tp| any_overlap(tp, placed_polygons, config.min_area))
        {
            continue;
        }
        return Some((x, y));
    }
    None
}

/// Fast variant of [`find_valid_position`] that uses a [`SpatialGrid`]
/// to accelerate overlap checks against many placed parts.
pub fn find_valid_position_fast(
    ifp_polygons: &[Polygon],
    part_polygons: &[Polygon],
    placed_polygons: &[Polygon],
    grid: &SpatialGrid,
    config: &PlacementConfig,
    spacing: f64,
) -> Option<(f64, f64)> {
    if ifp_polygons.is_empty() || part_polygons.is_empty() {
        return None;
    }
    let part_bounds =
        crate::geo::shape::polygon::get_polygon_group_bounds(part_polygons);
    let ifp_bounds =
        crate::geo::shape::polygon::get_polygon_group_bounds(ifp_polygons);

    let mut candidates =
        generate_bottom_left_candidates(ifp_bounds, part_bounds, spacing);
    if candidates.is_empty() {
        candidates = generate_grid_candidates(ifp_bounds, part_bounds, spacing);
    }
    if !placed_polygons.is_empty() {
        let perim_groups: Vec<Vec<Polygon>> =
            placed_polygons.iter().map(|p| vec![p.clone()]).collect();
        candidates.extend(generate_perimeter_candidates(
            &perim_groups,
            part_bounds,
            spacing,
        ));
    }
    candidates = filter_candidates_multi_resolution(
        &candidates,
        ifp_bounds,
        spacing * 0.5,
    );

    for &(x, y) in &candidates {
        let in_ifp = ifp_polygons
            .iter()
            .any(|ifp| is_point_in_polygon((x, y), ifp));
        if !in_ifp {
            continue;
        }
        let dx = x - part_bounds.0;
        let dy = y - part_bounds.1;
        let translated: Vec<Polygon> = part_polygons
            .iter()
            .map(|p| translate_polygon(p, dx, dy))
            .collect();
        if translated.iter().any(|tp| {
            any_overlap_with_grid(tp, placed_polygons, grid, config.min_area)
        }) {
            continue;
        }
        return Some((x, y));
    }
    None
}

// ---------------------------------------------------------------------------
// High-level orchestration
// ---------------------------------------------------------------------------

/// Place as many parts as possible onto sheets.
///
/// * `parts` — each item is a vector of polygons representing one part
///   (multiple polygons for parts with holes).
/// * `sheets` — each item is a bin polygon.
/// * `rotations` — rotation angles (degrees) to try for each part.
/// * `config` — placement parameters.
///
/// Returns one [`NestResult`] per sheet.
pub fn place_parts(
    parts: &[Vec<Polygon>],
    sheets: &[Polygon],
    rotations: &[f64],
    config: &PlacementConfig,
) -> Vec<NestResult> {
    let mut results: Vec<NestResult> = Vec::new();
    let mut remaining: Vec<usize> = (0..parts.len()).collect();

    for (si, sheet) in sheets.iter().enumerate() {
        if sheet.len() < 3 || remaining.is_empty() {
            results.push(NestResult {
                placements: vec![],
                sheet_index: si,
                unused_part_indices: remaining.clone(),
            });
            continue;
        }

        let mut placements: Vec<PlacedPart> = Vec::new();
        let mut placed_polygons: Vec<Polygon> = Vec::new();

        let mut i = 0;
        while i < remaining.len() {
            let pi = remaining[i];
            let part_polys = &parts[pi];

            let mut best_pos: Option<(usize, (f64, f64), Vec<Polygon>)> = None;

            for (ri, &angle) in rotations.iter().enumerate() {
                let rotated: Vec<Polygon> = part_polys
                    .iter()
                    .map(|p| rotate_polygon(p, angle))
                    .collect();

                // Inner-fit polygon via the ifp module (outer boundary only)
                let ifp_polys = if let Some(outer) = rotated.first() {
                    crate::nest::ifp::inner_fit_polygon(
                        sheet,
                        outer,
                        config.scale,
                    )
                } else {
                    continue;
                };
                if ifp_polys.is_empty() {
                    continue;
                }

                if let Some(pos) = find_valid_position(
                    &ifp_polys,
                    &rotated,
                    &placed_polygons,
                    config,
                    config.spacing,
                ) {
                    best_pos = Some((ri, pos, rotated));
                    break;
                }
            }

            if let Some((ri, pos, placed_part_polys)) = best_pos {
                let dx = pos.0;
                let dy = pos.1;
                let part_bounds =
                    crate::geo::shape::polygon::get_polygon_group_bounds(
                        &placed_part_polys,
                    );
                let offset_x = dx - part_bounds.0;
                let offset_y = dy - part_bounds.1;

                let placed: Vec<Polygon> = placed_part_polys
                    .iter()
                    .map(|p| translate_polygon(p, offset_x, offset_y))
                    .collect();

                placements.push(PlacedPart {
                    part_index: pi,
                    rotation_index: ri,
                    position: (dx, dy),
                    polygons: placed.clone(),
                });
                placed_polygons.extend(placed);
                remaining.remove(i);
            } else {
                i += 1;
            }
        }

        results.push(NestResult {
            placements,
            sheet_index: si,
            unused_part_indices: remaining.clone(),
        });

        if remaining.is_empty() {
            break;
        }
    }

    results
}
