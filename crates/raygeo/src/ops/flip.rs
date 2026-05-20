use super::container::Ops;
use super::types::{MoveCmd, OpCategory, OpNode};

pub fn flip_ops(ops: &Ops) -> Ops {
    let moving_indices: Vec<usize> = (0..ops.len())
        .filter(|&i| ops.commands[i].is_moving())
        .collect();

    if moving_indices.is_empty() {
        return ops.copy();
    }

    let mut result = Ops::new();

    let last_moving_end =
        ops.endpoint(moving_indices[moving_indices.len() - 1]);
    let first_state = ops.state(moving_indices[0]).cloned();

    let mut first_cmd = OpNode::move_to(
        last_moving_end.0,
        last_moving_end.1,
        last_moving_end.2,
        None,
    );
    if let Some(ref s) = first_state {
        first_cmd.set_state(s.clone());
    }
    result.commands.push(first_cmd);

    for k in (0..moving_indices.len() - 1).rev() {
        let orig_k_idx = moving_indices[k + 1];
        let orig_prev_idx = moving_indices[k];
        let new_end = ops.endpoint(orig_prev_idx);
        let orig_node = &ops.commands[orig_k_idx];
        let extra = orig_node.extra_axes.as_deref().map(|ea| ea.to_vec());

        if let OpCategory::Moving { cmd, .. } = &orig_node.category {
            let mut new_node = match cmd {
                MoveCmd::ScanLine { power_values } => {
                    let reversed: Vec<u8> =
                        power_values.iter().rev().copied().collect();
                    OpNode::scan_to(
                        new_end.0,
                        new_end.1,
                        new_end.2,
                        Some(reversed),
                        extra,
                    )
                }
                MoveCmd::BezierTo { c1, c2 } => {
                    OpNode::bezier_to(*c2, *c1, new_end, extra)
                }
                MoveCmd::ArcTo { center, cw } => {
                    let original_start = ops.endpoint(orig_prev_idx);
                    let original_end = ops.endpoint(orig_k_idx);
                    let center_x = original_start.0 + center.0;
                    let center_y = original_start.1 + center.1;
                    let new_i = center_x - original_end.0;
                    let new_j = center_y - original_end.1;
                    OpNode::arc_to(
                        new_end.0, new_end.1, new_i, new_j, !cw, new_end.2,
                        extra,
                    )
                }
                MoveCmd::MoveTo => {
                    OpNode::move_to(new_end.0, new_end.1, new_end.2, extra)
                }
                MoveCmd::LineTo => {
                    OpNode::line_to(new_end.0, new_end.1, new_end.2, extra)
                }
                MoveCmd::QuadraticBezierTo { control } => {
                    OpNode::quadratic_bezier_to(*control, new_end, extra)
                }
            };

            if let Some(ref s) = orig_node.state {
                new_node.set_state(s.clone());
            }
            result.commands.push(new_node);
        } else {
            result.commands.push(orig_node.clone());
        }
    }

    result
}
