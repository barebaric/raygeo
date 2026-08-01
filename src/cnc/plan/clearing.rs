//! Clearing plan builder.
//!
//! [`plan_clearing`] takes a [`Part`] and options and produces a [`Plan`]
//! organised as a BFS traversal of the region/passage graph:
//!
//! 1. Wide regions (detected by [`find_regions`]) are the graph nodes.
//! 2. Narrow / slot / unreachable passages (detected by
//!    [`analyze_pocket`]) are the graph edges.
//! 3. BFS from the largest region: enter → clear → clear connecting
//!    passage → move to the neighbour region.
//!
//! A region reachable through a cleared narrow/slot passage needs no
//! separate entry.  A region behind an unreachable passage gets its own
//! entry.  Dead-end passages (adjacent to only one region) are cleared
//! as part of that region's processing.
//!
//! The boundary and islands are read from the specified [`Part`] face.
//! All steps target that same face so that cleared fragments propagate
//! correctly between consecutive steps.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use prof_macros::prof;

use crate::cnc::plan::entry::{self, EntryWorkplanOptions};
use crate::cnc::plan::plan::{Plan, PlanStep};
use crate::error::RaygeoResult;
use crate::geo::algo::helix::HelixDirection;
use crate::geo::shape::polygon::{get_polygon_bounds, get_polygon_centroid};
use crate::geo::types::{Point, Point3D, Polygon};
use crate::ops::assembly::adaptive::AdaptiveClearingSpec;
use crate::ops::assembly::profile::{ProfileKind, ProfileSpec};
use crate::ops::assembly::slot::SlotSpec;
use crate::ops::assembly::toroid::ToroidalClearSpec;
use crate::ops::feature::narrow::{self, NarrowAnalysisOptions, PassageClass};
use crate::ops::feature::ramp::find_ramp_carrier;
use crate::ops::feature::region::{self, Region};
use crate::ops::feature::slot_path::{
    find_slot_path, measure_passage_min_width,
};
use crate::ops::part::Part;

/// Options for [`plan_clearing`].
///
/// The pocket boundary and islands are read from the [`Part`] — this
/// struct carries only the tool and machining parameters.
#[derive(Clone, Debug)]
pub struct ClearingWorkplanOptions {
    pub tool_radius: f64,
    pub step_over: f64,
    pub step_length: f64,
    pub target_z: f64,
    pub safe_z: f64,
    pub wall_margin: f64,
    pub safe_margin: f64,
    pub stock_to_leave: f64,
    pub plunge_pitch: f64,
    pub angular_step: f64,
    pub area_tolerance: f64,
    pub max_deflection_deg: f64,
    pub finishing: bool,
}

/// A directed edge in the region graph.
#[derive(Clone)]
struct PassageEdge {
    passage_idx: usize,
    region_a: usize,
    region_b: usize,
}

#[prof]
pub fn plan_clearing(
    part: &Part,
    face_id: &str,
    opts: &ClearingWorkplanOptions,
) -> RaygeoResult<Plan> {
    // Read boundary + islands from the specified face.
    let primary = part.face(face_id).ok_or_else(|| {
        crate::error::RaygeoError::InternalError(format!(
            "Part has no face {:?}",
            face_id
        ))
    })?;
    let (boundary, islands) = primary.extract_boundary();
    let boundary = boundary.ok_or_else(|| {
        crate::error::RaygeoError::InternalError(
            "Face has no boundary geometry".into(),
        )
    })?;

    let mut steps = Vec::new();

    let regions =
        region::find_regions(&boundary, &islands, opts.tool_radius, 0.5);

    if regions.is_empty() {
        return Ok(Plan::new(boundary, islands, opts.safe_z));
    }

    let passages = narrow::analyze_pocket(
        &boundary,
        &islands,
        &NarrowAnalysisOptions {
            tool_radius: opts.tool_radius,
            tolerance: 0.5,
            min_slot_width: 0.0,
        },
    )
    .unwrap_or_default();
    let edges = build_passage_edges(&passages, &regions);

    let mut visited = vec![false; regions.len()];
    let mut cleared_passages: HashSet<usize> = HashSet::new();
    let mut multi_region_passages: HashSet<usize> = HashSet::new();

    for edge in &edges {
        if edge.region_a != edge.region_b {
            multi_region_passages.insert(edge.passage_idx);
        }
    }

    let sorted_indices = sort_regions_by_size(&regions);
    if sorted_indices.is_empty() {
        return Ok(Plan::new(boundary, islands, opts.safe_z));
    }

    let entry_opts = EntryWorkplanOptions {
        islands: islands.clone(),
        tool_radius: opts.tool_radius,
        step_over: opts.step_over,
        safe_z: opts.safe_z,
        target_z: opts.target_z,
        plunge_pitch: opts.plunge_pitch,
        safe_margin: opts.safe_margin,
        angular_step: opts.angular_step,
    };

    // All steps share the same face so that cleared fragments
    // propagate correctly between consecutive steps.
    let cur_face = face_id.to_string();

    let mut queue: VecDeque<BfsItem> = VecDeque::new();
    queue.push_back(BfsItem {
        region_idx: sorted_indices[0],
        via_class: None,
        via_end: None,
    });

    visited[sorted_indices[0]] = true;

    while let Some(item) = queue.pop_front() {
        let region_idx = item.region_idx;
        let via_class = item.via_class;
        let via_end = item.via_end;
        let region = &regions[region_idx];

        let needs_entry =
            matches!(via_class, None | Some(PassageClass::Unreachable));

        if needs_entry {
            let mut entry_steps =
                entry::plan_entry(region, &entry_opts, &cur_face)?;
            // Set per-region boundary on entry steps so the seed is
            // created inside the region boundary.
            if matches!(via_class, None | Some(PassageClass::Unreachable)) {
                let bnd = Some((region.polygon.clone(), Vec::new()));
                for es in &mut entry_steps {
                    es.region_boundary = bnd.clone();
                }
            }
            steps.extend(entry_steps);
        }

        // Root / unreachable regions: scope to the region polygon so
        // the algorithm stays out of adjacent passages.
        // Passage-reached regions: use the full pocket boundary with
        // start_pos at the passage exit.  The already-cleared corridor
        // is visible through the shared face and won't be re-cut.
        let use_region_bnd =
            matches!(via_class, None | Some(PassageClass::Unreachable));

        let start_pos_3d = match via_class {
            Some(PassageClass::Slot) | Some(PassageClass::Narrow) => {
                via_end.map(|p| Point3D::new(p.x, p.y, opts.target_z))
            }
            _ => None,
        };

        steps.push(PlanStep {
            face_id: cur_face.clone(),
            spec: Arc::new(AdaptiveClearingSpec {
                tool_radius: opts.tool_radius,
                step_over: opts.step_over,
                step_length: opts.step_length,
                target_z: opts.target_z,
                safe_z: opts.safe_z,
                max_deflection_deg: opts.max_deflection_deg,
                wall_margin: opts.wall_margin,
                area_tolerance: opts.area_tolerance,
                cut_direction: crate::ops::types::CutDirection::Ccw,
                start_pos: start_pos_3d,
                start_heading: None,
                expansion_batch_size: 20,
                trace_path: None,
                tolerance: 0.01,
                cancel_check: None,
            }),
            region_boundary: if use_region_bnd {
                Some((region.polygon.clone(), Vec::new()))
            } else {
                None
            },
        });

        for (p_idx, passage) in passages.iter().enumerate() {
            if cleared_passages.contains(&p_idx) {
                continue;
            }
            if multi_region_passages.contains(&p_idx) {
                continue;
            }
            if !polygons_adjacent(&passage.polygon, &region.polygon, 1.0) {
                continue;
            }
            emit_passage_step(
                &mut steps,
                passage,
                &islands,
                opts.tool_radius,
                opts.target_z,
                opts.safe_z,
                Some(region.entry_pt),
                &cur_face,
            );
            cleared_passages.insert(p_idx);
        }

        let mut neighbors: Vec<(usize, PassageEdge)> = Vec::new();
        for edge in &edges {
            if edge.region_a == region_idx && !visited[edge.region_b] {
                neighbors.push((edge.region_b, edge.clone()));
            } else if edge.region_b == region_idx && !visited[edge.region_a] {
                neighbors.push((edge.region_a, edge.clone()));
            }
        }

        neighbors.sort_by(|a, b| {
            let da = centroid_distance(&regions[a.0].polygon, region.entry_pt);
            let db = centroid_distance(&regions[b.0].polygon, region.entry_pt);
            crate::utils::sort_f64(da, db)
        });

        for (neighbor_idx, edge) in neighbors {
            visited[neighbor_idx] = true;
            let passage = &passages[edge.passage_idx];

            let via_end = match passage.class {
                PassageClass::Narrow | PassageClass::Slot => {
                    if let Some(ep) = emit_passage_step(
                        &mut steps,
                        passage,
                        &islands,
                        opts.tool_radius,
                        opts.target_z,
                        opts.safe_z,
                        None,
                        &cur_face,
                    ) {
                        cleared_passages.insert(edge.passage_idx);
                        let d0 = ((ep.0.x - region.entry_pt.x).powi(2)
                            + (ep.0.y - region.entry_pt.y).powi(2))
                        .sqrt();
                        let d1 = ((ep.1.x - region.entry_pt.x).powi(2)
                            + (ep.1.y - region.entry_pt.y).powi(2))
                        .sqrt();
                        Some(if d0 > d1 { ep.0 } else { ep.1 })
                    } else {
                        None
                    }
                }
                PassageClass::Unreachable => None,
            };
            queue.push_back(BfsItem {
                region_idx: neighbor_idx,
                via_class: Some(passage.class),
                via_end,
            });
        }
    }

    if opts.finishing {
        let has_unreachable = passages
            .iter()
            .any(|p| p.class == PassageClass::Unreachable);

        if has_unreachable {
            for reg in &regions {
                steps.push(PlanStep {
                    face_id: cur_face.clone(),
                    spec: Arc::new(ProfileSpec {
                        kind: ProfileKind::Inner,
                        tool_radius: opts.tool_radius,
                        step_over: opts.step_over,
                        step_length: opts.step_length,
                        target_z: opts.target_z,
                        safe_z: opts.safe_z,
                        wall_margin: 0.0,
                        stock_to_leave: opts.stock_to_leave,
                        cut_direction: crate::ops::types::CutDirection::Ccw,
                        start_pos: None,
                        tolerance: 0.05,
                        expansion_batch_size: 20,
                        cancel_check: None,
                        engagement_area_threshold: 1.0,
                        engagement_angle_threshold: 30.0,
                        feed_reduction_factor: 0.5,
                        trace_path: None,
                    }),
                    region_boundary: Some((reg.polygon.clone(), Vec::new())),
                });
            }
        } else {
            steps.push(PlanStep {
                face_id: cur_face.clone(),
                spec: Arc::new(ProfileSpec {
                    kind: ProfileKind::Inner,
                    tool_radius: opts.tool_radius,
                    step_over: opts.step_over,
                    step_length: opts.step_length,
                    target_z: opts.target_z,
                    safe_z: opts.safe_z,
                    wall_margin: 0.0,
                    stock_to_leave: opts.stock_to_leave,
                    cut_direction: crate::ops::types::CutDirection::Ccw,
                    start_pos: None,
                    tolerance: 0.05,
                    expansion_batch_size: 20,
                    cancel_check: None,
                    engagement_area_threshold: 1.0,
                    engagement_angle_threshold: 30.0,
                    feed_reduction_factor: 0.5,
                    trace_path: None,
                }),
                region_boundary: None,
            });
        }
    }

    let mut plan = Plan::new(boundary, islands, opts.safe_z);
    plan.extend(steps);
    Ok(plan)
}

#[derive(Clone)]
struct BfsItem {
    region_idx: usize,
    via_class: Option<PassageClass>,
    via_end: Option<Point>,
}

#[allow(clippy::too_many_arguments)]
#[prof]
fn emit_passage_step(
    steps: &mut Vec<PlanStep>,
    passage: &narrow::NarrowRegion,
    islands: &[Polygon],
    tool_radius: f64,
    target_z: f64,
    safe_z: f64,
    dead_end_ref: Option<Point>,
    face_id: &str,
) -> Option<(Point, Point)> {
    match passage.class {
        PassageClass::Narrow => {
            if let Some((start, end)) =
                find_ramp_carrier(&passage.polygon, islands, tool_radius, 45.0)
            {
                let carrier_arr = [start, end];
                let pw =
                    measure_passage_min_width(&passage.polygon, &carrier_arr);
                let safe_step = safe_toroidal_step_over(pw, tool_radius);

                let margin = 0.2;
                let loop_r = (pw / 2.0 - tool_radius - margin).max(0.0);

                let (cs, ce) =
                    shorten_dead_end(start, end, loop_r, dead_end_ref);

                steps.push(PlanStep {
                    face_id: face_id.to_string(),
                    spec: Arc::new(ToroidalClearSpec {
                        carrier: vec![cs, ce],
                        start: Point3D::new(
                            (start.x + end.x) / 2.0,
                            (start.y + end.y) / 2.0,
                            safe_z,
                        ),
                        target_z,
                        tool_radius,
                        step_over: safe_step,
                        max_ramp_angle_deg: 45.0,
                        direction: HelixDirection::Cw,
                        angular_step: 0.1,
                    }),
                    region_boundary: None,
                });
                Some((start, end))
            } else {
                None
            }
        }
        PassageClass::Slot => {
            let entry_pt = get_polygon_centroid(&passage.polygon);
            if let Some(carrier) = find_slot_path(
                &passage.polygon,
                &passage.entry_edge_indices,
                entry_pt,
                tool_radius,
            ) {
                let last = carrier[carrier.len() - 1];
                let first = carrier[0];
                steps.push(PlanStep {
                    face_id: face_id.to_string(),
                    spec: Arc::new(SlotSpec {
                        carrier: carrier.clone(),
                        tool_radius,
                        target_z,
                    }),
                    region_boundary: None,
                });
                Some((first, last))
            } else {
                None
            }
        }
        PassageClass::Unreachable => None,
    }
}

fn safe_toroidal_step_over(passage_width: f64, tool_radius: f64) -> f64 {
    let margin = 0.2;
    let avail = (passage_width / 2.0 - tool_radius - margin).max(0.0);
    let alpha_half = (15.0f64).to_radians();
    (2.0 * avail * alpha_half.sin()).max(0.05)
}

fn shorten_dead_end(
    start: Point,
    end: Point,
    loop_r: f64,
    dead_end_ref: Option<Point>,
) -> (Point, Point) {
    let Some(ref_pt) = dead_end_ref else {
        return (start, end);
    };
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-12 || loop_r <= 0.0 {
        return (start, end);
    }
    let ux = dx / len;
    let uy = dy / len;
    let d_start =
        ((start.x - ref_pt.x).powi(2) + (start.y - ref_pt.y).powi(2)).sqrt();
    let d_end =
        ((end.x - ref_pt.x).powi(2) + (end.y - ref_pt.y).powi(2)).sqrt();
    if d_start < d_end {
        (start, Point::new(end.x - ux * loop_r, end.y - uy * loop_r))
    } else {
        (
            Point::new(start.x + ux * loop_r, start.y + uy * loop_r),
            end,
        )
    }
}

fn build_passage_edges(
    passages: &[narrow::NarrowRegion],
    regions: &[Region],
) -> Vec<PassageEdge> {
    let mut edges = Vec::new();
    for (p_idx, passage) in passages.iter().enumerate() {
        let mut region_ids: Vec<usize> = Vec::new();
        for (r_idx, region) in regions.iter().enumerate() {
            if polygons_adjacent(&passage.polygon, &region.polygon, 1.0) {
                region_ids.push(r_idx);
            }
        }
        if region_ids.len() >= 2 {
            for i in 0..region_ids.len() {
                for j in (i + 1)..region_ids.len() {
                    edges.push(PassageEdge {
                        passage_idx: p_idx,
                        region_a: region_ids[i],
                        region_b: region_ids[j],
                    });
                }
            }
        } else if region_ids.len() == 1 {
            edges.push(PassageEdge {
                passage_idx: p_idx,
                region_a: region_ids[0],
                region_b: region_ids[0],
            });
        }
    }
    edges
}

fn polygons_adjacent(a: &Polygon, b: &Polygon, tolerance: f64) -> bool {
    // Simple bounding-box check
    let ba = get_polygon_bounds(a);
    let bb = get_polygon_bounds(b);
    let overlaps_x =
        (ba.min.x - tolerance) < bb.max.x && (ba.max.x + tolerance) > bb.min.x;
    let overlaps_y =
        (ba.min.y - tolerance) < bb.max.y && (ba.max.y + tolerance) > bb.min.y;
    overlaps_x && overlaps_y
}

fn sort_regions_by_size(regions: &[Region]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..regions.len()).collect();
    indices.sort_by(|a, b| {
        regions[*b]
            .area
            .partial_cmp(&regions[*a].area)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    indices
}

fn centroid_distance(poly: &Polygon, pt: Point) -> f64 {
    let c = get_polygon_centroid(poly);
    (c.x - pt.x).hypot(c.y - pt.y)
}

pub fn safe_toroidal_step_over_public(
    passage_width: f64,
    tool_radius: f64,
) -> f64 {
    safe_toroidal_step_over(passage_width, tool_radius)
}
