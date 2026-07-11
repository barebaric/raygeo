//! Clearing workplan builder.
//!
//! [`build_clearing_workplan`] takes a pocket boundary with islands and
//! produces a sequence of [`WorkplanStep`]s organised as a BFS traversal
//! of the region/passage graph:
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

use std::collections::{HashSet, VecDeque};

use prof_macros::prof;

use crate::cnc::machining::entry::{self, EntryWorkplanOptions};
use crate::cnc::machining::plan::WorkplanStep;
use crate::error::RaygeoResult;
use crate::geo::algo::helix::HelixDirection;
use crate::geo::shape::polygon::{get_polygon_bounds, get_polygon_centroid};
use crate::ops::feature::narrow::{self, NarrowAnalysisOptions, PassageClass};
use crate::ops::feature::ramp::find_ramp_carrier;
use crate::ops::feature::region::{self, Region};
use crate::ops::feature::slot_path::{
    find_slot_path, measure_passage_min_width,
};
use crate::part::Part;
use crate::types::{Point, Point3D, Polygon};

/// Options for [`build_clearing_workplan`].
#[derive(Clone, Debug)]
pub struct ClearingWorkplanOptions {
    pub pocket_boundary: Polygon,
    pub islands: Vec<Polygon>,
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
pub fn build_clearing_workplan(
    opts: &ClearingWorkplanOptions,
) -> RaygeoResult<Vec<WorkplanStep>> {
    let mut steps = Vec::new();

    let regions = region::find_regions(
        &opts.pocket_boundary,
        &opts.islands,
        opts.tool_radius,
        0.5,
    );

    let passages = narrow::analyze_pocket(
        &opts.pocket_boundary,
        &opts.islands,
        &NarrowAnalysisOptions {
            tool_radius: opts.tool_radius,
            tolerance: 0.5,
            min_slot_width: 0.0,
        },
    );

    let passages = passages.unwrap_or_default();

    if regions.is_empty() {
        return Ok(steps);
    }

    let edges = build_connectivity(&regions, &passages);

    let multi_region_passages: HashSet<usize> =
        edges.iter().map(|e| e.passage_idx).collect();

    let mut cleared_passages: HashSet<usize> = HashSet::new();

    struct BfsItem {
        region_idx: usize,
        via_class: Option<PassageClass>,
        via_end: Option<Point>,
    }

    let mut visited = vec![false; regions.len()];
    let mut queue: VecDeque<BfsItem> = VecDeque::new();

    visited[0] = true;
    queue.push_back(BfsItem {
        region_idx: 0,
        via_class: None,
        via_end: None,
    });

    let entry_opts = EntryWorkplanOptions {
        islands: opts.islands.clone(),
        tool_radius: opts.tool_radius,
        step_over: opts.step_over,
        safe_z: opts.safe_z,
        target_z: opts.target_z,
        plunge_pitch: opts.plunge_pitch,
        safe_margin: opts.safe_margin,
        angular_step: opts.angular_step,
    };

    while let Some(item) = queue.pop_front() {
        let region_idx = item.region_idx;
        let via_class = item.via_class;
        let via_end = item.via_end;
        let region = &regions[region_idx];

        let needs_entry =
            matches!(via_class, None | Some(PassageClass::Unreachable));

        if needs_entry {
            let entry_steps = entry::build_entry_workplan(region, &entry_opts)?;
            steps.extend(entry_steps);
        }

        // For the root region and regions behind an unreachable passage we
        // scope the adaptive clearing to just the region polygon, keeping
        // the algorithm out of adjacent passages.  For regions reached via
        // a cleared passage the boundary is the full pocket (accessible
        // through the passage) and we supply an explicit start position
        // at the passage end.
        let boundary = match via_class {
            None | Some(PassageClass::Unreachable) => region.polygon.clone(),
            _ => opts.pocket_boundary.clone(),
        };

        let start_pos_3d = match via_class {
            Some(PassageClass::Slot) | Some(PassageClass::Narrow) => {
                via_end.map(|p| Point3D::new(p.x, p.y, opts.target_z))
            }
            _ => None,
        };

        let part = Part::from_polygons(&boundary, &opts.islands, (0.0, 0.0));
        steps.push(WorkplanStep::AdaptiveClear {
            part,
            tool_radius: opts.tool_radius,
            step_over: opts.step_over,
            step_length: opts.step_length,
            target_z: opts.target_z,
            safe_z: opts.safe_z,
            max_deflection_deg: opts.max_deflection_deg,
            wall_margin: opts.wall_margin,
            area_tolerance: opts.area_tolerance,
            angular_step: opts.angular_step,
            start_pos: start_pos_3d,
            start_heading: None,
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
                &opts.islands,
                opts.tool_radius,
                opts.target_z,
                opts.safe_z,
                Some(region.entry_pt),
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
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });

        for (neighbor_idx, edge) in neighbors {
            visited[neighbor_idx] = true;
            let passage = &passages[edge.passage_idx];

            let via_end = match passage.class {
                PassageClass::Narrow | PassageClass::Slot => {
                    if let Some(ep) = emit_passage_step(
                        &mut steps,
                        passage,
                        &opts.islands,
                        opts.tool_radius,
                        opts.target_z,
                        opts.safe_z,
                        None,
                    ) {
                        cleared_passages.insert(edge.passage_idx);
                        // Pick the endpoint farther from the current region.
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
            for region in &regions {
                let part = Part::from_polygons(
                    &region.polygon,
                    &opts.islands,
                    (0.0, 0.0),
                );
                steps.push(WorkplanStep::ProfileInner {
                    part,
                    tool_radius: opts.tool_radius,
                    step_over: opts.step_over,
                    step_length: opts.step_length,
                    target_z: opts.target_z,
                    safe_z: opts.safe_z,
                    wall_margin: 0.0,
                    stock_to_leave: opts.stock_to_leave,
                });
            }
        } else {
            let part = Part::from_polygons(
                &opts.pocket_boundary,
                &opts.islands,
                (0.0, 0.0),
            );
            steps.push(WorkplanStep::ProfileInner {
                part,
                tool_radius: opts.tool_radius,
                step_over: opts.step_over,
                step_length: opts.step_length,
                target_z: opts.target_z,
                safe_z: opts.safe_z,
                wall_margin: 0.0,
                stock_to_leave: opts.stock_to_leave,
            });
        }
    }

    Ok(steps)
}

#[prof]
fn emit_passage_step(
    steps: &mut Vec<WorkplanStep>,
    passage: &narrow::NarrowRegion,
    islands: &[Polygon],
    tool_radius: f64,
    target_z: f64,
    safe_z: f64,
    dead_end_ref: Option<Point>,
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

                steps.push(WorkplanStep::ToroidalClear {
                    carrier: vec![cs, ce],
                    start: Point3D::new(cs.x, cs.y, safe_z),
                    target_z,
                    tool_radius,
                    step_over: safe_step,
                    max_ramp_angle_deg: 45.0,
                    direction: HelixDirection::Cw,
                    angular_step: 0.1,
                });
                Some((cs, ce))
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
                steps.push(WorkplanStep::Slot {
                    carrier,
                    tool_radius,
                    target_z,
                });
                Some((first, last))
            } else {
                None
            }
        }
        PassageClass::Unreachable => None,
    }
}

/// Shorten the carrier at the dead-end by `loop_radius`.
///
/// `dead_end_ref` is the point on the connecting side (typically the
/// region entry point).  The carrier endpoint farther from it is the
/// dead-end and is pulled in by `loop_radius` along the carrier
/// direction so the trochoid's longitudinal extent stays inside the
/// passage.  When `dead_end_ref` is `None` (connecting passage) the
/// carrier is returned unchanged.
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

fn build_connectivity(
    regions: &[Region],
    passages: &[narrow::NarrowRegion],
) -> Vec<PassageEdge> {
    let tol = 1.0;
    let mut edges = Vec::new();

    for (p_idx, passage) in passages.iter().enumerate() {
        let adjacent: Vec<usize> = regions
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                polygons_adjacent(&passage.polygon, &r.polygon, tol)
            })
            .map(|(i, _)| i)
            .collect();

        if adjacent.len() >= 2 {
            edges.push(PassageEdge {
                passage_idx: p_idx,
                region_a: adjacent[0],
                region_b: adjacent[1],
            });
        }
    }

    edges
}

fn polygons_adjacent(a: &Polygon, b: &Polygon, tol: f64) -> bool {
    for &pa in a.iter() {
        for window in b.windows(2) {
            if point_to_segment_distance(pa, window[0], window[1]) < tol {
                return true;
            }
        }
    }
    for &pb in b.iter() {
        for window in a.windows(2) {
            if point_to_segment_distance(pb, window[0], window[1]) < tol {
                return true;
            }
        }
    }
    false
}

fn point_to_segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-20 {
        return ((p.x - a.x).powi(2) + (p.y - a.y).powi(2)).sqrt();
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq).clamp(0.0, 1.0);
    let cx = a.x + t * dx;
    let cy = a.y + t * dy;
    ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt()
}

fn centroid_distance(poly: &Polygon, ref_pt: Point) -> f64 {
    let bounds = get_polygon_bounds(poly);
    let cx = (bounds.min.x + bounds.max.x) * 0.5;
    let cy = (bounds.min.y + bounds.max.y) * 0.5;
    ((cx - ref_pt.x).powi(2) + (cy - ref_pt.y).powi(2)).sqrt()
}

fn safe_toroidal_step_over(passage_w: f64, tool_radius: f64) -> f64 {
    let margin = 0.2;
    let avail = (passage_w / 2.0 - tool_radius - margin).max(0.0);
    let alpha_half = (15.0f64).to_radians();
    (2.0 * avail * alpha_half.sin()).max(0.05)
}
