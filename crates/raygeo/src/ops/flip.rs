use std::sync::Arc;

use super::container::Ops;
use super::enums::{CommandCategory, CommandType};
use super::types::{OpCommand, OpMetadata};

pub fn flip_ops(ops: &Ops) -> Ops {
    let moving_indices: Vec<usize> = (0..ops.len())
        .filter(|&i| {
            ops.command_type(i).category() == CommandCategory::Moving
        })
        .collect();

    if moving_indices.len() <= 1 {
        let mut result = Ops::new();
        for &i in &moving_indices {
            result.commands.push(ops.commands[i].clone());
        }
        result.invalidate_time_cache();
        return result;
    }

    let last_moving_end =
        ops.endpoint(moving_indices[moving_indices.len() - 1]);
    let first_state = ops.state(moving_indices[0]).cloned();

    let mut result = Ops::new();
    let mut first_cmd = OpCommand::new(CommandType::MoveTo);
    first_cmd.end = last_moving_end;
    first_cmd.state = first_state;
    result.commands.push(first_cmd);

    for k in (0..moving_indices.len() - 1).rev() {
        let orig_k_idx = moving_indices[k + 1];
        let orig_prev_idx = moving_indices[k];
        let ct = ops.command_type(orig_k_idx);
        let new_end = ops.endpoint(orig_prev_idx);
        let orig_state = ops.state(orig_k_idx).cloned();
        let orig_ea = ops.extra_axes(orig_k_idx).map(|ea| Arc::from(ea));

        if ct == CommandType::ScanLine {
            let pv = ops.scanline_data(orig_k_idx);
            let reversed: Vec<u8> = pv.iter().rev().copied().collect();
            let mut cmd = OpCommand::new(ct);
            cmd.end = new_end;
            cmd.metadata = OpMetadata::ScanLine(Arc::from(reversed));
            cmd.extra_axes = orig_ea;
            cmd.state = orig_state;
            result.commands.push(cmd);
        } else if ct == CommandType::BezierTo {
            let (c1, c2) = ops.bezier_params(orig_k_idx);
            let mut cmd = OpCommand::new(ct);
            cmd.end = new_end;
            cmd.metadata = OpMetadata::Bezier((*c2, *c1));
            cmd.extra_axes = orig_ea;
            cmd.state = orig_state;
            result.commands.push(cmd);
        } else if ct == CommandType::ArcTo {
            let original_start = ops.endpoint(orig_prev_idx);
            let original_end = ops.endpoint(orig_k_idx);
            let (ci, cj, cw) = ops.arc_params(orig_k_idx);
            let center_x = original_start.0 + ci;
            let center_y = original_start.1 + cj;
            let new_i = center_x - original_end.0;
            let new_j = center_y - original_end.1;
            let mut cmd = OpCommand::new(ct);
            cmd.end = new_end;
            cmd.metadata = OpMetadata::Arc((new_i, new_j, !cw));
            cmd.extra_axes = orig_ea;
            cmd.state = orig_state;
            result.commands.push(cmd);
        } else {
            let mut cmd = OpCommand::new(ct);
            cmd.end = new_end;
            cmd.extra_axes = orig_ea;
            cmd.state = orig_state;
            result.commands.push(cmd);
        }
    }

    result.invalidate_time_cache();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flip_empty() {
        let ops = Ops::new();
        let flipped = flip_ops(&ops);
        assert_eq!(flipped.len(), 0);
    }

    #[test]
    fn test_flip_single_move() {
        let mut ops = Ops::new();
        ops.move_to(1.0, 2.0, 3.0, None);
        let flipped = flip_ops(&ops);
        assert_eq!(flipped.len(), 1);
        assert_eq!(flipped.command_type(0), CommandType::MoveTo);
    }

    #[test]
    fn test_flip_lines_only() {
        let mut ops = Ops::new();
        ops.move_to(0.0, 0.0, 0.0, None);
        ops.line_to(10.0, 0.0, 1.0, None);
        ops.line_to(10.0, 10.0, 2.0, None);
        let flipped = flip_ops(&ops);
        assert_eq!(flipped.len(), 3);
        assert_eq!(flipped.endpoint(0), (10.0, 10.0, 2.0));
        assert_eq!(flipped.endpoint(1), (10.0, 0.0, 1.0));
        assert_eq!(flipped.endpoint(2), (0.0, 0.0, 0.0));
    }

    #[test]
    fn test_flip_arc() {
        let mut ops = Ops::new();
        ops.move_to(0.0, 10.0, 0.0, None);
        ops.line_to(10.0, 0.0, 0.0, None);
        ops.arc_to(0.0, 0.0, -5.0, 0.0, false, 0.0, None);
        let flipped = flip_ops(&ops);
        assert_eq!(flipped.len(), 3);
        assert_eq!(flipped.endpoint(0), (0.0, 0.0, 0.0));
        assert_eq!(flipped.endpoint(1), (10.0, 0.0, 0.0));
        assert_eq!(flipped.endpoint(2), (0.0, 10.0, 0.0));
        assert_eq!(flipped.command_type(1), CommandType::ArcTo);
        let (ci, cj, cw) = flipped.arc_params(1);
        assert!(cw);
        assert!((ci - 5.0).abs() < 1e-9);
        assert!(cj.abs() < 1e-9);
    }

    #[test]
    fn test_flip_scanline() {
        let mut ops = Ops::new();
        ops.move_to(0.0, 0.0, 0.0, None);
        ops.scan_to(10.0, 10.0, 10.0, Some(vec![10, 20, 30]), None);
        let flipped = flip_ops(&ops);
        assert_eq!(flipped.len(), 2);
        assert_eq!(flipped.command_type(0), CommandType::MoveTo);
        assert_eq!(flipped.command_type(1), CommandType::ScanLine);
        assert_eq!(flipped.endpoint(0), (10.0, 10.0, 10.0));
        assert_eq!(flipped.endpoint(1), (0.0, 0.0, 0.0));
        let pv = flipped.scanline_data(1);
        assert_eq!(pv, &[30, 20, 10]);
    }
}
