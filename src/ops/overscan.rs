use super::container::Ops;
use super::enums::{CommandCategory, CommandType, SectionType};
use super::types::{MarkerCmd, OpCategory};
use crate::types::Point3D;

pub fn apply_overscan(ops: &mut Ops, distance_mm: f64) {
    if ops.is_empty() || distance_mm <= 0.0 {
        return;
    }

    ops.preload_state();

    let mut new_ops = Ops::new();
    let mut line_buffer: Vec<usize> = Vec::new();
    let mut in_raster_section = false;

    let flush_buffer =
        |buf: &mut Vec<usize>, new: &mut Ops, old: &Ops, dist: f64| {
            if !buf.is_empty() {
                rewrite_buffered_line(new, old, buf, dist);
                buf.clear();
            }
        };

    for i in 0..ops.len() {
        let ct = ops.command_type(i);

        let is_section_start = if ct == CommandType::OpsSectionStart {
            if let OpCategory::Marker(MarkerCmd::OpsSectionStart {
                section_type,
                ..
            }) = &ops.commands[i].category
            {
                *section_type == SectionType::RasterFill
            } else {
                false
            }
        } else {
            false
        };

        let is_section_end = if ct == CommandType::OpsSectionEnd {
            if let OpCategory::Marker(MarkerCmd::OpsSectionEnd {
                section_type,
                ..
            }) = &ops.commands[i].category
            {
                *section_type == SectionType::RasterFill
            } else {
                false
            }
        } else {
            false
        };

        if is_section_start {
            flush_buffer(&mut line_buffer, &mut new_ops, ops, distance_mm);
            in_raster_section = true;
            new_ops.transfer_command_from(ops, i);
        } else if is_section_end {
            flush_buffer(&mut line_buffer, &mut new_ops, ops, distance_mm);
            in_raster_section = false;
            new_ops.transfer_command_from(ops, i);
        } else if !in_raster_section {
            new_ops.transfer_command_from(ops, i);
        } else if ct == CommandType::MoveTo {
            flush_buffer(&mut line_buffer, &mut new_ops, ops, distance_mm);
            line_buffer.push(i);
        } else if !line_buffer.is_empty() {
            line_buffer.push(i);
        } else {
            flush_buffer(&mut line_buffer, &mut new_ops, ops, distance_mm);
            new_ops.transfer_command_from(ops, i);
        }
    }

    flush_buffer(&mut line_buffer, &mut new_ops, ops, distance_mm);
    ops.replace_with(&new_ops);
}

fn rewrite_buffered_line(
    new_ops: &mut Ops,
    old_ops: &Ops,
    indices: &[usize],
    distance_mm: f64,
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

    let content_start = old_ops.endpoint(moving_indices[0]);
    let content_end =
        old_ops.endpoint(moving_indices[moving_indices.len() - 1]);

    let (sx, sy) = (content_start.0, content_start.1);
    let (ex, ey) = (content_end.0, content_end.1);

    if sx == ex && sy == ey {
        for &j in indices {
            new_ops.transfer_command_from(old_ops, j);
        }
        return;
    }

    let dx = ex - sx;
    let dy = ey - sy;
    let original_length = dx.hypot(dy);
    if original_length < 1e-9 {
        for &j in indices {
            new_ops.transfer_command_from(old_ops, j);
        }
        return;
    }

    let dir_x = dx / original_length;
    let dir_y = dy / original_length;

    let overscan_start: Point3D = (
        sx - distance_mm * dir_x,
        sy - distance_mm * dir_y,
        content_start.2,
    );
    let overscan_end: Point3D = (
        ex + distance_mm * dir_x,
        ey + distance_mm * dir_y,
        content_end.2,
    );

    // Case 1: Variable Power ScanLine
    if indices.len() == 2
        && old_ops.command_type(indices[1]) == CommandType::ScanLine
    {
        let scan_idx = indices[1];
        let old_pv = old_ops.scanline_data(scan_idx);
        let pixels_per_mm = if original_length > 0.0 {
            old_pv.len() as f64 / original_length
        } else {
            0.0
        };
        let num_pad = (distance_mm * pixels_per_mm).round() as usize;
        let pad = vec![0u8; num_pad];

        let mut padded =
            Vec::with_capacity(pad.len() + old_pv.len() + pad.len());
        padded.extend_from_slice(&pad);
        padded.extend_from_slice(&old_pv);
        padded.extend_from_slice(&pad);

        new_ops.move_to(
            overscan_start.0,
            overscan_start.1,
            overscan_start.2,
            None,
        );
        new_ops.scan_to(
            overscan_end.0,
            overscan_end.1,
            overscan_end.2,
            Some(padded),
            None,
        );
        return;
    }

    // Case 2: Constant Power LineTo(s)
    let first_cut_idx = moving_indices[1..]
        .iter()
        .find(|&&j| old_ops.command_type(j) == CommandType::LineTo)
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

    let original_power = old_ops
        .preloaded_state(first_cut_idx)
        .map(|s| s.power)
        .unwrap_or(0.0);

    new_ops.move_to(overscan_start.0, overscan_start.1, overscan_start.2, None);
    new_ops.set_power(0.0);
    new_ops.line_to(content_start.0, content_start.1, content_start.2, None);

    let content_indices = &indices[1..];

    let needs_power_set = content_indices.is_empty()
        || old_ops.command_type(content_indices[0]) != CommandType::SetPower;

    if needs_power_set {
        new_ops.set_power(original_power);
    }

    for &j in content_indices {
        new_ops.transfer_command_from(old_ops, j);
    }

    new_ops.set_power(0.0);
    new_ops.line_to(overscan_end.0, overscan_end.1, overscan_end.2, None);
}
