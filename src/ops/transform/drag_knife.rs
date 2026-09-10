//! Drag-knife compensation: rewrites vector contours into the pivot
//! path a trailing-blade drag knife must follow.
//!
//! A drag knife's blade tip trails the machine's controlled point
//! (the pivot above the blade bearing) by the blade offset. The tip
//! only stays planted while the pivot rotates around it, so the
//! machine path differs from the desired cut path:
//!
//! - the pivot path of a straight segment is the segment shifted
//!   forward by ``offset`` along its direction;
//! - the pivot path of a circular arc is a concentric arc with
//!   radius ``sqrt(r^2 + offset^2)`` (the tip still traces the
//!   original arc exactly);
//! - the path starts with a half-circle "arc in" around the start
//!   point that swings the blade into the initial cut direction;
//! - at corners the pivot swivels around the corner point on a
//!   radius-``offset`` arc (blade stays down for turns within the
//!   swivel tolerance, lifts for sharper corners);
//! - the path ends with a half-circle "arc out" that overcuts the
//!   end point cleanly.
//!
//! Only contours inside [`SectionType::VectorOutline`] sections are
//! rewritten; everything else is transferred unchanged.

use crate::geo::algo::analysis::get_tangent_at_from_array;
use crate::geo::types::Point3D;
use crate::ops::container::Ops;
use crate::ops::enums::{CommandCategory, CommandType, SectionType};
use crate::ops::transform::{Phase, TransformCtx, Transformer};
use crate::ops::types::{MarkerCmd, MoveCmd, OpCategory};

/// Junctions whose turn angle is below this (radians) are considered
/// tangent-continuous.
const ANGLE_EPS: f64 = 1e-9;

/// Parameters for the [`apply_drag_knife`] transformer.
#[derive(Clone, Debug, PartialEq)]
pub struct DragKnifeSpec {
    /// Distance between blade tip and pivot in millimeters.
    pub offset_mm: f64,
    /// Maximum direction change (degrees) that is swiveled with the
    /// blade down. Sharper corners lift the knife before pivoting.
    pub swivel_angle_deg: f64,
}

impl Transformer for DragKnifeSpec {
    fn phase(&self) -> Phase {
        Phase::PostProcessing
    }

    fn apply(&self, ctx: &mut TransformCtx<'_>) {
        apply_drag_knife(ctx.ops, self.offset_mm, self.swivel_angle_deg);
    }

    fn name(&self) -> &str {
        "drag_knife"
    }

    fn cache_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.name().hash(&mut h);
        self.offset_mm.to_bits().hash(&mut h);
        self.swivel_angle_deg.to_bits().hash(&mut h);
        h.finish()
    }
}

pub fn apply_drag_knife(ops: &mut Ops, offset_mm: f64, swivel_angle_deg: f64) {
    if offset_mm <= 0.0 || ops.is_empty() {
        return;
    }

    ops.preload_state();

    let mut new_ops = Ops::new();
    let mut line_buffer: Vec<usize> = Vec::new();
    let mut in_vector_section = false;

    let flush_buffer =
        |buf: &mut Vec<usize>, new: &mut Ops, old: &Ops, off: f64, sw: f64| {
            if !buf.is_empty() {
                rewrite_buffered_contour(new, old, buf, off, sw);
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
            flush_buffer(
                &mut line_buffer,
                &mut new_ops,
                ops,
                offset_mm,
                swivel_angle_deg,
            );
            in_vector_section = true;
            new_ops.transfer_command_from(ops, i);
        } else if is_end {
            flush_buffer(
                &mut line_buffer,
                &mut new_ops,
                ops,
                offset_mm,
                swivel_angle_deg,
            );
            in_vector_section = false;
            new_ops.transfer_command_from(ops, i);
        } else if !in_vector_section {
            new_ops.transfer_command_from(ops, i);
        } else if ct == CommandType::MoveTo {
            flush_buffer(
                &mut line_buffer,
                &mut new_ops,
                ops,
                offset_mm,
                swivel_angle_deg,
            );
            line_buffer.push(i);
        } else if !line_buffer.is_empty() {
            line_buffer.push(i);
        } else {
            flush_buffer(
                &mut line_buffer,
                &mut new_ops,
                ops,
                offset_mm,
                swivel_angle_deg,
            );
            new_ops.transfer_command_from(ops, i);
        }
    }

    flush_buffer(
        &mut line_buffer,
        &mut new_ops,
        ops,
        offset_mm,
        swivel_angle_deg,
    );
    ops.replace_with(&new_ops);
}

enum SegKind {
    Line,
    Arc { center: (f64, f64), cw: bool },
    Bezier { c1: Point3D, c2: Point3D },
    Other,
}

struct Seg {
    src: usize,
    kind: SegKind,
    start: Point3D,
    end: Point3D,
    tin: (f64, f64),
    tout: (f64, f64),
}

fn make_sub_ops(ops: &Ops, indices: &[usize]) -> Ops {
    let mut sub = Ops::new();
    for &j in indices {
        sub.transfer_command_from(ops, j);
    }
    sub
}

fn normalize(v: (f64, f64)) -> Option<(f64, f64)> {
    let len = v.0.hypot(v.1);
    if len < 1e-9 {
        None
    } else {
        Some((v.0 / len, v.1 / len))
    }
}

fn signed_angle(a: (f64, f64), b: (f64, f64)) -> f64 {
    a.0.mul_add(b.1, -(a.1 * b.0)).atan2(a.0 * b.0 + a.1 * b.1)
}

fn collect_segments(
    old_ops: &Ops,
    moving_indices: &[usize],
    data: &[crate::geo::types::Command],
) -> Option<Vec<Seg>> {
    let mut segs = Vec::with_capacity(moving_indices.len() - 1);
    for k in 1..moving_indices.len() {
        let src = moving_indices[k];
        let start = data[k - 1].end_point();
        let end = data[k].end_point();
        let tin = get_tangent_at_from_array(data, k, 0.0)?;
        let tout = get_tangent_at_from_array(data, k, 1.0)?;
        if tin.length() < 1e-9 || tout.length() < 1e-9 {
            return None;
        }
        let tin = (tin.x, tin.y);
        let tout = (tout.x, tout.y);
        let kind = match old_ops.command_type(src) {
            CommandType::LineTo => SegKind::Line,
            CommandType::ArcTo => {
                if let OpCategory::Moving {
                    cmd: MoveCmd::ArcTo(arc),
                    ..
                } = &old_ops.commands[src].category
                {
                    let center =
                        (start.x + arc.center.x, start.y + arc.center.y);
                    SegKind::Arc { center, cw: arc.cw }
                } else {
                    SegKind::Other
                }
            }
            CommandType::BezierTo => {
                if let OpCategory::Moving {
                    cmd: MoveCmd::BezierTo(bez),
                    ..
                } = &old_ops.commands[src].category
                {
                    SegKind::Bezier {
                        c1: bez.control1,
                        c2: bez.control2,
                    }
                } else {
                    SegKind::Other
                }
            }
            _ => SegKind::Other,
        };
        segs.push(Seg {
            src,
            kind,
            start,
            end,
            tin,
            tout,
        });
    }
    Some(segs)
}

fn emit_shifted_segment(
    new_ops: &mut Ops,
    old_ops: &Ops,
    seg: &Seg,
    offset_mm: f64,
    cur: &mut (f64, f64),
) {
    let end_pivot = (
        seg.end.x + seg.tout.0 * offset_mm,
        seg.end.y + seg.tout.1 * offset_mm,
    );
    let idx = new_ops.len();
    match &seg.kind {
        SegKind::Line => {
            new_ops.line_to(end_pivot.0, end_pivot.1, seg.end.z, None);
        }
        SegKind::Arc { center, cw } => {
            let i = center.0 - cur.0;
            let j = center.1 - cur.1;
            new_ops.arc_to(
                end_pivot.0,
                end_pivot.1,
                i,
                j,
                *cw,
                seg.end.z,
                None,
            );
        }
        SegKind::Bezier { c1, c2 } => {
            let nend = Point3D::new(end_pivot.0, end_pivot.1, seg.end.z);
            let nc1 = Point3D::new(
                c1.x + seg.tin.0 * offset_mm,
                c1.y + seg.tin.1 * offset_mm,
                c1.z,
            );
            let c2d = normalize((seg.end.x - c2.x, seg.end.y - c2.y))
                .unwrap_or(seg.tout);
            let nc2 = Point3D::new(
                c2.x + c2d.0 * offset_mm,
                c2.y + c2d.1 * offset_mm,
                c2.z,
            );
            new_ops.bezier_to(nc1, nc2, nend, None);
        }
        SegKind::Other => {
            new_ops.transfer_command_from(old_ops, seg.src);
        }
    }
    if let Some(state) = old_ops.state(seg.src) {
        new_ops.set_state_at(idx, state);
    }
    *cur = end_pivot;
}

fn rewrite_buffered_contour(
    new_ops: &mut Ops,
    old_ops: &Ops,
    indices: &[usize],
    offset_mm: f64,
    swivel_angle_deg: f64,
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

    if moving_indices.len() < 2
        || old_ops.command_type(moving_indices[0]) != CommandType::MoveTo
    {
        pass_through(new_ops);
        return;
    }

    let sub = make_sub_ops(old_ops, indices);
    let geo = sub.to_geometry();
    let data = geo.data();

    let segs = match collect_segments(old_ops, &moving_indices, data) {
        Some(s) if s.len() == moving_indices.len() - 1 => s,
        _ => {
            pass_through(new_ops);
            return;
        }
    };

    let first_cut = segs
        .iter()
        .find(|s| old_ops.state(s.src).is_some())
        .map(|s| s.src);
    let first_cut = match first_cut {
        Some(idx) => idx,
        None => {
            pass_through(new_ops);
            return;
        }
    };
    let original_power =
        old_ops.state(first_cut).map(|s| s.power).unwrap_or(0.0);
    let swivel_rad = swivel_angle_deg.to_radians();

    // Arc-in: plunge the tip one offset behind the start (blade
    // pointing against the cut direction), then swing the pivot on a
    // half circle around the start point to align the blade with the
    // initial cut direction.
    let start = segs[0].start;
    let z = start.z;
    let d0 = segs[0].tin;
    new_ops.move_to(
        start.x - d0.0 * offset_mm,
        start.y - d0.1 * offset_mm,
        z,
        None,
    );
    new_ops.set_power(original_power);
    new_ops.arc_to(
        start.x + d0.0 * offset_mm,
        start.y + d0.1 * offset_mm,
        d0.0 * offset_mm,
        d0.1 * offset_mm,
        false,
        z,
        None,
    );

    let mut cur = (start.x + d0.0 * offset_mm, start.y + d0.1 * offset_mm);
    let mut prev_tout = segs[0].tin;
    let mut current_power = original_power;

    for (k, seg) in segs.iter().enumerate() {
        if k > 0 {
            let turn = signed_angle(prev_tout, seg.tin);
            if turn.abs() > ANGLE_EPS {
                let c = seg.start;
                let ce =
                    (c.x + seg.tin.0 * offset_mm, c.y + seg.tin.1 * offset_mm);
                let i = -prev_tout.0 * offset_mm;
                let j = -prev_tout.1 * offset_mm;
                let clockwise = turn < 0.0;
                if turn.abs() > swivel_rad {
                    new_ops.set_power(0.0);
                    new_ops.arc_to(ce.0, ce.1, i, j, clockwise, z, None);
                    new_ops.set_power(current_power);
                } else {
                    new_ops.arc_to(ce.0, ce.1, i, j, clockwise, z, None);
                }
                cur = ce;
            }
        }

        emit_shifted_segment(new_ops, old_ops, seg, offset_mm, &mut cur);
        if let Some(state) = old_ops.state(seg.src) {
            current_power = state.power;
        }
        prev_tout = seg.tout;
    }

    // Arc-out: swing the pivot on a half circle around the end point
    // so the blade overcuts the end cleanly.
    let last = segs.last().unwrap();
    let dn = last.tout;
    new_ops.arc_to(
        last.end.x - dn.0 * offset_mm,
        last.end.y - dn.1 * offset_mm,
        -dn.0 * offset_mm,
        -dn.1 * offset_mm,
        true,
        last.end.z,
        None,
    );
    new_ops.set_power(0.0);
}
