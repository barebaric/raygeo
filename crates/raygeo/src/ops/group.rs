use super::container::Ops;
use super::enums::SectionType;
use super::types::{MarkerCmd, MoveCmd, OpCategory};

#[derive(Clone, Debug)]
pub struct OpsSection {
    pub section_type: Option<SectionType>,
    pub marker_indices: Vec<usize>,
    pub content_indices: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct OpsSectionRange {
    pub section_type: Option<SectionType>,
    pub marker_indices: Vec<usize>,
    pub content_indices: Vec<usize>,
}

pub fn split_into_subpaths(ops: &Ops) -> Vec<Ops> {
    let subpath_idx = ops.subpath_indices();
    let mut result = Vec::with_capacity(subpath_idx.len());
    for indices in &subpath_idx {
        result.push(ops.sub_ops(indices));
    }
    result
}

pub fn iter_sections(ops: &Ops) -> Vec<OpsSection> {
    let mut sections: Vec<OpsSection> = Vec::new();
    let mut active_type: Option<SectionType> = None;
    let mut marker_indices: Vec<usize> = Vec::new();
    let mut content_indices: Vec<usize> = Vec::new();

    for (i, node) in ops.commands.iter().enumerate() {
        match &node.category {
            OpCategory::Marker(MarkerCmd::OpsSectionStart {
                section_type,
                ..
            }) => {
                if !content_indices.is_empty() || !marker_indices.is_empty() {
                    sections.push(OpsSection {
                        section_type: active_type,
                        marker_indices: std::mem::take(&mut marker_indices),
                        content_indices: std::mem::take(&mut content_indices),
                    });
                }
                active_type = Some(*section_type);
                marker_indices = vec![i];
            }
            OpCategory::Marker(MarkerCmd::OpsSectionEnd { .. }) => {
                marker_indices.push(i);
                sections.push(OpsSection {
                    section_type: active_type,
                    marker_indices: std::mem::take(&mut marker_indices),
                    content_indices: std::mem::take(&mut content_indices),
                });
                active_type = None;
                marker_indices = Vec::new();
                content_indices = Vec::new();
            }
            _ => {
                content_indices.push(i);
            }
        }
    }

    if !content_indices.is_empty() || !marker_indices.is_empty() {
        sections.push(OpsSection {
            section_type: active_type,
            marker_indices,
            content_indices,
        });
    }

    sections
}

pub fn iter_section_ranges(ops: &Ops) -> Vec<OpsSectionRange> {
    let mut ranges: Vec<OpsSectionRange> = Vec::new();
    let mut active_type: Option<SectionType> = None;
    let mut marker_indices: Vec<usize> = Vec::new();
    let mut content_indices: Vec<usize> = Vec::new();

    for (i, node) in ops.commands.iter().enumerate() {
        match &node.category {
            OpCategory::Marker(MarkerCmd::OpsSectionStart {
                section_type,
                ..
            }) => {
                if !content_indices.is_empty() || !marker_indices.is_empty() {
                    ranges.push(OpsSectionRange {
                        section_type: active_type,
                        marker_indices: std::mem::take(&mut marker_indices),
                        content_indices: std::mem::take(&mut content_indices),
                    });
                }
                active_type = Some(*section_type);
                marker_indices = vec![i];
            }
            OpCategory::Marker(MarkerCmd::OpsSectionEnd { .. }) => {
                marker_indices.push(i);
                ranges.push(OpsSectionRange {
                    section_type: active_type,
                    marker_indices: std::mem::take(&mut marker_indices),
                    content_indices: std::mem::take(&mut content_indices),
                });
                active_type = None;
                marker_indices = Vec::new();
                content_indices = Vec::new();
            }
            _ => {
                content_indices.push(i);
            }
        }
    }

    if !content_indices.is_empty() || !marker_indices.is_empty() {
        ranges.push(OpsSectionRange {
            section_type: active_type,
            marker_indices,
            content_indices,
        });
    }

    ranges
}

pub fn segment_indices(ops: &Ops) -> Vec<Vec<usize>> {
    let mut result: Vec<Vec<usize>> = Vec::new();
    let mut segment: Vec<usize> = Vec::new();

    for (i, node) in ops.commands.iter().enumerate() {
        if segment.is_empty() {
            segment.push(i);
            continue;
        }

        if let OpCategory::Moving {
            cmd: MoveCmd::MoveTo,
            ..
        } = &node.category
        {
            result.push(segment);
            segment = vec![i];
        } else if node.is_moving() {
            segment.push(i);
        } else {
            result.push(segment);
            result.push(vec![i]);
            segment = Vec::new();
        }
    }

    if !segment.is_empty() {
        result.push(segment);
    }

    result
}

pub fn segments(ops: &Ops) -> Vec<Vec<usize>> {
    segment_indices(ops)
}

pub fn without_state(ops: &Ops) -> Ops {
    let mut result = Ops::new();
    for node in &ops.commands {
        if !node.is_state_cmd() {
            result.commands.push(node.clone());
        }
    }
    result.invalidate_time_cache();
    result
}

pub fn group_by_state_continuity(ops: &Ops) -> Vec<Ops> {
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
            if ls.air_assist == os.air_assist {
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
            seg_ops.commands.push(ops.commands[idx].clone());
        }
        seg_ops.invalidate_time_cache();
        result.push(seg_ops);
    }
    result
}
