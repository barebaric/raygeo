use crate::error::RaygeoError;
use crate::ops::enums::{RasterMode, SectionType};
use crate::ops::types::{MarkerCmd, MoveCmd, OpCategory};
use crate::ops::Ops;

fn ops_from_indices(ops: &Ops, indices: &[usize]) -> Ops {
    let mut result = Ops::new();
    for &idx in indices {
        result.commands.push(ops.commands[idx].clone());
    }
    result.invalidate_time_cache();
    result
}

#[derive(Clone, Debug)]
pub struct OpsSection {
    pub section_type: Option<SectionType>,
    pub raster_mode: Option<RasterMode>,
    pub marker_indices: Vec<usize>,
    pub content_indices: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct OpsSectionRange {
    pub section_type: Option<SectionType>,
    pub raster_mode: Option<RasterMode>,
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
        let mut active_mode: Option<RasterMode> = None;
        let mut marker_indices: Vec<usize> = Vec::new();
        let mut content_indices: Vec<usize> = Vec::new();

        for (i, node) in self.commands.iter().enumerate() {
            match &node.category {
                OpCategory::Marker(MarkerCmd::OpsSectionStart {
                    section_type,
                    raster_mode,
                    ..
                }) => {
                    if !content_indices.is_empty() || !marker_indices.is_empty()
                    {
                        sections.push(OpsSection {
                            section_type: active_type,
                            raster_mode: active_mode,
                            marker_indices: std::mem::take(&mut marker_indices),
                            content_indices: std::mem::take(
                                &mut content_indices,
                            ),
                        });
                    }
                    active_type = Some(*section_type);
                    active_mode = *raster_mode;
                    marker_indices = vec![i];
                }
                OpCategory::Marker(MarkerCmd::OpsSectionEnd { .. }) => {
                    marker_indices.push(i);
                    sections.push(OpsSection {
                        section_type: active_type,
                        raster_mode: active_mode,
                        marker_indices: std::mem::take(&mut marker_indices),
                        content_indices: std::mem::take(&mut content_indices),
                    });
                    active_type = None;
                    active_mode = None;
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
                raster_mode: active_mode,
                marker_indices,
                content_indices,
            });
        }

        sections
    }

    pub fn iter_section_ranges(&self) -> Vec<OpsSectionRange> {
        let mut ranges: Vec<OpsSectionRange> = Vec::new();
        let mut active_type: Option<SectionType> = None;
        let mut active_mode: Option<RasterMode> = None;
        let mut marker_indices: Vec<usize> = Vec::new();
        let mut content_indices: Vec<usize> = Vec::new();

        for (i, node) in self.commands.iter().enumerate() {
            match &node.category {
                OpCategory::Marker(MarkerCmd::OpsSectionStart {
                    section_type,
                    raster_mode,
                    ..
                }) => {
                    if !content_indices.is_empty() || !marker_indices.is_empty()
                    {
                        ranges.push(OpsSectionRange {
                            section_type: active_type,
                            raster_mode: active_mode,
                            marker_indices: std::mem::take(&mut marker_indices),
                            content_indices: std::mem::take(
                                &mut content_indices,
                            ),
                        });
                    }
                    active_type = Some(*section_type);
                    active_mode = *raster_mode;
                    marker_indices = vec![i];
                }
                OpCategory::Marker(MarkerCmd::OpsSectionEnd { .. }) => {
                    marker_indices.push(i);
                    ranges.push(OpsSectionRange {
                        section_type: active_type,
                        raster_mode: active_mode,
                        marker_indices: std::mem::take(&mut marker_indices),
                        content_indices: std::mem::take(&mut content_indices),
                    });
                    active_type = None;
                    active_mode = None;
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
                raster_mode: active_mode,
                marker_indices,
                content_indices,
            });
        }

        ranges
    }

    pub fn section_ops(&self, section: &OpsSection) -> Ops {
        ops_from_indices(self, &section.content_indices)
    }

    pub fn section_range_ops(&self, range: &OpsSectionRange) -> Ops {
        ops_from_indices(self, &range.content_indices)
    }

    pub fn sections_by_type(
        &self,
        section_type: SectionType,
    ) -> Vec<OpsSection> {
        self.iter_sections()
            .into_iter()
            .filter(|s| s.section_type == Some(section_type))
            .collect()
    }

    pub fn sections_by_mode(&self, raster_mode: RasterMode) -> Vec<OpsSection> {
        self.iter_sections()
            .into_iter()
            .filter(|s| s.raster_mode == Some(raster_mode))
            .collect()
    }

    pub fn state_block_content_from_indices(
        &self,
        _marker_indices: &[usize],
        content_indices: &[usize],
    ) -> Ops {
        ops_from_indices(self, content_indices)
    }
}

#[derive(Clone, Debug)]
pub struct StateBlock {
    pub name: Option<Arc<str>>,
    pub marker_indices: Vec<usize>,
    pub content_indices: Vec<usize>,
}

impl Ops {
    /// Extract state blocks within a given section.
    pub fn state_blocks(
        &self,
        section: &OpsSection,
    ) -> Result<Vec<StateBlock>, RaygeoError> {
        let all_indices: Vec<usize> = section
            .marker_indices
            .iter()
            .chain(section.content_indices.iter())
            .copied()
            .collect();

        let mut blocks: Vec<StateBlock> = Vec::new();
        let mut current_name: Option<Arc<str>> = None;
        let mut current_markers: Vec<usize> = Vec::new();
        let mut current_content: Vec<usize> = Vec::new();
        let mut depth: i32 = 0;

        for &i in &all_indices {
            match &self.commands[i].category {
                OpCategory::Marker(MarkerCmd::StateBlockStart { name }) => {
                    if depth > 0 {
                        return Err(RaygeoError::InvalidCommand(
                            "nested StateBlockStart detected".into(),
                        ));
                    }
                    if !current_content.is_empty() {
                        return Err(RaygeoError::InvalidCommand(
                            "StateBlockStart inside section content".into(),
                        ));
                    }
                    current_name = name.clone();
                    current_markers.push(i);
                    depth += 1;
                }
                OpCategory::Marker(MarkerCmd::StateBlockEnd) => {
                    if depth == 0 {
                        return Err(RaygeoError::InvalidCommand(
                            "StateBlockEnd without matching StateBlockStart"
                                .into(),
                        ));
                    }
                    current_markers.push(i);
                    blocks.push(StateBlock {
                        name: current_name.take(),
                        marker_indices: std::mem::take(&mut current_markers),
                        content_indices: std::mem::take(&mut current_content),
                    });
                    depth -= 1;
                }
                _ => {
                    if depth > 0 {
                        current_content.push(i);
                    }
                }
            }
        }

        if depth > 0 {
            return Err(RaygeoError::InvalidCommand(
                "unclosed StateBlockStart".into(),
            ));
        }

        Ok(blocks)
    }

    /// Extract the Ops for a specific state block.
    pub fn state_block_ops(
        &self,
        _section: &OpsSection,
        block: &StateBlock,
    ) -> Ops {
        ops_from_indices(self, &block.content_indices)
    }

    /// Find state blocks by name pattern (`*` prefix match or exact).
    pub fn state_blocks_by_name(
        &self,
        section: &OpsSection,
        pattern: &str,
    ) -> Result<Vec<StateBlock>, RaygeoError> {
        let blocks = self.state_blocks(section)?;
        let is_prefix = pattern.ends_with('*');
        let match_prefix = if is_prefix {
            &pattern[..pattern.len() - 1]
        } else {
            pattern
        };
        Ok(blocks
            .into_iter()
            .filter(|b| match &b.name {
                Some(n) => {
                    if is_prefix {
                        n.as_ref().starts_with(match_prefix)
                    } else {
                        n.as_ref() == pattern
                    }
                }
                None => false,
            })
            .collect())
    }

    /// Flat convenience: all state blocks across all sections.
    pub fn state_blocks_all(&self) -> Result<Vec<StateBlock>, RaygeoError> {
        let mut result = Vec::new();
        for section in self.iter_sections() {
            if section.section_type.is_some() {
                let blocks = self.state_blocks(&section)?;
                result.extend(blocks);
            }
        }
        Ok(result)
    }
}

use std::sync::Arc;
