use super::container::Ops;
use super::enums::{CommandCategory, CommandType, SectionType};
use super::soa::SoA;

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

    for i in 0..ops.soa.len() {
        let ct = ops.soa.command_type(i);
        if ct == CommandType::OpsSectionStart {
            if !content_indices.is_empty() || !marker_indices.is_empty() {
                sections.push(OpsSection {
                    section_type: active_type,
                    marker_indices: std::mem::take(&mut marker_indices),
                    content_indices: std::mem::take(&mut content_indices),
                });
            }
            active_type = Some(ops.soa.section_type(i));
            marker_indices = vec![i];
        } else if ct == CommandType::OpsSectionEnd {
            marker_indices.push(i);
            sections.push(OpsSection {
                section_type: active_type,
                marker_indices: std::mem::take(&mut marker_indices),
                content_indices: std::mem::take(&mut content_indices),
            });
            active_type = None;
            marker_indices = Vec::new();
            content_indices = Vec::new();
        } else {
            content_indices.push(i);
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

    for i in 0..ops.soa.len() {
        let ct = ops.soa.command_type(i);
        if ct == CommandType::OpsSectionStart {
            if !content_indices.is_empty() || !marker_indices.is_empty() {
                ranges.push(OpsSectionRange {
                    section_type: active_type,
                    marker_indices: std::mem::take(&mut marker_indices),
                    content_indices: std::mem::take(&mut content_indices),
                });
            }
            active_type = Some(ops.soa.section_type(i));
            marker_indices = vec![i];
        } else if ct == CommandType::OpsSectionEnd {
            marker_indices.push(i);
            ranges.push(OpsSectionRange {
                section_type: active_type,
                marker_indices: std::mem::take(&mut marker_indices),
                content_indices: std::mem::take(&mut content_indices),
            });
            active_type = None;
            marker_indices = Vec::new();
            content_indices = Vec::new();
        } else {
            content_indices.push(i);
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

    for i in 0..ops.soa.len() {
        if segment.is_empty() {
            segment.push(i);
            continue;
        }

        if ops.is_travel(i) {
            result.push(segment);
            segment = vec![i];
        } else if ops.is_cutting(i) {
            segment.push(i);
        } else if ops.is_state(i) || ops.is_marker(i) {
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
    for i in 0..ops.soa.len() {
        if ops.soa.category(i) != CommandCategory::State {
            let args = ops.soa.deep_copy_entry(i);
            SoA::append_from_args(&mut result.soa, &args);
        }
    }
    result.invalidate_time_cache();
    result
}

pub fn group_by_state_continuity(ops: &Ops) -> Vec<Ops> {
    if ops.soa.is_empty() {
        return Vec::new();
    }

    let mut seg_indices: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();

    for i in 0..ops.soa.len() {
        if ops.is_marker(i) {
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

        let last_state = ops.soa.state(current[current.len() - 1]);
        let op_state = ops.soa.state(i);
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
            let args = ops.soa.deep_copy_entry(idx);
            SoA::append_from_args(&mut seg_ops.soa, &args);
        }
        seg_ops.invalidate_time_cache();
        result.push(seg_ops);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::enums::CommandType;
    use crate::ops::state::State;

    #[test]
    fn test_split_into_subpaths_empty() {
        let ops = Ops::new();
        assert!(split_into_subpaths(&ops).is_empty());
    }

    #[test]
    fn test_split_into_subpaths_single_move() {
        let mut ops = Ops::new();
        ops.move_to(0.0, 0.0, 0.0, None);
        let result = split_into_subpaths(&ops);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 1);
        assert!(result[0].is_travel(0));
    }

    #[test]
    fn test_split_into_subpaths_two_subpaths() {
        let mut ops = Ops::new();
        ops.move_to(0.0, 0.0, 0.0, None);
        ops.line_to(10.0, 0.0, 0.0, None);
        ops.move_to(20.0, 20.0, 0.0, None);
        ops.line_to(30.0, 30.0, 0.0, None);
        let result = split_into_subpaths(&ops);
        assert_eq!(result.len(), 2);
        assert!(result[0].is_travel(0));
        assert!(result[1].is_travel(0));
    }

    #[test]
    fn test_iter_sections_empty() {
        let ops = Ops::new();
        assert!(iter_sections(&ops).is_empty());
    }

    #[test]
    fn test_iter_sections_no_sections() {
        let mut ops = Ops::new();
        ops.move_to(0.0, 0.0, 0.0, None);
        ops.line_to(10.0, 10.0, 0.0, None);
        ops.set_power(0.5);
        let sections = iter_sections(&ops);
        assert_eq!(sections.len(), 1);
        assert!(sections[0].section_type.is_none());
        assert!(sections[0].marker_indices.is_empty());
        assert_eq!(sections[0].content_indices.len(), 3);
    }

    #[test]
    fn test_iter_section_ranges_empty() {
        let ops = Ops::new();
        assert!(iter_section_ranges(&ops).is_empty());
    }

    #[test]
    fn test_segment_indices_empty() {
        let ops = Ops::new();
        assert!(segment_indices(&ops).is_empty());
    }

    #[test]
    fn test_segment_indices_single_move() {
        let mut ops = Ops::new();
        ops.move_to(0.0, 0.0, 0.0, None);
        let indices = segment_indices(&ops);
        assert_eq!(indices.len(), 1);
        assert_eq!(indices[0].len(), 1);
        assert_eq!(ops.command_type(indices[0][0]), CommandType::MoveTo);
    }

    #[test]
    fn test_segment_indices_move_and_line() {
        let mut ops = Ops::new();
        ops.move_to(0.0, 0.0, 0.0, None);
        ops.line_to(1.0, 0.0, 0.0, None);
        let indices = segment_indices(&ops);
        assert_eq!(indices.len(), 1);
        assert_eq!(indices[0].len(), 2);
    }

    #[test]
    fn test_segment_indices_state_commands() {
        let mut ops = Ops::new();
        ops.set_power(1.0);
        ops.move_to(0.0, 0.0, 0.0, None);
        ops.line_to(1.0, 0.0, 0.0, None);
        ops.disable_air_assist();
        let indices = segment_indices(&ops);
        assert_eq!(indices.len(), 3);
        assert!(ops.is_state(indices[0][0]));
        assert!(ops.is_travel(indices[1][0]));
        assert!(ops.is_state(indices[2][0]));
    }

    #[test]
    fn test_segment_indices_path_continuity() {
        let mut ops = Ops::new();
        ops.move_to(0.0, 0.0, 0.0, None);
        ops.line_to(10.0, 0.0, 0.0, None);
        ops.line_to(10.0, 10.0, 0.0, None);
        ops.move_to(100.0, 100.0, 0.0, None);
        ops.line_to(110.0, 100.0, 0.0, None);
        let indices = segment_indices(&ops);
        assert_eq!(indices.len(), 2);
        assert_eq!(indices[0].len(), 3);
        assert!(ops.is_travel(indices[0][0]));
        assert_eq!(indices[1].len(), 2);
        assert!(ops.is_travel(indices[1][0]));
    }

    #[test]
    fn test_without_state_basic() {
        let mut ops = Ops::new();
        ops.set_power(1.0);
        ops.move_to(0.0, 0.0, 0.0, None);
        ops.set_cut_speed(800);
        ops.line_to(10.0, 0.0, 0.0, None);
        ops.enable_air_assist();

        let filtered = without_state(&ops);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered.command_type(0), CommandType::MoveTo);
        assert_eq!(filtered.command_type(1), CommandType::LineTo);
    }

    #[test]
    fn test_without_state_empty() {
        let ops = Ops::new();
        let filtered = without_state(&ops);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_without_state_no_state_commands() {
        let mut ops = Ops::new();
        ops.move_to(0.0, 0.0, 0.0, None);
        ops.line_to(10.0, 0.0, 0.0, None);
        let filtered = without_state(&ops);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_group_by_state_continuity_same_state() {
        let mut ops = Ops::new();
        ops.line_to(0.0, 0.0, 0.0, None);
        ops.line_to(1.0, 1.0, 0.0, None);
        ops.line_to(2.0, 2.0, 0.0, None);
        for i in 0..3 {
            ops.set_state_at(
                i,
                &State {
                    power: 1.0,
                    air_assist: true,
                    ..Default::default()
                },
            );
        }
        let groups = group_by_state_continuity(&ops);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 3);
    }

    #[test]
    fn test_group_by_state_continuity_state_change() {
        let mut ops = Ops::new();
        ops.line_to(0.0, 0.0, 0.0, None);
        ops.line_to(1.0, 1.0, 0.0, None);
        ops.line_to(2.0, 2.0, 0.0, None);
        ops.set_state_at(
            0,
            &State {
                power: 1.0,
                air_assist: true,
                ..Default::default()
            },
        );
        ops.set_state_at(
            1,
            &State {
                power: 1.0,
                air_assist: true,
                ..Default::default()
            },
        );
        ops.set_state_at(
            2,
            &State {
                power: 1.0,
                air_assist: false,
                ..Default::default()
            },
        );
        let groups = group_by_state_continuity(&ops);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].len(), 2);
        assert_eq!(groups[1].len(), 1);
    }

    #[test]
    fn test_group_by_state_continuity_empty() {
        let ops = Ops::new();
        assert!(group_by_state_continuity(&ops).is_empty());
    }

    #[test]
    fn test_group_by_state_continuity_with_marker() {
        let mut ops = Ops::new();
        ops.line_to(0.0, 0.0, 0.0, None);
        ops.job_start();
        ops.line_to(1.0, 1.0, 0.0, None);
        ops.set_state_at(
            0,
            &State {
                power: 1.0,
                air_assist: true,
                ..Default::default()
            },
        );
        ops.set_state_at(
            2,
            &State {
                power: 1.0,
                air_assist: true,
                ..Default::default()
            },
        );
        let groups = group_by_state_continuity(&ops);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].len(), 1);
        assert_eq!(groups[1].len(), 1);
        assert_eq!(groups[2].len(), 1);
        assert!(groups[1].is_marker(0));
    }

    #[test]
    fn test_group_by_state_continuity_multiple_changes() {
        let mut ops = Ops::new();
        let air_assist_values = [false, true, true, false, false, true];
        for (i, &air_on) in air_assist_values.iter().enumerate() {
            ops.line_to(i as f64, i as f64, 0.0, None);
            ops.set_state_at(
                i,
                &State {
                    power: 1.0,
                    air_assist: air_on,
                    ..Default::default()
                },
            );
        }
        let groups = group_by_state_continuity(&ops);
        assert_eq!(groups.len(), 4);
        assert_eq!(
            groups.iter().map(|g| g.len()).collect::<Vec<_>>(),
            vec![1, 2, 2, 1]
        );
    }
}
