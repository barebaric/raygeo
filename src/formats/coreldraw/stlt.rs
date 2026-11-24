// Adapted from the TraceDraw project, crates/cdr/src/text.rs (read_stlt),
// MIT OR Apache-2.0, Copyright (c) 2026 TraceDraw members and contributors.
// Modified for raygeo: reduced to the style properties text conversion
// needs, tests removed (tests live under tests/).

//! The `stlt` style table of versions 7 to 15.
//!
//! The table starts with a count and a series of id/value mapping tables
//! (fills, outlines, fonts, alignments, intervals, tabs, ...), then one
//! record per style. A text paragraph names a style id; the style names a
//! font entry with the font id, size and weight of its run.

use std::collections::HashMap;

/// Coordinate units (1/254000 inch) to points.
const PT_PER_UNIT: f64 = 72.0 / 254_000.0;

/// A text style after its indirections are resolved.
#[derive(Debug, Clone, Default)]
pub(crate) struct Style {
    pub parent: u32,
    pub font_id: Option<u16>,
    pub size_pt: Option<f64>,
    pub flags: Option<u32>,
    pub align: Option<u32>,
    /// Line spacing as a share of the character height.
    pub line_spacing: Option<f64>,
}

/// A cursor over the table.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Reader { data, pos: 0 }
    }

    fn u16(&mut self) -> Option<u16> {
        let bytes = self.data.get(self.pos..self.pos + 2)?;
        self.pos += 2;
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Option<u32> {
        let bytes = self.data.get(self.pos..self.pos + 4)?;
        self.pos += 4;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn i32(&mut self) -> Option<i32> {
        self.u32().map(|value| value as i32)
    }

    fn skip(&mut self, count: usize) {
        self.pos = self.pos.saturating_add(count);
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }
}

/// Read the style records of a version 7 to 15 `stlt` body.
pub(crate) fn read_styles(data: &[u8], version: u16) -> HashMap<u32, Style> {
    let mut r = Reader::new(data);
    let Some(count) = r.u32() else {
        return HashMap::new();
    };
    if count == 0 {
        return HashMap::new();
    }
    read_entries(&mut r, if version >= 13 { 48 } else { 0 });
    read_entries(&mut r, 0);
    let fonts = read_fonts(&mut r, version);
    let aligns = read_entries(&mut r, 0);
    let intervals = read_intervals(&mut r);
    skip_table(&mut r, 152);
    skip_table(&mut r, 784);
    skip_bullets(&mut r, version);
    skip_table(&mut r, 4 + 12 + 12);
    skip_table(&mut r, 32 + if version >= 13 { 4 } else { 0 });
    skip_table(&mut r, 28);
    if version >= 9 {
        skip_table(&mut r, 12);
    }
    let mut styles = HashMap::new();
    for _ in 0..count.min(65_536) {
        let Some(groups) = r.u32() else { break };
        let Some(style_id) = r.u32() else { break };
        let Some(parent) = r.u32() else { break };
        r.skip(8);
        let Some(name_len) = r.u32() else { break };
        let bytes = name_len as usize * if version >= 12 { 2 } else { 1 };
        if bytes > r.remaining() {
            break;
        }
        r.skip(bytes);
        if r.u32().is_none() || r.u32().is_none() {
            break;
        }
        let mut style = Style {
            parent,
            ..Style::default()
        };
        if groups > 1 {
            let (Some(font_ref), Some(align_ref)) = (r.u32(), r.u32()) else {
                break;
            };
            let Some(interval_ref) = r.u32() else { break };
            if r.u32().is_none() {
                break;
            }
            if version >= 9 {
                r.skip(4);
            }
            if let Some((font_id, size_pt, flags)) = fonts.get(&font_ref) {
                style.font_id = Some(*font_id);
                if *size_pt > 0.0 {
                    style.size_pt = Some(*size_pt);
                }
                style.flags = Some(*flags);
            }
            style.align = aligns.get(&align_ref).copied();
            if let Some(spacing) = intervals.get(&interval_ref) {
                style.line_spacing = Some(*spacing);
            }
        }
        if groups > 2 {
            r.skip(20);
        }
        styles.insert(style_id, style);
    }
    styles
}

/// Entries of the shape `id u32, 4 bytes, value u32, tail`.
fn read_entries(r: &mut Reader, extra: usize) -> HashMap<u32, u32> {
    let Some(count) = r.u32() else {
        return HashMap::new();
    };
    let size = 12 + extra;
    let count = (count as usize).min(r.remaining() / size.max(1));
    let mut entries = HashMap::with_capacity(count);
    for _ in 0..count {
        let Some(id) = r.u32() else { break };
        r.skip(4);
        let Some(value) = r.u32() else { break };
        r.skip(extra);
        entries.insert(id, value);
    }
    entries
}

/// Font entries: `id`, unknown bytes, three scripts (font id and
/// encoding each), their sizes (one for all before version 10) and their
/// style flags.
fn read_fonts(r: &mut Reader, version: u16) -> HashMap<u32, (u16, f64, u32)> {
    let gap = if version < 10 { 12 } else { 20 };
    let size_slots = if version >= 10 { 3 } else { 1 };
    let entry_size = 4 + gap + 3 * 4 + size_slots * 4 + 12;
    let Some(count) = r.u32() else {
        return HashMap::new();
    };
    let count = (count as usize).min(r.remaining() / entry_size.max(1));
    let mut fonts = HashMap::with_capacity(count);
    for _ in 0..count {
        let Some(id) = r.u32() else { break };
        r.skip(gap);
        let mut ids = [(0u16, 0u16); 3];
        let mut ok = true;
        for slot in &mut ids {
            match (r.u16(), r.u16()) {
                (Some(font_id), Some(encoding)) => *slot = (font_id, encoding),
                _ => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            break;
        }
        let mut sizes = [0.0_f64; 3];
        ok = true;
        for size in sizes.iter_mut().take(size_slots) {
            match r.i32() {
                Some(value) => *size = f64::from(value) * PT_PER_UNIT,
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            break;
        }
        if size_slots == 1 {
            sizes = [sizes[0]; 3];
        }
        let mut flags = [0u32; 3];
        ok = true;
        for flag in &mut flags {
            match r.u32() {
                Some(value) => *flag = value,
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            break;
        }
        fonts.insert(id, (ids[0].0, sizes[0], flags[0]));
    }
    fonts
}

/// Interval entries: the line spacing as a share of the character height
/// when the unit says so.
fn read_intervals(r: &mut Reader) -> HashMap<u32, f64> {
    let Some(count) = r.u32() else {
        return HashMap::new();
    };
    let count = (count as usize).min(r.remaining() / 52);
    let mut intervals = HashMap::with_capacity(count);
    for _ in 0..count {
        let Some(id) = r.u32() else { break };
        r.skip(8);
        if r.u32().is_none() {
            break;
        }
        r.skip(4);
        if r.u32().is_none() {
            break;
        }
        let Some(line) = r.u32() else { break };
        r.skip(20);
        let Some(unit) = r.u32() else { break };
        if unit != 1 {
            let factor = f64::from(line as i32) / 1_000_000.0;
            if factor > 0.0 && factor <= 20.0 {
                intervals.insert(id, factor);
            }
        }
    }
    intervals
}

fn skip_table(r: &mut Reader, size: usize) {
    let Some(count) = r.u32() else { return };
    let count = (count as usize).min(r.remaining() / size.max(1));
    r.skip(count * size);
}

fn skip_bullets(r: &mut Reader, version: u16) {
    let Some(count) = r.u32() else { return };
    for _ in 0..(count as usize).min(4096) {
        if r.remaining() == 0 {
            break;
        }
        r.skip(40);
        if version >= 14 {
            r.skip(4);
        }
        if version >= 13 {
            let Some(indirect) = r.u32() else { break };
            r.skip(if indirect != 0 { 68 } else { 12 });
        } else {
            r.skip(20);
            if version >= 10 {
                r.skip(8);
            }
            let Some(indirect) = r.u32() else { break };
            if indirect != 0 {
                r.skip(8);
            }
            r.skip(8);
        }
    }
}
