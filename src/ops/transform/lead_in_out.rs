use crate::geo::algo::analysis::get_tangent_at_from_array;
use crate::geo::types::Point3D;
use crate::ops::container::Ops;
use crate::ops::enums::{CommandCategory, CommandType, SectionType};
use crate::ops::transform::{Phase, TransformCtx, Transformer};
use crate::ops::types::{MarkerCmd, OpCategory};

/// Parameters for the [`apply_lead_in_out`] transformer.
#[derive(Clone, Debug, PartialEq)]
pub struct LeadInOutSpec {
    /// Lead-in distance in millimeters.
    pub lead_in_mm: f64,
    /// Lead-out distance in millimeters.
    pub lead_out_mm: f64,
}

impl Transformer for LeadInOutSpec {
    fn phase(&self) -> Phase {
        Phase::PathInterruption
    }

    fn apply(&self, ctx: &mut TransformCtx<'_>) {
        apply_lead_in_out(ctx.ops, self.lead_in_mm, self.lead_out_mm);
    }

    fn name(&self) -> &str {
        "lead_in_out"
    }

    fn cache_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.name().hash(&mut h);
        self.lead_in_mm.to_bits().hash(&mut h);
        self.lead_out_mm.to_bits().hash(&mut h);
        h.finish()
    }
}

pub fn apply_lead_in_out(ops: &mut Ops, lead_in_mm: f64, lead_out_mm: f64) {
    let has_lead_in = lead_in_mm > 0.0;
    let has_lead_out = lead_out_mm > 0.0;
    if !has_lead_in && !has_lead_out {
        return;
    }
    if ops.is_empty() {
        return;
    }

    ops.preload_state();

    let mut new_ops = Ops::new();
    let mut line_buffer: Vec<usize> = Vec::new();
    let mut in_vector_section = false;

    let flush_buffer =
        |buf: &mut Vec<usize>, new: &mut Ops, old: &Ops, li: f64, lo: f64| {
            if !buf.is_empty() {
                rewrite_buffered_contour(new, old, buf, li, lo);
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
                lead_in_mm,
                lead_out_mm,
            );
            in_vector_section = true;
            new_ops.transfer_command_from(ops, i);
        } else if is_end {
            flush_buffer(
                &mut line_buffer,
                &mut new_ops,
                ops,
                lead_in_mm,
                lead_out_mm,
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
                lead_in_mm,
                lead_out_mm,
            );
            line_buffer.push(i);
        } else if !line_buffer.is_empty() {
            line_buffer.push(i);
        } else {
            flush_buffer(
                &mut line_buffer,
                &mut new_ops,
                ops,
                lead_in_mm,
                lead_out_mm,
            );
            new_ops.transfer_command_from(ops, i);
        }
    }

    flush_buffer(&mut line_buffer, &mut new_ops, ops, lead_in_mm, lead_out_mm);
    ops.replace_with(&new_ops);
}

fn get_tangent_at_start(ops: &Ops, indices: &[usize]) -> Option<(f64, f64)> {
    let sub = make_sub_ops(ops, indices);
    let geo = sub.to_geometry();
    let data = geo.data();
    if data.len() < 2 {
        return None;
    }
    let seg_start_x = data[0].end_point().x;
    let seg_start_y = data[0].end_point().y;
    let seg_end_x = data[1].end_point().x;
    let seg_end_y = data[1].end_point().y;
    let seg_len = (seg_end_x - seg_start_x).hypot(seg_end_y - seg_start_y);
    if seg_len < 1e-9 {
        return None;
    }
    let tangent = get_tangent_at_from_array(data, 1, 0.0)?;
    let len = (tangent.x).hypot(tangent.y);
    if len < 1e-9 {
        return None;
    }
    Some((tangent.x / len, tangent.y / len))
}

fn get_tangent_at_end(ops: &Ops, indices: &[usize]) -> Option<(f64, f64)> {
    let sub = make_sub_ops(ops, indices);
    let geo = sub.to_geometry();
    let data = geo.data();
    if data.len() < 2 {
        return None;
    }
    let last_idx = data.len() - 1;
    let prev_x = data[last_idx - 1].end_point().x;
    let prev_y = data[last_idx - 1].end_point().y;
    let end_x = data[last_idx].end_point().x;
    let end_y = data[last_idx].end_point().y;
    let seg_len = (end_x - prev_x).hypot(end_y - prev_y);
    if seg_len < 1e-9 {
        return None;
    }
    let tangent = get_tangent_at_from_array(data, last_idx, 1.0)?;
    let len = (tangent.x).hypot(tangent.y);
    if len < 1e-9 {
        return None;
    }
    Some((tangent.x / len, tangent.y / len))
}

fn make_sub_ops(ops: &Ops, indices: &[usize]) -> Ops {
    let mut sub = Ops::new();
    for &j in indices {
        sub.transfer_command_from(ops, j);
    }
    sub
}

fn rewrite_buffered_contour(
    new_ops: &mut Ops,
    old_ops: &Ops,
    indices: &[usize],
    lead_in_mm: f64,
    lead_out_mm: f64,
) {
    let moving_indices: Vec<usize> = indices
        .iter()
        .copied()
        .filter(|&j| old_ops.category(j) == CommandCategory::Moving)
        .collect();

    if moving_indices.len() < 2
        || old_ops.command_type(moving_indices[0]) != CommandType::MoveTo
    {
        for &j in indices {
            new_ops.transfer_command_from(old_ops, j);
        }
        return;
    }

    let mut has_lead_in = lead_in_mm > 0.0;
    let mut has_lead_out = lead_out_mm > 0.0;

    if !has_lead_in && !has_lead_out {
        for &j in indices {
            new_ops.transfer_command_from(old_ops, j);
        }
        return;
    }

    let lead_in_tangent = if has_lead_in {
        match get_tangent_at_start(old_ops, indices) {
            Some(t) => Some(t),
            None => {
                has_lead_in = false;
                None
            }
        }
    } else {
        None
    };

    let lead_out_tangent = if has_lead_out {
        match get_tangent_at_end(old_ops, indices) {
            Some(t) => Some(t),
            None => {
                has_lead_out = false;
                None
            }
        }
    } else {
        None
    };

    if !has_lead_in && !has_lead_out {
        for &j in indices {
            new_ops.transfer_command_from(old_ops, j);
        }
        return;
    }

    let first_cut_idx = moving_indices[1..]
        .iter()
        .find(|&&j| {
            let ct = old_ops.command_type(j);
            matches!(
                ct,
                CommandType::LineTo
                    | CommandType::ArcTo
                    | CommandType::BezierTo
            ) && old_ops.state(j).is_some()
        })
        .copied();

    let first_cut_idx = match first_cut_idx {
        Some(idx) => idx,
        None => {
            for &j in indices {
                new_ops.transfer_command_from(old_ops, j);
            }
            return;
        }
    };

    let original_power =
        old_ops.state(first_cut_idx).map(|s| s.power).unwrap_or(0.0);

    let start_3d = old_ops.endpoint(moving_indices[0]);
    let end_3d = old_ops.endpoint(moving_indices[moving_indices.len() - 1]);

    if has_lead_in {
        if let Some((tx, ty)) = lead_in_tangent {
            let lead_in_start: Point3D = Point3D::new(
                start_3d.x - tx * lead_in_mm,
                start_3d.y - ty * lead_in_mm,
                start_3d.z,
            );
            new_ops.move_to(
                lead_in_start.x,
                lead_in_start.y,
                lead_in_start.z,
                None,
            );
            new_ops.set_power(0.0);
            new_ops.line_to(start_3d.x, start_3d.y, start_3d.z, None);
        }
    } else {
        new_ops.transfer_command_from(old_ops, indices[0]);
    }

    let content_indices = &indices[1..];
    let needs_power_set = content_indices.is_empty()
        || old_ops.command_type(content_indices[0]) != CommandType::SetPower;

    if needs_power_set {
        new_ops.set_power(original_power);
    }

    for &j in content_indices {
        new_ops.copy_command_from(old_ops, j);
    }

    if has_lead_out {
        if let Some((tx, ty)) = lead_out_tangent {
            let lead_out_end: Point3D = Point3D::new(
                end_3d.x + tx * lead_out_mm,
                end_3d.y + ty * lead_out_mm,
                end_3d.z,
            );
            new_ops.set_power(0.0);
            new_ops.line_to(
                lead_out_end.x,
                lead_out_end.y,
                lead_out_end.z,
                None,
            );
        }
    }
}
