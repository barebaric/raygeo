use crate::geo::shape::polygon::{
    get_polygon_area, get_polygon_group_bounds, get_polygons_group_difference,
    get_polygons_group_intersection, is_point_in_polygon, offset_polygon,
    rotate_polygon, translate_polygon,
};
use crate::types::Polygon;

use super::collision::any_overlap_hierarchical_grid;
use super::gravity::apply_gravity;
use super::ifp::inner_fit_polygon;
use super::nfp::no_fit_polygon;
use super::spatial_grid::SpatialGrid;

/// A single placed part result.
#[derive(Clone, Debug)]
pub struct PlacedPart {
    pub part_index: usize,
    pub rotation_index: usize,
    pub position: (f64, f64),
    pub polygons: Vec<Polygon>,
    pub hulls: Vec<Polygon>,
}

/// Result of nesting parts into a sheet.
#[derive(Clone, Debug)]
pub struct NestResult {
    pub placements: Vec<PlacedPart>,
    pub sheet_index: usize,
    pub unused_part_indices: Vec<usize>,
    pub fitness: f64,
}

/// Configuration for placement search.
#[derive(Clone, Debug)]
pub struct PlacementConfig {
    pub spacing: f64,
    pub scale: i64,
    pub min_area: f64,
    pub curve_tolerance: f64,
}

impl Default for PlacementConfig {
    fn default() -> Self {
        PlacementConfig {
            spacing: 1.0,
            scale: 10_000_000,
            min_area: 1.0,
            curve_tolerance: 0.5,
        }
    }
}

/// Sheet descriptor for the high-level orchestrator.
#[derive(Clone, Debug)]
pub struct SheetDesc {
    pub polygon: Polygon,
    pub world_offset: (f64, f64),
}

/// A configurable part descriptor with hulls.
#[derive(Clone, Debug)]
pub struct PartDesc {
    pub polygons: Vec<Polygon>,
    pub hulls: Vec<Polygon>,
}

// ---------------------------------------------------------------------------
// Candidate generation
// ---------------------------------------------------------------------------

/// Generate edge-aligned placement candidates around already-placed parts.
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

/// Generate candidate positions by sweeping bottom-to-top, left-to-right.
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

/// Generate a uniform grid of candidate positions.
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
// Helpers
// ---------------------------------------------------------------------------

fn score_position(x: f64, y: f64) -> f64 {
    y * 100.0 + x
}

fn ifp_vertex_candidates(ifp_polygons: &[Polygon]) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for poly in ifp_polygons {
        out.extend(poly.iter().copied());
    }
    out
}

fn placed_vertex_candidates(
    placed_polys_list: &[Vec<Polygon>],
    part_bounds: (f64, f64, f64, f64),
) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for group in placed_polys_list {
        for poly in group {
            for &(vx, vy) in poly {
                out.push((vx - part_bounds.0, vy - part_bounds.1));
            }
        }
    }
    out
}

fn translate_polygons(polys: &[Polygon], dx: f64, dy: f64) -> Vec<Polygon> {
    polys.iter().map(|p| translate_polygon(p, dx, dy)).collect()
}

fn bbox_overlaps(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    !(a.0 > b.2 || a.2 < b.0 || a.1 > b.3 || a.3 < b.1)
}

fn filter_dist(
    candidates: &[(f64, f64)],
    ifp_bounds: (f64, f64, f64, f64),
    curve_tolerance: f64,
) -> Vec<(f64, f64)> {
    let min_dist = curve_tolerance.max(0.1) * 2.0;
    let mut result =
        filter_candidates_multi_resolution(candidates, ifp_bounds, min_dist);
    if result.is_empty() && !candidates.is_empty() {
        result = candidates.to_vec();
        result.sort_by(|a, b| {
            score_position(a.0, a.1)
                .partial_cmp(&score_position(b.0, b.1))
                .unwrap()
        });
    }
    result
}

fn get_nearby_parts(
    placed_polys_list: &[Vec<Polygon>],
    grid: &SpatialGrid,
    bbox: (f64, f64, f64, f64),
) -> Vec<Vec<Polygon>> {
    let indices = grid.query(bbox);
    indices
        .iter()
        .filter_map(|&i| placed_polys_list.get(i).cloned())
        .collect()
}

fn query_nearby_indices(
    placed_polys_list: &[Vec<Polygon>],
    grid: &SpatialGrid,
    bbox: (f64, f64, f64, f64),
) -> Vec<usize> {
    grid.query(bbox)
        .into_iter()
        .filter(|&i| i < placed_polys_list.len())
        .collect()
}

// ---------------------------------------------------------------------------
// Candidate evaluation (shared between scored and NFP search)
// ---------------------------------------------------------------------------

/// Evaluate candidates and return the best valid position.
///
/// Iterates through `candidates` (assumed sorted best-first), checks IFP
/// containment and overlap with placed parts (including hulls), and returns
/// the best scoring valid position.
#[allow(clippy::too_many_arguments)]
fn evaluate_candidates(
    candidates: &[(f64, f64)],
    ifp_world: &[Polygon],
    part_polygons: &[Polygon],
    part_hulls: &[Polygon],
    placed_polys_list: &[Vec<Polygon>],
    placed_hulls_list: &[Vec<Polygon>],
    grid: &SpatialGrid,
    part_bounds: (f64, f64, f64, f64),
) -> Option<(f64, f64)> {
    let mut best_score = f64::INFINITY;
    let mut best_pos: Option<(f64, f64)> = None;

    for &(x, y) in candidates {
        let score = score_position(x, y);
        if score >= best_score {
            continue;
        }

        let in_ifp =
            ifp_world.iter().any(|ifp| is_point_in_polygon((x, y), ifp));
        if !in_ifp {
            continue;
        }

        let dx = x - part_bounds.0;
        let dy = y - part_bounds.1;
        let test_polys = translate_polygons(part_polygons, dx, dy);
        let test_hulls = if !part_hulls.is_empty() {
            translate_polygons(part_hulls, dx, dy)
        } else {
            Vec::new()
        };

        let cand_bbox = (
            x + part_bounds.0,
            y + part_bounds.1,
            x + part_bounds.2,
            y + part_bounds.3,
        );

        if any_overlap_hierarchical_grid(
            &test_polys,
            &test_hulls,
            placed_polys_list,
            placed_hulls_list,
            grid,
            cand_bbox,
            10.0,
        ) {
            continue;
        }

        best_score = score;
        best_pos = Some((x, y));
    }

    best_pos
}

// ---------------------------------------------------------------------------
// Position search — scored heuristic
// ---------------------------------------------------------------------------

/// Build candidate list for the fast heuristic search.
fn build_fast_candidates(
    ifp_world: &[Polygon],
    part_bounds: (f64, f64, f64, f64),
    placed_polys_list: &[Vec<Polygon>],
    grid: &SpatialGrid,
    spacing: f64,
) -> Vec<(f64, f64)> {
    let ifp_bounds = get_polygon_group_bounds(ifp_world);
    let pw = part_bounds.2 - part_bounds.0;
    let ph = part_bounds.3 - part_bounds.1;

    let mut candidates = ifp_vertex_candidates(ifp_world);
    candidates.extend(generate_bottom_left_candidates(
        ifp_bounds,
        part_bounds,
        10.0,
    ));
    candidates.extend(generate_grid_candidates(ifp_bounds, part_bounds, 10.0));

    if !placed_polys_list.is_empty() {
        candidates
            .extend(placed_vertex_candidates(placed_polys_list, part_bounds));

        let query_bbox = (
            ifp_bounds.0 - pw - spacing,
            ifp_bounds.1 - ph - spacing,
            ifp_bounds.2 + pw + spacing,
            ifp_bounds.3 + ph + spacing,
        );
        let nearby = get_nearby_parts(placed_polys_list, grid, query_bbox);
        if !nearby.is_empty() {
            candidates.extend(generate_perimeter_candidates(
                &nearby,
                part_bounds,
                spacing,
            ));
        }
    }

    candidates
}

/// Find a valid position using heuristic candidate search.
#[allow(clippy::too_many_arguments)]
pub fn find_valid_position_scored(
    ifp_polygons: &[Polygon],
    part_polygons: &[Polygon],
    part_hulls: &[Polygon],
    placed_polys_list: &[Vec<Polygon>],
    placed_hulls_list: &[Vec<Polygon>],
    grid: &SpatialGrid,
    sheet_world_offset: (f64, f64),
    config: &PlacementConfig,
    spacing: f64,
) -> Option<(f64, f64)> {
    if ifp_polygons.is_empty() || part_polygons.is_empty() {
        return None;
    }

    let ifp_world = translate_polygons(
        ifp_polygons,
        sheet_world_offset.0,
        sheet_world_offset.1,
    );
    let ifp_bounds = get_polygon_group_bounds(&ifp_world);
    let part_bounds = get_polygon_group_bounds(part_polygons);

    let candidates = build_fast_candidates(
        &ifp_world,
        part_bounds,
        placed_polys_list,
        grid,
        spacing,
    );
    let filtered = filter_dist(&candidates, ifp_bounds, config.curve_tolerance);

    evaluate_candidates(
        &filtered,
        &ifp_world,
        part_polygons,
        part_hulls,
        placed_polys_list,
        placed_hulls_list,
        grid,
        part_bounds,
    )
}

// ---------------------------------------------------------------------------
// Position search — NFP fallback
// ---------------------------------------------------------------------------

/// Compute NFP clip polygons for a single placed part against this part.
fn compute_nfp_clips_for_placed(
    placed_polys: &[Polygon],
    part_polygons: &[Polygon],
    ifp_bounds: (f64, f64, f64, f64),
    spacing: f64,
    scale: i64,
) -> Vec<Polygon> {
    let mut clips = Vec::new();
    let _p_bounds = get_polygon_group_bounds(placed_polys);

    for placed_poly in placed_polys {
        let placed_bbox =
            get_polygon_group_bounds(std::slice::from_ref(placed_poly));
        for part_poly in part_polygons {
            let part_bbox =
                get_polygon_group_bounds(std::slice::from_ref(part_poly));
            let pw = part_bbox.2 - part_bbox.0;
            let ph = part_bbox.3 - part_bbox.1;

            let expanded = (
                placed_bbox.0 - pw - spacing,
                placed_bbox.1 - ph - spacing,
                placed_bbox.2 + pw + spacing,
                placed_bbox.3 + ph + spacing,
            );

            if !bbox_overlaps(expanded, ifp_bounds) {
                continue;
            }

            let nfps = no_fit_polygon(placed_poly, part_poly, scale);
            for nfp in nfps {
                let origin = part_poly.first().copied().unwrap_or((0.0, 0.0));
                let shifted: Polygon = nfp
                    .iter()
                    .map(|(px, py)| (px - origin.0, py - origin.1))
                    .collect();

                if spacing > 0.0 {
                    for e in offset_polygon(&shifted, spacing) {
                        if e.len() >= 3 {
                            clips.push(e);
                        }
                    }
                } else if shifted.len() >= 3 {
                    clips.push(shifted);
                }
            }
        }
    }
    clips
}

/// Compute valid placement regions by subtracting NFP clips from the IFP.
fn compute_valid_regions(
    ifp_world: &[Polygon],
    nfp_clips: &[Polygon],
) -> Vec<Polygon> {
    let ifp_points: Vec<(f64, f64)> =
        ifp_world.iter().flat_map(|p| p.iter().copied()).collect();

    if ifp_points.len() < 3 {
        return Vec::new();
    }

    if nfp_clips.is_empty() {
        return vec![ifp_points];
    }

    let result = get_polygons_group_difference(
        std::slice::from_ref(&ifp_points),
        nfp_clips,
    );
    if result.is_empty() {
        vec![ifp_points]
    } else {
        result
    }
}

/// Build candidate list for the NFP-based fallback search.
fn build_nfp_candidates(
    ifp_world: &[Polygon],
    part_polygons: &[Polygon],
    part_bounds: (f64, f64, f64, f64),
    placed_polys_list: &[Vec<Polygon>],
    grid: &SpatialGrid,
    config: &PlacementConfig,
    spacing: f64,
) -> Vec<(f64, f64)> {
    let ifp_bounds = get_polygon_group_bounds(ifp_world);
    let pw = part_bounds.2 - part_bounds.0;
    let ph = part_bounds.3 - part_bounds.1;

    let query_bbox = (
        ifp_bounds.0 - pw - spacing,
        ifp_bounds.1 - ph - spacing,
        ifp_bounds.2 + pw + spacing,
        ifp_bounds.3 + ph + spacing,
    );
    let nearby_indices =
        query_nearby_indices(placed_polys_list, grid, query_bbox);

    let mut candidates: Vec<(f64, f64)> = Vec::new();
    let mut nfp_clips: Vec<Polygon> = Vec::new();

    for &pi in &nearby_indices {
        let placed_polys = &placed_polys_list[pi];
        let p_bounds = get_polygon_group_bounds(placed_polys);

        // Bounding-box corners
        candidates.push((
            p_bounds.0 - part_bounds.2 - spacing,
            p_bounds.1 - part_bounds.3 - spacing,
        ));
        candidates.push((
            p_bounds.2 - part_bounds.0 + spacing,
            p_bounds.1 - part_bounds.3 - spacing,
        ));
        candidates.push((
            p_bounds.2 - part_bounds.0 + spacing,
            p_bounds.3 - part_bounds.1 + spacing,
        ));
        candidates.push((
            p_bounds.0 - part_bounds.2 - spacing,
            p_bounds.3 - part_bounds.1 + spacing,
        ));

        nfp_clips.extend(compute_nfp_clips_for_placed(
            placed_polys,
            part_polygons,
            ifp_bounds,
            spacing,
            config.scale,
        ));
    }

    // Subtract NFPs from IFP to get valid regions
    let valid_regions = compute_valid_regions(ifp_world, &nfp_clips);
    for region in &valid_regions {
        candidates.extend(region.iter().copied());
    }
    candidates.extend(ifp_vertex_candidates(ifp_world));

    candidates.sort_by(|a, b| {
        score_position(a.0, a.1)
            .partial_cmp(&score_position(b.0, b.1))
            .unwrap()
    });
    candidates
}

/// Find a valid position using NFP-based region subtraction.
#[allow(clippy::too_many_arguments)]
pub fn find_valid_position_nfp(
    ifp_polygons: &[Polygon],
    part_polygons: &[Polygon],
    part_hulls: &[Polygon],
    placed_polys_list: &[Vec<Polygon>],
    placed_hulls_list: &[Vec<Polygon>],
    grid: &SpatialGrid,
    sheet_world_offset: (f64, f64),
    config: &PlacementConfig,
    spacing: f64,
) -> Option<(f64, f64)> {
    if ifp_polygons.is_empty()
        || part_polygons.is_empty()
        || placed_polys_list.is_empty()
    {
        return None;
    }

    let ifp_world = translate_polygons(
        ifp_polygons,
        sheet_world_offset.0,
        sheet_world_offset.1,
    );
    let ifp_bounds = get_polygon_group_bounds(&ifp_world);
    let part_bounds = get_polygon_group_bounds(part_polygons);

    let candidates = build_nfp_candidates(
        &ifp_world,
        part_polygons,
        part_bounds,
        placed_polys_list,
        grid,
        config,
        spacing,
    );
    let filtered = filter_dist(&candidates, ifp_bounds, config.curve_tolerance);

    evaluate_candidates(
        &filtered,
        &ifp_world,
        part_polygons,
        part_hulls,
        placed_polys_list,
        placed_hulls_list,
        grid,
        part_bounds,
    )
}

/// Find a valid position: heuristic search first, NFP fallback second.
#[allow(clippy::too_many_arguments)]
pub fn find_valid_position(
    ifp_polygons: &[Polygon],
    part_polygons: &[Polygon],
    part_hulls: &[Polygon],
    placed_polys_list: &[Vec<Polygon>],
    placed_hulls_list: &[Vec<Polygon>],
    grid: &SpatialGrid,
    sheet_world_offset: (f64, f64),
    config: &PlacementConfig,
    spacing: f64,
) -> Option<(f64, f64)> {
    let fast = find_valid_position_scored(
        ifp_polygons,
        part_polygons,
        part_hulls,
        placed_polys_list,
        placed_hulls_list,
        grid,
        sheet_world_offset,
        config,
        spacing,
    );
    if fast.is_some() {
        return fast;
    }
    find_valid_position_nfp(
        ifp_polygons,
        part_polygons,
        part_hulls,
        placed_polys_list,
        placed_hulls_list,
        grid,
        sheet_world_offset,
        config,
        spacing,
    )
}

// ---------------------------------------------------------------------------
// Combined IFP
// ---------------------------------------------------------------------------

/// Compute the combined IFP for all polygons in a part using intersection.
pub fn get_combined_ifp(
    bin: &Polygon,
    part_polygons: &[Polygon],
    scale: i64,
) -> Vec<Polygon> {
    if part_polygons.is_empty() {
        return vec![];
    }
    let mut combined: Option<Vec<Polygon>> = None;
    for poly in part_polygons {
        let ifps = inner_fit_polygon(bin, poly, scale);
        if ifps.is_empty() {
            return vec![];
        }
        combined = Some(match combined {
            None => ifps,
            Some(prev) => get_polygons_group_intersection(&prev, &ifps),
        });
        if combined.as_ref().is_none_or(|r| r.is_empty()) {
            return vec![];
        }
    }
    combined.unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Part helpers
// ---------------------------------------------------------------------------

fn sort_part_indices_by_area(parts: &[PartDesc]) -> Vec<usize> {
    let mut areas: Vec<(f64, usize)> = parts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let area: f64 = p
                .polygons
                .iter()
                .map(|poly| get_polygon_area(poly).abs())
                .sum();
            (-area, i) // negate for descending sort
        })
        .collect();
    areas.sort_by(|a, b| {
        a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)
    });
    areas.into_iter().map(|(_, idx)| idx).collect()
}

struct PreparedPart {
    polygons: Vec<Polygon>,
    hulls: Vec<Polygon>,
    part_bounds: (f64, f64, f64, f64),
}

fn prepare_part(
    part: &PartDesc,
    rotation: f64,
    _flip_h: bool,
    _flip_v: bool,
) -> PreparedPart {
    let rotated = part
        .polygons
        .iter()
        .map(|p| rotate_polygon(p, rotation))
        .collect::<Vec<_>>();
    let rotated_hulls = part
        .hulls
        .iter()
        .map(|p| rotate_polygon(p, rotation))
        .collect::<Vec<_>>();

    // Flips are handled in Python — just use rotated directly
    let norm_bounds = get_polygon_group_bounds(&rotated);
    let ox = norm_bounds.0;
    let oy = norm_bounds.1;

    PreparedPart {
        polygons: translate_polygons(&rotated, -ox, -oy),
        hulls: translate_polygons(&rotated_hulls, -ox, -oy),
        part_bounds: (
            0.0,
            0.0,
            norm_bounds.2 - norm_bounds.0,
            norm_bounds.3 - norm_bounds.1,
        ),
    }
}

// ---------------------------------------------------------------------------
// Sheet state management
// ---------------------------------------------------------------------------

struct SheetState {
    placed_polys: Vec<Vec<Polygon>>,
    placed_hulls: Vec<Vec<Polygon>>,
    grid: SpatialGrid,
}

fn new_sheet_states(count: usize) -> Vec<SheetState> {
    (0..count)
        .map(|_| SheetState {
            placed_polys: Vec::new(),
            placed_hulls: Vec::new(),
            grid: SpatialGrid::new(50.0),
        })
        .collect()
}

fn insert_placement(
    state: &mut SheetState,
    polygons: Vec<Polygon>,
    hulls: Vec<Polygon>,
) {
    let index = state.placed_polys.len();
    state.placed_polys.push(polygons);
    state.placed_hulls.push(hulls);
    state
        .grid
        .insert(index, get_polygon_group_bounds(&state.placed_polys[index]));
}

// ---------------------------------------------------------------------------
// Gravity application
// ---------------------------------------------------------------------------

fn apply_sheet_gravity(
    all_placements: &mut [PlacedPart],
    sheet_indices: &[usize],
    sheet_poly: &Polygon,
    spacing: f64,
    scale: i64,
) {
    let count = sheet_indices.len();
    if count < 2 {
        return;
    }

    // Can't borrow all_placements mutably through indices while also
    // iterating, so clone the polygons for the gravity computation
    let groups: Vec<Vec<Polygon>> = sheet_indices
        .iter()
        .map(|&i| all_placements[i].polygons.clone())
        .collect();

    let adjustments = apply_gravity(&groups, sheet_poly, spacing, scale);

    for (&idx, &(dx, dy)) in sheet_indices.iter().zip(adjustments.iter()) {
        let p = &mut all_placements[idx];
        if dx.abs() > 0.01 {
            p.position.0 += dx;
            p.polygons = translate_polygons(&p.polygons, dx, 0.0);
            p.hulls = translate_polygons(&p.hulls, dx, 0.0);
        }
        if dy.abs() > 0.01 {
            p.position.1 += dy;
            p.polygons = translate_polygons(&p.polygons, 0.0, dy);
            p.hulls = translate_polygons(&p.hulls, 0.0, dy);
        }
    }
}

// ---------------------------------------------------------------------------
// Result building
// ---------------------------------------------------------------------------

fn build_per_sheet_results(
    all_placements: &[PlacedPart],
    placement_sheet_map: &[usize],
    num_sheets: usize,
    num_parts: usize,
    remaining: &[usize],
) -> Vec<NestResult> {
    let mut results = Vec::with_capacity(num_sheets);

    for si in 0..num_sheets {
        let sheet_placements: Vec<PlacedPart> = all_placements
            .iter()
            .enumerate()
            .filter(|(i, _)| placement_sheet_map[*i] == si)
            .map(|(_, p)| p.clone())
            .collect();

        let polygon_groups: Vec<Vec<Polygon>> = sheet_placements
            .iter()
            .map(|p| p.polygons.clone())
            .collect();
        let rot_list: Vec<f64> = vec![0.0; sheet_placements.len()];
        let sheet_indices: Vec<usize> = vec![si; sheet_placements.len()];

        let fitness = calculate_fitness(
            &polygon_groups,
            &rot_list,
            &sheet_indices,
            num_parts,
        );

        results.push(NestResult {
            placements: sheet_placements,
            sheet_index: si,
            unused_part_indices: remaining.to_vec(),
            fitness,
        });
    }

    results
}

// ---------------------------------------------------------------------------
// High-level orchestration
// ---------------------------------------------------------------------------

/// Place as many parts as possible onto sheets.
///
/// Supports combined IFP for multi-polygon parts, hull-based collision
/// detection, world-space offsets per sheet, gravity post-processing,
/// and fitness calculation.
pub fn place_parts(
    parts: &[PartDesc],
    sheets: &[SheetDesc],
    rotations: &[f64],
    config: &PlacementConfig,
    flips_h: &[bool],
    flips_v: &[bool],
) -> Vec<NestResult> {
    let num_parts = parts.len();
    if num_parts == 0 || sheets.is_empty() {
        return Vec::new();
    }

    let sorted_indices = sort_part_indices_by_area(parts);
    let mut remaining: Vec<usize> = (0..num_parts).collect();
    let mut sheet_states = new_sheet_states(sheets.len());
    let mut all_placements: Vec<PlacedPart> = Vec::new();
    let mut placement_sheet_map: Vec<usize> = Vec::new();

    // Place each part on the best sheet
    for &sorted_idx in &sorted_indices {
        if !remaining.contains(&sorted_idx) {
            continue;
        }

        let part = &parts[sorted_idx];
        if part.polygons.is_empty() {
            continue;
        }

        let flip_h = flips_h.get(sorted_idx).copied().unwrap_or(false);
        let flip_v = flips_v.get(sorted_idx).copied().unwrap_or(false);
        let rotation = rotations.get(sorted_idx).copied().unwrap_or(0.0);

        let prepared = prepare_part(part, rotation, flip_h, flip_v);

        if let Some((world_pos, si)) =
            find_best_sheet(&prepared, sheets, &sheet_states, config)
        {
            let placed_polys = translate_polygons(
                &prepared.polygons,
                world_pos.0,
                world_pos.1,
            );
            let placed_hulls =
                translate_polygons(&prepared.hulls, world_pos.0, world_pos.1);

            all_placements.push(PlacedPart {
                part_index: sorted_idx,
                rotation_index: 0,
                position: world_pos,
                polygons: placed_polys.clone(),
                hulls: placed_hulls.clone(),
            });
            placement_sheet_map.push(si);

            insert_placement(&mut sheet_states[si], placed_polys, placed_hulls);
            remaining.retain(|&x| x != sorted_idx);
        }
    }

    // Apply gravity per sheet
    for (si, sheet) in sheets.iter().enumerate() {
        let sheet_indices: Vec<usize> = placement_sheet_map
            .iter()
            .enumerate()
            .filter(|(_, &s)| s == si)
            .map(|(i, _)| i)
            .collect();

        if sheet_indices.len() < 2 {
            continue;
        }

        let sheet_world_poly = translate_polygon(
            &sheet.polygon,
            sheet.world_offset.0,
            sheet.world_offset.1,
        );

        apply_sheet_gravity(
            &mut all_placements,
            &sheet_indices,
            &sheet_world_poly,
            config.spacing,
            config.scale,
        );
    }

    build_per_sheet_results(
        &all_placements,
        &placement_sheet_map,
        sheets.len(),
        num_parts,
        &remaining,
    )
}

/// Find the best sheet and position for a prepared part.
fn find_best_sheet(
    prepared: &PreparedPart,
    sheets: &[SheetDesc],
    sheet_states: &[SheetState],
    config: &PlacementConfig,
) -> Option<((f64, f64), usize)> {
    let part_width = prepared.part_bounds.2 - prepared.part_bounds.0;
    let part_height = prepared.part_bounds.3 - prepared.part_bounds.1;

    let mut best_pos: Option<((f64, f64), usize)> = None;
    let mut best_score = f64::INFINITY;

    for (si, sheet) in sheets.iter().enumerate() {
        let sheet_bounds =
            get_polygon_group_bounds(std::slice::from_ref(&sheet.polygon));
        let sheet_width = sheet_bounds.2 - sheet_bounds.0;
        let sheet_height = sheet_bounds.3 - sheet_bounds.1;

        if part_width > sheet_width || part_height > sheet_height {
            continue;
        }

        let ifps =
            get_combined_ifp(&sheet.polygon, &prepared.polygons, config.scale);
        if ifps.is_empty() {
            continue;
        }

        let state = &sheet_states[si];

        for ifp in &ifps {
            let pos = find_valid_position(
                std::slice::from_ref(ifp),
                &prepared.polygons,
                &prepared.hulls,
                &state.placed_polys,
                &state.placed_hulls,
                &state.grid,
                sheet.world_offset,
                config,
                config.spacing,
            );

            if let Some(p) = pos {
                let rel_x = p.0 - sheet.world_offset.0;
                let rel_y = p.1 - sheet.world_offset.1;
                let score = score_position(rel_x, rel_y);
                if score < best_score {
                    best_score = score;
                    best_pos = Some((p, si));
                }
            }
        }
    }

    best_pos
}

// ---------------------------------------------------------------------------
// Fitness
// ---------------------------------------------------------------------------

/// Calculate fitness score for a set of placements.
///
/// Lower is better.  Returns `f64::INFINITY` if no placements or zero area.
pub fn calculate_fitness(
    polygon_groups: &[Vec<Polygon>],
    rotations: &[f64],
    sheet_indices: &[usize],
    num_parts: usize,
) -> f64 {
    if polygon_groups.is_empty() {
        return f64::INFINITY;
    }

    let total_part_area: f64 = polygon_groups
        .iter()
        .flat_map(|polys| polys.iter())
        .map(|poly| get_polygon_area(poly).abs())
        .sum();

    if total_part_area < 1e-9 {
        return f64::INFINITY;
    }

    let num_sheets = sheet_indices.iter().max().map(|m| m + 1).unwrap_or(0);
    let mut sheet_bounds: Vec<(f64, f64, f64, f64, f64, f64, usize)> = vec![
            (
                f64::INFINITY, f64::INFINITY,
                f64::NEG_INFINITY, f64::NEG_INFINITY,
                0.0, 0.0, 0,
            );
            num_sheets.max(1)
        ];

    for (i, polys) in polygon_groups.iter().enumerate() {
        let si = sheet_indices.get(i).copied().unwrap_or(0);
        if si >= sheet_bounds.len() {
            continue;
        }
        let b = get_polygon_group_bounds(polys);
        let entry = &mut sheet_bounds[si];
        entry.0 = entry.0.min(b.0);
        entry.1 = entry.1.min(b.1);
        entry.2 = entry.2.max(b.2);
        entry.3 = entry.3.max(b.3);
        entry.4 += b.0;
        entry.5 += b.1;
        entry.6 += 1;
    }

    let mut total_bounds_area = 0.0;
    let mut gravity_penalty = 0.0;
    let mut compaction_penalty = 0.0;

    for b in &sheet_bounds {
        let width = b.2 - b.0;
        let height = b.3 - b.1;
        total_bounds_area += width * height;
        compaction_penalty += width + height * 10.0;
        gravity_penalty += b.0 + b.1;
        gravity_penalty += b.4 - (b.0 * b.6 as f64);
        gravity_penalty += b.5 - (b.1 * b.6 as f64);
    }

    let mut fitness = total_bounds_area / total_part_area;

    let scale_factor = total_part_area.sqrt().max(1.0);
    fitness += (gravity_penalty / scale_factor) * 0.001;
    fitness += (compaction_penalty / scale_factor) * 0.01;

    // Orientation penalty
    let mut orientation_penalty = 0.0;
    for (i, polys) in polygon_groups.iter().enumerate() {
        if polys.len() == 1 && polys[0].len() == 4 {
            let rot = rotations.get(i).copied().unwrap_or(0.0);
            let rotation_mod = rot.abs() % 90.0;
            let rot_mod = if rotation_mod > 45.0 {
                90.0 - rotation_mod
            } else {
                rotation_mod
            };
            orientation_penalty += (rot_mod / 90.0) * 0.05;
        }
    }
    fitness += orientation_penalty;

    // Missing parts penalty
    if num_parts > 0 && polygon_groups.len() < num_parts {
        let missing = (num_parts - polygon_groups.len()) as f64;
        fitness += missing * 1000.0;
    }

    fitness
}
