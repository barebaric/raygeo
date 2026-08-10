use std::sync::Arc;

use crate::ops::container::Ops;

pub fn without_state(ops: &Ops) -> Ops {
    let mut result = Ops::new();
    result.cmds_mut().reserve(ops.commands.len());
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
    let n = ops.commands.len();
    if n == 0 {
        return Vec::new();
    }

    // Segments are contiguous index runs; track them as ranges so no
    // per-segment index Vec is allocated (raster jobs can produce
    // hundreds of thousands of segments).
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut start = 0usize;

    for i in 0..n {
        let node = &ops.commands[i];
        if node.is_marker() {
            if i > start {
                ranges.push((start, i));
            }
            ranges.push((i, i + 1));
            start = i + 1;
            continue;
        }
        if i > start {
            let same = match (ops.commands[i - 1].state(), node.state()) {
                (Some(ls), Some(os)) => {
                    ls.coolant == os.coolant
                        && ls.air_assist == os.air_assist
                        && ls.head_coolant == os.head_coolant
                }
                _ => false,
            };
            if !same {
                ranges.push((start, i));
                start = i;
            }
        }
    }
    if start < n {
        ranges.push((start, n));
    }

    let mut result = Vec::with_capacity(ranges.len());
    for (start, end) in ranges {
        let mut seg_ops = Ops::new();
        seg_ops.commands = Arc::new(ops.commands[start..end].to_vec());
        seg_ops.invalidate_time_cache();
        result.push(seg_ops);
    }
    result
}
