use crate::ops::enums::SectionType;
use crate::ops::types::{MarkerCmd, MoveCmd, OpCategory};
use crate::ops::Ops;

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

impl Ops {
    pub fn subpath_indices(&self) -> Vec<Vec<usize>> {
        let mut subpaths: Vec<Vec<usize>> = Vec::new();
        let mut current: Vec<usize> = Vec::new();
        let mut has_move_to = false;
        for (i, node) in self.commands.iter().enumerate() {
            let is_move = matches!(
                node.category,
                OpCategory::Moving {
                    cmd: MoveCmd::MoveTo,
                    ..
                }
            );
            if is_move && has_move_to {
                subpaths.push(current);
                current = Vec::new();
            }
            if is_move {
                has_move_to = true;
            }
            current.push(i);
        }
        if !current.is_empty() {
            subpaths.push(current);
        }
        subpaths
    }

    pub fn segment_indices(&self) -> Vec<Vec<usize>> {
        let mut result: Vec<Vec<usize>> = Vec::new();
        let mut segment: Vec<usize> = Vec::new();

        for (i, node) in self.commands.iter().enumerate() {
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

    pub fn iter_sections(&self) -> Vec<OpsSection> {
        let mut sections: Vec<OpsSection> = Vec::new();
        let mut active_type: Option<SectionType> = None;
        let mut marker_indices: Vec<usize> = Vec::new();
        let mut content_indices: Vec<usize> = Vec::new();

        for (i, node) in self.commands.iter().enumerate() {
            match &node.category {
                OpCategory::Marker(MarkerCmd::OpsSectionStart {
                    section_type,
                    ..
                }) => {
                    if !content_indices.is_empty() || !marker_indices.is_empty()
                    {
                        sections.push(OpsSection {
                            section_type: active_type,
                            marker_indices: std::mem::take(&mut marker_indices),
                            content_indices: std::mem::take(
                                &mut content_indices,
                            ),
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

    pub fn iter_section_ranges(&self) -> Vec<OpsSectionRange> {
        let mut ranges: Vec<OpsSectionRange> = Vec::new();
        let mut active_type: Option<SectionType> = None;
        let mut marker_indices: Vec<usize> = Vec::new();
        let mut content_indices: Vec<usize> = Vec::new();

        for (i, node) in self.commands.iter().enumerate() {
            match &node.category {
                OpCategory::Marker(MarkerCmd::OpsSectionStart {
                    section_type,
                    ..
                }) => {
                    if !content_indices.is_empty() || !marker_indices.is_empty()
                    {
                        ranges.push(OpsSectionRange {
                            section_type: active_type,
                            marker_indices: std::mem::take(&mut marker_indices),
                            content_indices: std::mem::take(
                                &mut content_indices,
                            ),
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
}
