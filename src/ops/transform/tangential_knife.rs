//! Tangential-knife support: adds a rotary axis (A) that keeps the
//! blade tangent to the toolpath, with Z lifts for sharp turns.
//!
//! Every moving command of a contour gets an ``A`` extra-axis value
//! carrying the (continuous, unwrapped) heading in degrees of the
//! path at the command's end point. Straight segments therefore cut
//! with a constant blade angle while arcs rotate the blade
//! continuously during the cut.
//!
//! At junctions between segments the heading changes in a step; if
//! the change exceeds ``angle_tolerance_deg`` — or either adjacent
//! segment is an arc tighter than ``radius_tolerance_mm`` — the knife
//! is lifted to ``safe_z``, rotated in the air, and plunged back in
//! at the corner. Smaller changes rotate the blade in place with the
//! knife still down. Tight arcs (below ``radius_tolerance_mm``) are
//! traversed entirely in the air: the knife lifts before the arc,
//! follows it at ``safe_z`` while rotating, and plunges at its end.
//!
//! Only contours inside [`SectionType::VectorOutline`] sections are
//! touched; everything else is transferred unchanged.

use crate::geo::algo::analysis::get_tangent_at_from_array;
use crate::geo::types::Point3D;
use crate::ops::container::Ops;
use crate::ops::enums::{CommandCategory, CommandType, SectionType};
use crate::ops::transform::{Phase, TransformCtx, Transformer};
use crate::ops::types::{MarkerCmd, MoveCmd, OpCategory};
use crate::ops::Axis;

/// Junctions whose turn angle is below this (radians) are considered
/// tangent-continuous and need no rotation move.
const ANGLE_EPS_RAD: f64 = 1e-9;

/// Parameters for the [`apply_tangential_knife`] transformer.
#[derive(Clone, Debug, PartialEq)]
pub struct TangentialKnifeSpec {
    /// Maximum heading change (degrees) rotated with the knife down.
    pub angle_tolerance_deg: f64,
    /// Arcs tighter than this radius (millimeters) force a lift.
    pub radius_tolerance_mm: f64,
    /// Z height used for lifting the knife at sharp corners.
    pub safe_z: f64,
}

impl Transformer for TangentialKnifeSpec {
    fn phase(&self) -> Phase {
        Phase::PostProcessing
    }

    fn apply(&self, ctx: &mut TransformCtx<'_>) {
        apply_tangential_knife(
            ctx.ops,
            self.angle_tolerance_deg,
            self.radius_tolerance_mm,
            self.safe_z,
        );
    }

    fn name(&self) -> &str {
        "tangential_knife"
    }

    fn cache_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.name().hash(&mut h);
        self.angle_tolerance_deg.to_bits().hash(&mut h);
        self.radius_tolerance_mm.to_bits().hash(&mut h);
        self.safe_z.to_bits().hash(&mut h);
        h.finish()
    }
}

pub fn apply_tangential_knife(
    ops: &mut Ops,
    angle_tolerance_deg: f64,
    radius_tolerance_mm: f64,
    safe_z: f64,
) {
    if ops.is_empty() {
        return;
    }

    ops.preload_state();

    let mut new_ops = Ops::new();
    let mut line_buffer: Vec<usize> = Vec::new();
    let mut in_vector_section = false;

    let flush_buffer = |buf: &mut Vec<usize>, new: &mut Ops, old: &Ops| {
        if !buf.is_empty() {
            rewrite_buffered_contour(
                new,
                old,
                buf,
                angle_tolerance_deg,
                radius_tolerance_mm,
                safe_z,
            );
            buf.clear();
        }
    };

    for i in 0..ops.len() {
        let ct = ops.command_type(i);

        let is_start = if ct == CommandType::OpsSectionStart {
            if let OpCategory::Marker(MarkerCmd::OpsSectionStart {
                section_type,
                ..
            }) = &ops.commands[i].category
            {
                *section_type == SectionType::VectorOutline
            } else {
                false
            }
        } else {
            false
        };

        let is_end = if ct == CommandType::OpsSectionEnd {
            if let OpCategory::Marker(MarkerCmd::OpsSectionEnd {
                section_type,
                ..
            }) = &ops.commands[i].category
            {
                *section_type == SectionType::VectorOutline
            } else {
                false
            }
        } else {
            false
        };

        if is_start {
            flush_buffer(&mut line_buffer, &mut new_ops, ops);
            in_vector_section = true;
            new_ops.transfer_command_from(ops, i);
        } else if is_end {
            flush_buffer(&mut line_buffer, &mut new_ops, ops);
            in_vector_section = false;
            new_ops.transfer_command_from(ops, i);
        } else if !in_vector_section {
            new_ops.transfer_command_from(ops, i);
        } else if ct == CommandType::MoveTo {
            flush_buffer(&mut line_buffer, &mut new_ops, ops);
            line_buffer.push(i);
        } else if !line_buffer.is_empty() {
            line_buffer.push(i);
        } else {
            flush_buffer(&mut line_buffer, &mut new_ops, ops);
            new_ops.transfer_command_from(ops, i);
        }
    }

    flush_buffer(&mut line_buffer, &mut new_ops, ops);
    ops.replace_with(&new_ops);
}

fn make_sub_ops(ops: &Ops, indices: &[usize]) -> Ops {
    let mut sub = Ops::new();
    for &j in indices {
        sub.transfer_command_from(ops, j);
    }
    sub
}

fn shortest_diff_deg(from_deg: f64, to_deg: f64) -> f64 {
    (to_deg - from_deg + 180.0).rem_euclid(360.0) - 180.0
}
fn arc_radius(old_ops: &Ops, idx: usize) -> f64 {
    if let OpCategory::Moving {
        cmd: MoveCmd::ArcTo(arc),
        ..
    } = &old_ops.commands[idx].category
    {
        arc.center.length()
    } else {
        f64::INFINITY
    }
}

fn arc_params(old_ops: &Ops, idx: usize) -> Option<(f64, f64, bool)> {
    if let OpCategory::Moving {
        cmd: MoveCmd::ArcTo(arc),
        ..
    } = &old_ops.commands[idx].category
    {
        Some((arc.center.x, arc.center.y, arc.cw))
    } else {
        None
    }
}

/// Signed heading change (degrees) of an arc segment, following the
/// arc's own sweep direction. `start` is the arc's start point and
/// the stored center is a relative (I/J) offset from it.
fn arc_heading_delta(
    old_ops: &Ops,
    idx: usize,
    start: Point3D,
    end: Point3D,
) -> Option<f64> {
    let (i, j, cw) = arc_params(old_ops, idx)?;
    let cx = start.x + i;
    let cy = start.y + j;
    let sa = (start.y - cy).atan2(start.x - cx).to_degrees();
    let ea = (end.y - cy).atan2(end.x - cx).to_degrees();
    let mut sweep = (ea - sa).rem_euclid(360.0);
    if sweep < 1e-9 {
        return None;
    }
    if cw {
        sweep -= 360.0;
    }
    Some(sweep)
}

fn lift_to(new_ops: &mut Ops, corner: Point3D, safe_z: f64, heading_deg: f64) {
    new_ops.line_to(
        corner.x,
        corner.y,
        safe_z,
        Some(vec![(Axis::A, heading_deg)]),
    );
}

fn rotate_at(
    new_ops: &mut Ops,
    corner: Point3D,
    z: f64,
    from_deg: f64,
    to_deg: f64,
) {
    if (to_deg - from_deg).abs() < ANGLE_EPS_RAD.to_degrees() {
        return;
    }
    new_ops.line_to(corner.x, corner.y, z, Some(vec![(Axis::A, to_deg)]));
}

fn handle_junction(
    new_ops: &mut Ops,
    corner: Point3D,
    heading_prev_deg: f64,
    heading_new_deg: f64,
    force_lift: bool,
    safe_z: f64,
) {
    if (heading_new_deg - heading_prev_deg).abs() < ANGLE_EPS_RAD.to_degrees() {
        return;
    }
    if force_lift && safe_z > corner.z + 1e-9 {
        lift_to(new_ops, corner, safe_z, heading_prev_deg);
        rotate_at(new_ops, corner, safe_z, heading_prev_deg, heading_new_deg);
        new_ops.line_to(
            corner.x,
            corner.y,
            corner.z,
            Some(vec![(Axis::A, heading_new_deg)]),
        );
    } else {
        rotate_at(new_ops, corner, corner.z, heading_prev_deg, heading_new_deg);
    }
}

fn rewrite_buffered_contour(
    new_ops: &mut Ops,
    old_ops: &Ops,
    indices: &[usize],
    angle_tolerance_deg: f64,
    radius_tolerance_mm: f64,
    safe_z: f64,
) {
    let moving_indices: Vec<usize> = indices
        .iter()
        .copied()
        .filter(|&j| old_ops.category(j) == CommandCategory::Moving)
        .collect();

    let pass_through = |new: &mut Ops| {
        for &j in indices {
            new.transfer_command_from(old_ops, j);
        }
    };

    let n = moving_indices.len();
    if n < 2 || old_ops.command_type(moving_indices[0]) != CommandType::MoveTo {
        pass_through(new_ops);
        return;
    }

    let sub = make_sub_ops(old_ops, indices);
    let geo = sub.to_geometry();
    let data = geo.data();

    let mut tin = vec![0.0f64; n];
    let mut tout = vec![0.0f64; n];
    for k in 1..n {
        let t0 = get_tangent_at_from_array(data, k, 0.0)
            .filter(|t| t.length() >= 1e-9);
        let t1 = get_tangent_at_from_array(data, k, 1.0)
            .filter(|t| t.length() >= 1e-9);
        match (t0, t1) {
            (Some(a), Some(b)) => {
                tin[k] = a.y.atan2(a.x).to_degrees();
                tout[k] = b.y.atan2(b.x).to_degrees();
            }
            _ => {
                pass_through(new_ops);
                return;
            }
        }
    }

    let mut heading_in = vec![0.0f64; n];
    let mut heading_out = vec![0.0f64; n];
    heading_in[1] = tin[1];
    heading_out[1] = heading_in[1] + shortest_diff_deg(tin[1], tout[1]);

    let mut seg_k = 0usize;
    for &j in indices {
        match old_ops.command_type(j) {
            CommandType::MoveTo => {
                new_ops.transfer_command_from(old_ops, j);
                let idx = new_ops.len() - 1;
                new_ops.set_extra_axes(idx, vec![(Axis::A, heading_in[1])]);
            }
            ct if ct.category() == CommandCategory::Moving => {
                seg_k += 1;
                let start = old_ops.endpoint(moving_indices[seg_k - 1]);
                let tight = arc_radius(old_ops, j) < radius_tolerance_mm
                    && safe_z > start.z + 1e-9;
                let heading_in_k;
                if seg_k >= 2 {
                    let turn = shortest_diff_deg(tout[seg_k - 1], tin[seg_k]);
                    heading_in_k = heading_out[seg_k - 1] + turn;
                    let corner = old_ops.endpoint(moving_indices[seg_k - 1]);
                    if tight {
                        lift_to(
                            new_ops,
                            corner,
                            safe_z,
                            heading_out[seg_k - 1],
                        );
                        rotate_at(
                            new_ops,
                            corner,
                            safe_z,
                            heading_out[seg_k - 1],
                            heading_in_k,
                        );
                    } else {
                        let radius_forced =
                            arc_radius(old_ops, moving_indices[seg_k])
                                < radius_tolerance_mm;
                        handle_junction(
                            new_ops,
                            corner,
                            heading_out[seg_k - 1],
                            heading_in_k,
                            turn.abs() > angle_tolerance_deg || radius_forced,
                            safe_z,
                        );
                    }
                } else {
                    heading_in_k = heading_in[1];
                    if tight {
                        lift_to(new_ops, start, safe_z, heading_in_k);
                    }
                }
                let end = old_ops.endpoint(j);
                heading_out[seg_k] = heading_in_k
                    + arc_heading_delta(old_ops, j, start, end).unwrap_or_else(
                        || shortest_diff_deg(tin[seg_k], tout[seg_k]),
                    );

                if tight {
                    // Traverse the tight arc in the air and plunge at
                    // its end point.
                    if let Some((i, joff, cw)) = arc_params(old_ops, j) {
                        let end = old_ops.endpoint(j);
                        new_ops.arc_to(
                            end.x,
                            end.y,
                            i,
                            joff,
                            cw,
                            safe_z,
                            Some(vec![(Axis::A, heading_out[seg_k])]),
                        );
                        new_ops.line_to(
                            end.x,
                            end.y,
                            end.z,
                            Some(vec![(Axis::A, heading_out[seg_k])]),
                        );
                        continue;
                    }
                }

                new_ops.transfer_command_from(old_ops, j);
                let idx = new_ops.len() - 1;
                new_ops
                    .set_extra_axes(idx, vec![(Axis::A, heading_out[seg_k])]);
            }
            _ => {
                new_ops.transfer_command_from(old_ops, j);
            }
        }
    }
}
