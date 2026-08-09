use crate::ops::container::Ops;
use crate::ops::types::{MoveCmd, OpCategory, OpNode};

pub fn flip_ops(ops: &Ops) -> Ops {
    let moving_indices: Vec<usize> = (0..ops.len())
        .filter(|&i| ops.commands[i].is_moving())
        .collect();

    if moving_indices.is_empty() {
        return ops.copy();
    }

    let mut result = Ops::new();

    let last_moving_end =
        ops.commands[moving_indices[moving_indices.len() - 1]].end_point();
    let first_state = ops.commands[moving_indices[0]].state().cloned();

    let mut first_cmd = OpNode::move_to(
        last_moving_end.x,
        last_moving_end.y,
        last_moving_end.z,
        None,
    );
    if let Some(ref s) = first_state {
        first_cmd.set_state(s.clone());
    }
    result.cmds_mut().push(first_cmd);

    for k in (0..moving_indices.len() - 1).rev() {
        let orig_k_idx = moving_indices[k + 1];
        let orig_prev_idx = moving_indices[k];
        let new_end = ops.commands[orig_prev_idx].end_point();
        let orig_node = &ops.commands[orig_k_idx];
        let extra = orig_node.extra_axes.as_deref().map(|ea| ea.to_vec());

        if let OpCategory::Moving { cmd, .. } = &orig_node.category {
            let mut new_node = match cmd {
                MoveCmd::ScanLine { power_values } => {
                    let reversed: Vec<u8> =
                        power_values.iter().rev().copied().collect();
                    OpNode::scan_to(
                        new_end.x, new_end.y, new_end.z, reversed, extra,
                    )
                }
                MoveCmd::BezierTo { control1, control2 } => {
                    OpNode::bezier_to(*control2, *control1, new_end, extra)
                }
                MoveCmd::ArcTo { center, cw } => {
                    let original_start =
                        ops.commands[orig_prev_idx].end_point();
                    let original_end = ops.commands[orig_k_idx].end_point();
                    let center_x = original_start.x + center.x;
                    let center_y = original_start.y + center.y;
                    let new_i = center_x - original_end.x;
                    let new_j = center_y - original_end.y;
                    OpNode::arc_to(
                        new_end.x, new_end.y, new_i, new_j, !cw, new_end.z,
                        extra,
                    )
                }
                MoveCmd::MoveTo => {
                    OpNode::move_to(new_end.x, new_end.y, new_end.z, extra)
                }
                MoveCmd::LineTo => {
                    OpNode::line_to(new_end.x, new_end.y, new_end.z, extra)
                }
                MoveCmd::QuadraticBezierTo { control } => {
                    OpNode::quadratic_bezier_to(*control, new_end, extra)
                }
            };

            if let Some(s) = orig_node.state() {
                new_node.set_state(s.clone());
            }
            result.cmds_mut().push(new_node);
        } else {
            result.cmds_mut().push(orig_node.clone());
        }
    }

    result
}
