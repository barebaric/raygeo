use crate::ops::container::Ops;

pub fn without_state(ops: &Ops) -> Ops {
    let mut result = Ops::new();
    for node in ops.commands.iter() {
        if !node.is_state_cmd() {
            result.cmds_mut().push(node.clone());
        }
    }
    result.invalidate_time_cache();
    result
}

/// Groups by continuity of auxiliary state (coolant, air_assist,
/// head_coolant) only. For full parameter-regime grouping, use
/// `StateBlockStart`/`StateBlockEnd` markers.
pub fn group_by_auxiliary_state(ops: &Ops) -> Vec<Ops> {
    if ops.is_empty() {
        return Vec::new();
    }

    let mut seg_indices: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();

    for (i, node) in ops.commands.iter().enumerate() {
        if node.is_marker() {
            if !current.is_empty() {
                seg_indices.push(current);
            }
            seg_indices.push(vec![i]);
            current = Vec::new();
            continue;
        }

        if current.is_empty() {
            current.push(i);
            continue;
        }

        let last_state = ops.commands[current[current.len() - 1]].state();
        let op_state = node.state();
        if let (Some(ls), Some(os)) = (last_state, op_state) {
            if ls.coolant == os.coolant
                && ls.air_assist == os.air_assist
                && ls.head_coolant == os.head_coolant
            {
                current.push(i);
            } else {
                seg_indices.push(current);
                current = vec![i];
            }
        } else {
            seg_indices.push(current);
            current = vec![i];
        }
    }

    if !current.is_empty() {
        seg_indices.push(current);
    }

    let mut result = Vec::new();
    for seg in &seg_indices {
        let mut seg_ops = Ops::new();
        for &idx in seg {
            seg_ops.cmds_mut().push(ops.commands[idx].clone());
        }
        seg_ops.invalidate_time_cache();
        result.push(seg_ops);
    }
    result
}
