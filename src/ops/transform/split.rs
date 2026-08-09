use crate::ops::enums::CommandType;
use crate::ops::Ops;

impl Ops {
    pub fn sub_ops(&self, indices: &[usize]) -> Self {
        let mut result = Ops::new();
        for &i in indices {
            let cmd = self.commands[i].clone();
            result.cmds_mut().push(cmd);
        }
        result.invalidate_time_cache();
        result
    }

    pub fn split_into_subpaths(&self) -> Vec<Ops> {
        let subpath_idx = self.subpath_indices();
        let mut result = Vec::with_capacity(subpath_idx.len());
        for indices in &subpath_idx {
            result.push(self.sub_ops(indices));
        }
        result
    }

    pub fn split_at(&self, start_ct: CommandType) -> Vec<Ops> {
        let end_ct = match start_ct {
            CommandType::LayerStart => CommandType::LayerEnd,
            CommandType::WorkpieceStart => CommandType::WorkpieceEnd,
            CommandType::OpsSectionStart => CommandType::OpsSectionEnd,
            CommandType::JobStart => CommandType::JobEnd,
            _ => panic!("split_at: {start_ct} is not a paired start marker"),
        };

        if self.is_empty() {
            return Vec::new();
        }

        let mut segments: Vec<Ops> = Vec::new();
        let mut gap: Vec<usize> = Vec::new();
        let mut i = 0;

        while i < self.len() {
            if self.command_type(i) == start_ct {
                if !gap.is_empty() {
                    segments.push(self.sub_ops(&gap));
                    gap = Vec::new();
                }
                let mut pair: Vec<usize> = Vec::new();
                pair.push(i);
                i += 1;
                while i < self.len() && self.command_type(i) != end_ct {
                    pair.push(i);
                    i += 1;
                }
                if i < self.len() {
                    pair.push(i);
                    i += 1;
                }
                segments.push(self.sub_ops(&pair));
            } else {
                gap.push(i);
                i += 1;
            }
        }

        if !gap.is_empty() {
            segments.push(self.sub_ops(&gap));
        }

        segments
    }
}
