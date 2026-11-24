// Adapted from the TraceDraw project, crates/cdr/src/text.rs (text
// record readers), MIT OR Apache-2.0, Copyright (c) 2026 TraceDraw
// members and contributors. Modified for raygeo: reduced to runs and
// styles, tests removed (tests live under tests/).

//! Text records (`txsm`) and their conversion to glyph outlines.
//!
//! A text object keeps its content in a `txsm` chunk: paragraphs made of
//! styled runs. Versions 7 to 15 name a style id that the `stlt` table
//! resolves to a font entry; version 16 and later carry JSON style
//! strings beside the runs. Both layouts are reduced to runs of text with
//! a font, and drawn as glyph outlines with [`crate::geo::shape::text`].
//!
//! The layout of both records is the one documented in TraceDraw's
//! clean-room `docs/cdr-format.md` ("Text and styles").

use std::collections::HashMap;

use serde_json::Value;

use crate::formats::coreldraw::stlt::Style;
use crate::geo::geometry::Geometry;
use crate::geo::shape::text::{get_text_width, text_to_geometry, FontConfig};

/// Coordinate units (1/254000 inch) to points.
const PT_PER_UNIT: f64 = 72.0 / 254_000.0;
/// Millimetres per inch.
const MM_PER_INCH: f64 = 25.4;
/// Line advance as a share of the font size when the style gives none.
const DEFAULT_LINE_FACTOR: f64 = 1.2;

/// One styled run of text.
#[derive(Debug, Clone, Default)]
pub(crate) struct TextStyle {
    pub font_id: Option<u16>,
    pub family: Option<String>,
    pub size_pt: Option<f64>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub align: Option<u32>,
    pub line_spacing: Option<f64>,
}

impl TextStyle {
    /// `self` over `base`: the fields `self` sets win.
    fn over(self, base: &TextStyle) -> TextStyle {
        TextStyle {
            font_id: self.font_id.or(base.font_id),
            family: self.family.or_else(|| base.family.clone()),
            size_pt: self.size_pt.or(base.size_pt),
            bold: self.bold.or(base.bold),
            italic: self.italic.or(base.italic),
            align: self.align.or(base.align),
            line_spacing: self.line_spacing.or(base.line_spacing),
        }
    }
}

/// One run of a paragraph.
#[derive(Debug, Clone, Default)]
pub(crate) struct Run {
    pub text: String,
    pub style: TextStyle,
}

/// One paragraph of a text object.
#[derive(Debug, Clone, Default)]
pub(crate) struct Paragraph {
    pub style_id: u32,
    pub base: TextStyle,
    pub runs: Vec<Run>,
}

/// The text of a `txsm` record.
#[derive(Debug, Clone, Default)]
pub(crate) struct Text {
    pub paragraphs: Vec<Paragraph>,
}

/// Reads a `txsm` payload of the given version.
pub(crate) fn read_text(data: &[u8], version: u16) -> Option<Text> {
    if version < 7 {
        return None;
    }
    if version < 16 {
        read_15(data, version)
    } else {
        read_16(data, version)
    }
}

/// A cursor over a `txsm` record.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Reader { data, pos: 0 }
    }

    fn u8(&mut self) -> Option<u8> {
        let value = *self.data.get(self.pos)?;
        self.pos += 1;
        Some(value)
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

    fn bytes(&mut self, count: usize) -> Option<&'a [u8]> {
        let bytes = self.data.get(self.pos..self.pos.checked_add(count)?)?;
        self.pos += count;
        Some(bytes)
    }

    fn skip(&mut self, count: usize) {
        self.pos = self.pos.saturating_add(count);
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }
}

/// The most paragraphs, runs and characters read from one record.
const MAX_PARAGRAPHS: usize = 4096;
const MAX_RUNS: usize = 1024;
const MAX_CHARS: usize = 1_000_000;
const MAX_FRAMES: usize = 1024;

/// Reads a version 7 to 15 record.
fn read_15(data: &[u8], version: u16) -> Option<Text> {
    let mut r = Reader::new(data);
    let frame = r.u32()? != 0;
    r.skip(32);
    if version >= 15 {
        r.skip(1);
    }
    if version < 8 && r.u32()? != 0 {
        r.skip(32);
    }
    let frame_count = r.u32()? as usize;
    if frame_count > MAX_FRAMES {
        return None;
    }
    for _ in 0..frame_count {
        r.u32()?;
        r.skip(48);
        if version >= 8 {
            if r.u32()? != 0 {
                r.skip(4);
                if version >= 13 {
                    r.skip(8);
                }
                r.skip(28);
                if version >= 15 {
                    r.skip(8);
                }
            } else if version >= 15 {
                r.skip(8);
            }
        }
        if !frame {
            r.skip(if version >= 15 {
                40
            } else if version >= 14 {
                36
            } else if version > 8 {
                34
            } else if version == 8 {
                32
            } else {
                36
            });
        } else if version >= 15 {
            r.skip(4);
        }
    }
    let paragraph_count = r.u32()? as usize;
    if paragraph_count > MAX_PARAGRAPHS {
        return None;
    }
    let mut text = Text::default();
    for _ in 0..paragraph_count {
        let style_id = r.u32()?;
        r.skip(1);
        if version >= 13 && frame && r.u8()? == 1 {
            r.skip(64);
        }
        let run_count = r.u32()? as usize;
        if run_count > MAX_RUNS {
            return None;
        }
        let mut run_styles: Vec<(usize, TextStyle)> =
            Vec::with_capacity(run_count);
        for _ in 0..run_count {
            let letters = r.u16()? as usize;
            let flags = r.u8()?;
            let extra = if version >= 8 { r.u8()? } else { 0 };
            let mut style = TextStyle::default();
            if flags & 0x01 != 0 {
                style.font_id = Some(r.u16()?);
                r.u16()?;
            }
            if flags & 0x02 != 0 {
                let weight = r.u32()?;
                style.bold = Some(weight & 0x1000 != 0);
                style.italic = Some(weight & 0x80 != 0);
            }
            if flags & 0x04 != 0 {
                let size = f64::from(r.i32()?) * PT_PER_UNIT;
                if size > 0.0 {
                    style.size_pt = Some(size);
                }
            }
            for bit in [0x08, 0x10, 0x20] {
                if flags & bit != 0 {
                    r.i32()?;
                }
            }
            if flags & 0x40 != 0 {
                r.u32()?;
                if version >= 13 {
                    r.skip(48);
                }
            }
            if flags & 0x80 != 0 {
                r.u32()?;
            }
            if extra & 0x02 != 0 {
                let length = r.u32()? as usize;
                r.skip(length.saturating_mul(2));
            }
            if extra & 0x08 != 0 {
                if version < 13 {
                    r.skip(4);
                } else {
                    let length = r.u32()? as usize;
                    r.skip(length.saturating_mul(2));
                }
            }
            if extra & 0x20 != 0 {
                let set = *r.data.get(r.pos)?;
                if version >= 15 {
                    r.skip(if set != 0 { 53 } else { 1 });
                } else if set != 0 {
                    r.skip(4);
                }
            }
            run_styles.push((letters, style));
        }
        let char_count = r.u32()? as usize;
        if char_count > MAX_CHARS {
            return None;
        }
        let mut wide = Vec::with_capacity(char_count.min(MAX_CHARS));
        let mut style_of = Vec::with_capacity(char_count.min(MAX_CHARS));
        if version >= 12 {
            if char_count > r.remaining() / 8 {
                return None;
            }
            for _ in 0..char_count {
                let first = r.u8()?;
                r.skip(1);
                let style = r.u8()?;
                r.skip(5);
                wide.push(first & 0x01 != 0);
                style_of.push((style >> 1) as usize);
            }
        } else {
            if char_count > r.remaining() / 4 {
                return None;
            }
            for _ in 0..char_count {
                r.skip(2);
                let style = r.u8()?;
                r.skip(1);
                style_of.push((style >> 1) as usize);
                wide.push(false);
            }
        }
        let byte_count = if version >= 12 {
            r.u32()? as usize
        } else {
            char_count
        };
        let bytes = r.bytes(byte_count)?;
        if r.u8()? != 0 {
            r.skip(char_count.saturating_mul(24));
        }
        let characters = decode_characters(bytes, &wide);
        let mut runs = Vec::new();
        if style_of.iter().any(|index| *index != 0)
            && style_of.iter().all(|index| *index < run_styles.len())
        {
            let mut start = 0;
            let mut current = style_of.first().copied().unwrap_or(0);
            for index in 1..=style_of.len() {
                let next = style_of.get(index).copied();
                if next != Some(current) || index == style_of.len() {
                    let text: String = characters
                        .get(start..index)
                        .unwrap_or(&[])
                        .iter()
                        .collect();
                    runs.push(Run {
                        text,
                        style: run_style(&run_styles, current),
                    });
                    start = index;
                    current = next.unwrap_or(0);
                }
            }
        } else {
            let mut start = 0;
            for (letters, style) in &run_styles {
                let end = (start + letters).min(characters.len());
                let text: String =
                    characters.get(start..end).unwrap_or(&[]).iter().collect();
                if !text.is_empty() {
                    runs.push(Run {
                        text,
                        style: style.clone(),
                    });
                }
                start = end;
            }
            if start < characters.len() {
                let text: String = characters[start..].iter().collect();
                runs.push(Run {
                    text,
                    style: TextStyle::default(),
                });
            }
        }
        text.paragraphs.push(Paragraph {
            style_id,
            base: TextStyle::default(),
            runs,
        });
    }
    Some(text)
}

fn run_style(run_styles: &[(usize, TextStyle)], index: usize) -> TextStyle {
    run_styles
        .get(index)
        .map(|(_, style)| style.clone())
        .unwrap_or_default()
}

/// Decodes the characters of a run: one byte each unless a character is
/// flagged as a UTF-16LE unit; a text whose bytes are all two-byte units
/// is UTF-16 throughout.
fn decode_characters(bytes: &[u8], wide: &[bool]) -> Vec<char> {
    let any_wide = wide.iter().any(|flag| *flag);
    if !any_wide && bytes.len() == wide.len() * 2 {
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .collect();
        return char::decode_utf16(units)
            .map(|unit| unit.unwrap_or('\u{FFFD}'))
            .collect();
    }
    let mut characters = Vec::with_capacity(wide.len());
    let mut at = 0;
    for flag in wide {
        if *flag {
            let Some(pair) = bytes.get(at..at + 2) else {
                break;
            };
            let unit = u16::from_le_bytes([pair[0], pair[1]]);
            characters.push(
                char::decode_utf16([unit])
                    .next()
                    .and_then(Result::ok)
                    .unwrap_or('\u{FFFD}'),
            );
            at += 2;
        } else {
            let Some(byte) = bytes.get(at) else { break };
            characters.push(char::from(*byte));
            at += 1;
        }
    }
    characters
}

/// Reads a version 16 and later record.
fn read_16(data: &[u8], version: u16) -> Option<Text> {
    let mut r = Reader::new(data);
    let frame = r.u32()? != 0;
    r.skip(32);
    let layout = r.u16()?;
    r.skip(3);
    let frame_count = r.u32()? as usize;
    if frame_count > MAX_FRAMES {
        return None;
    }
    for _ in 0..frame_count {
        r.u32()?;
        r.skip(48);
        if r.u32()? != 0 {
            r.skip(40);
        }
        r.skip(8);
        if !frame {
            r.skip(16);
            let length = r.u32()? as usize;
            r.skip(length.saturating_mul(if version >= 17 { 1 } else { 2 }));
        }
    }
    let paragraph_count = r.u32()? as usize;
    if paragraph_count > MAX_PARAGRAPHS {
        return None;
    }
    let mut text = Text::default();
    for _ in 0..paragraph_count {
        let style_id = r.u32()?;
        r.skip(1);
        if r.u8()? == 1 {
            r.skip(64);
        }
        let mut base = TextStyle::default();
        if layout < 1700 && !frame && string_starts(r.data, r.pos) {
            base = read_style_string(&mut r)?;
        }
        let default_style = read_style_string(&mut r)?;
        base = default_style.over(&base);
        let record_count = r.u32()? as usize;
        if record_count > MAX_RUNS {
            return None;
        }
        let mut records: Vec<(usize, TextStyle)> =
            Vec::with_capacity(record_count);
        for _ in 0..record_count {
            let letters = r.u16()? as usize;
            let members = r.u16()?;
            let flags = r.u16()?;
            if members == 0x3fff && flags & 0x11 != 0 {
                let length = r.u32()? as usize;
                r.skip(length.saturating_mul(2));
            }
            if flags & 0x04 != 0 {
                let length = r.u32()? as usize;
                r.skip(length.saturating_mul(2));
            }
            if flags & 0x08 != 0 {
                r.u16()?;
                let length = r.u16()? as usize;
                r.skip(length.saturating_mul(2));
            }
            if flags & 0x02 != 0 {
                r.u16()?;
                let length = r.u32()? as usize;
                r.skip(length);
            }
            let style = read_style_string(&mut r)?;
            records.push((letters, style));
        }
        let char_count = r.u32()? as usize;
        if char_count > MAX_CHARS {
            return None;
        }
        if char_count > r.remaining() / 8 {
            return None;
        }
        let mut wide = Vec::with_capacity(char_count);
        let mut override_of = Vec::with_capacity(char_count);
        for _ in 0..char_count {
            let flags = r.u16()?;
            let over = r.u8()?;
            r.skip(5);
            wide.push(flags & 0x01 != 0);
            override_of.push((over >> 1) as usize);
        }
        let byte_count = r.u32()? as usize;
        let bytes = r.bytes(byte_count)?;
        if r.u8()? != 0 {
            r.skip(char_count.saturating_mul(24));
        }
        let characters = decode_characters(bytes, &wide);
        let mut runs = Vec::new();
        if override_of.iter().any(|index| *index != 0) {
            let mut start = 0;
            let mut current = override_of.first().copied().unwrap_or(0);
            for index in 1..=override_of.len() {
                let next = override_of.get(index).copied();
                if next != Some(current) || index == override_of.len() {
                    let text: String = characters
                        .get(start..index)
                        .unwrap_or(&[])
                        .iter()
                        .collect();
                    let style = records
                        .get(current)
                        .map(|(_, style)| style.clone())
                        .unwrap_or_default()
                        .over(&base);
                    runs.push(Run { text, style });
                    start = index;
                    current = next.unwrap_or(0);
                }
            }
        } else {
            let mut start = 0;
            for (letters, style) in &records {
                let end = (start + letters).min(characters.len());
                let text: String =
                    characters.get(start..end).unwrap_or(&[]).iter().collect();
                if !text.is_empty() {
                    runs.push(Run {
                        text,
                        style: style.clone().over(&base),
                    });
                }
                start = end;
            }
            if start < characters.len() {
                let text: String = characters[start..].iter().collect();
                runs.push(Run {
                    text,
                    style: base.clone(),
                });
            }
        }
        text.paragraphs.push(Paragraph {
            style_id,
            base,
            runs,
        });
    }
    Some(text)
}

fn string_starts(data: &[u8], at: usize) -> bool {
    let Some(length) = data.get(at..at + 4) else {
        return false;
    };
    let length =
        u32::from_le_bytes([length[0], length[1], length[2], length[3]]);
    length as usize <= data.len().saturating_sub(at + 4)
        && data.get(at + 4) == Some(&b'{')
}

/// A style string: a u32 length, then the text.
fn read_style_string(r: &mut Reader) -> Option<TextStyle> {
    let length = r.u32()? as usize;
    let bytes = r.bytes(length)?;
    let string = String::from_utf8_lossy(bytes);
    Some(parse_style_string(&string))
}

/// The members of a style string this reader uses, matched loosely so
/// both the JSON spelling and plain key/value text work.
fn parse_style_string(text: &str) -> TextStyle {
    let mut style = TextStyle::default();
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        apply_style_object(&mut style, &value);
    }
    style
}

fn apply_style_object(style: &mut TextStyle, value: &Value) {
    let Value::Object(map) = value else {
        return;
    };
    for (key, value) in map {
        let key = key.to_ascii_lowercase();
        match value {
            Value::Object(_) | Value::Array(_) => {
                apply_style_object(style, value)
            }
            _ => apply_style_member(style, &key, value),
        }
    }
}

fn apply_style_member(style: &mut TextStyle, key: &str, value: &Value) {
    let number = match value {
        Value::String(text) => text.trim().parse::<f64>().ok(),
        Value::Number(number) => number.as_f64(),
        _ => None,
    };
    if key.contains("size") {
        if let Some(pt) = number.and_then(style_size_pt) {
            style.size_pt = Some(pt);
        }
    } else if key.contains("weight") {
        if let Some(weight) = number {
            style.bold =
                Some(weight >= 1024.0 || (600.0..=1000.0).contains(&weight));
        }
    } else if key.contains("italic") || key == "style" {
        match value {
            Value::String(text) => {
                let text = text.to_ascii_lowercase();
                if text.contains("italic") || text.contains("oblique") {
                    style.italic = Some(true);
                }
            }
            _ => {
                if let Some(italic) = number {
                    style.italic = Some(italic != 0.0);
                }
            }
        }
    } else if key.contains("justif") || key.contains("align") {
        let align = match value {
            Value::String(text) if text.trim().parse::<f64>().is_err() => {
                let text = text.to_ascii_lowercase();
                Some(if text.contains("cent") {
                    2
                } else if text.contains("right") {
                    3
                } else if text.contains("just") {
                    4
                } else {
                    1
                })
            }
            _ => number.map(|n| n as u32),
        };
        if let Some(align) = align.filter(|align| *align <= 5) {
            style.align = Some(if align == 0 { 1 } else { align });
        }
    } else if key.contains("font")
        || key.contains("family")
        || key.contains("face")
    {
        if let Value::String(name) = value {
            let name = name.trim();
            if !name.is_empty() && name.parse::<f64>().is_err() {
                style.family = Some(name.to_string());
            }
        }
    }
}

/// Font size as a style string stores it: values above 1000 are
/// coordinate units, smaller ones points.
fn style_size_pt(size: f64) -> Option<f64> {
    if !size.is_finite() || size <= 0.0 {
        return None;
    }
    Some(if size > 1000.0 {
        size * PT_PER_UNIT
    } else {
        size
    })
}

/// Draws the text as glyph outlines in inches, the first baseline at
/// `origin`, Y up.
pub(crate) fn text_geometry(
    text: &Text,
    styles: &HashMap<u32, Style>,
    fonts: &HashMap<u16, String>,
    origin: (f64, f64),
) -> Geometry {
    let mut geometry = Geometry::new();
    let mut baseline = 0.0_f64;
    for paragraph in &text.paragraphs {
        let from_table = paragraph_base(paragraph.style_id, styles, fonts);
        let base = paragraph.base.clone().over(&from_table);
        let mut lines: Vec<Vec<Run>> = vec![Vec::new()];
        for run in &paragraph.runs {
            let mut parts = run.text.split('\n');
            if let Some(first) = parts.next() {
                if !first.is_empty() {
                    lines.last_mut().unwrap().push(Run {
                        text: first.trim_end_matches('\r').to_string(),
                        style: run.style.clone().over(&base),
                    });
                }
            }
            for part in parts {
                lines.push(Vec::new());
                if !part.is_empty() {
                    lines.last_mut().unwrap().push(Run {
                        text: part.trim_end_matches('\r').to_string(),
                        style: run.style.clone().over(&base),
                    });
                }
            }
        }
        let align = base.align.unwrap_or(1);
        for line in &lines {
            let mut glyphs: Vec<(Geometry, f64, f64)> = Vec::new();
            let mut line_width = 0.0;
            for run in line {
                let config = font_for(&run.style, base.family.clone(), fonts);
                let width = get_text_width(&run.text, &config).unwrap_or(0.0);
                if let Some(mut glyph_geometry) =
                    text_to_geometry(&run.text, &config)
                {
                    glyph_geometry.transform_2d(
                        1.0 / MM_PER_INCH,
                        0.0,
                        0.0,
                        1.0 / MM_PER_INCH,
                        0.0,
                        0.0,
                    );
                    glyphs.push((glyph_geometry, line_width, width));
                }
                line_width += width;
            }
            let line_width = line_width / MM_PER_INCH;
            let start = match align {
                2 => -line_width / 2.0,
                3 => -line_width,
                _ => 0.0,
            };
            for (mut glyph_geometry, x, _) in glyphs {
                glyph_geometry.transform_2d(
                    1.0,
                    0.0,
                    0.0,
                    1.0,
                    start + x / MM_PER_INCH,
                    baseline,
                );
                geometry.extend(&glyph_geometry);
            }
            let size = base.size_pt.unwrap_or(10.0);
            let factor = base.line_spacing.unwrap_or(DEFAULT_LINE_FACTOR);
            baseline -= factor * size * MM_PER_INCH / 72.0;
        }
    }
    if !geometry.is_empty() {
        geometry.transform_2d(1.0, 0.0, 0.0, 1.0, origin.0, origin.1);
    }
    geometry
}

/// The paragraph's style as the `stlt` table and its parent chain give it.
fn paragraph_base(
    style_id: u32,
    styles: &HashMap<u32, Style>,
    fonts: &HashMap<u16, String>,
) -> TextStyle {
    let mut chain = Vec::new();
    let mut current = Some(style_id);
    let mut guard = 0;
    while let Some(id) = current {
        if guard > 32 {
            break;
        }
        guard += 1;
        let Some(style) = styles.get(&id) else { break };
        chain.push(style);
        current =
            (style.parent != 0 && style.parent != id).then_some(style.parent);
    }
    let mut base = TextStyle::default();
    for style in chain.into_iter().rev() {
        let mut resolved = TextStyle {
            font_id: style.font_id,
            size_pt: style.size_pt,
            align: style.align,
            line_spacing: style.line_spacing,
            ..TextStyle::default()
        };
        if let Some(flags) = style.flags {
            resolved.bold = Some(flags & 0x1000 != 0);
            resolved.italic = Some(flags & 0x80 != 0);
        }
        if resolved.family.is_none() {
            resolved.family =
                resolved.font_id.and_then(|id| fonts.get(&id).cloned());
        }
        base = resolved.over(&base);
    }
    base
}

/// The font to draw a run with.
fn font_for(
    style: &TextStyle,
    base_family: Option<String>,
    fonts: &HashMap<u16, String>,
) -> FontConfig {
    let family = style
        .family
        .clone()
        .or_else(|| base_family.clone())
        .or_else(|| style.font_id.and_then(|id| fonts.get(&id).cloned()))
        .unwrap_or_else(|| "sans-serif".to_string());
    let size = style.size_pt.unwrap_or(10.0);
    FontConfig::new(&family, size)
        .bold(style.bold.unwrap_or(false))
        .italic(style.italic.unwrap_or(false))
}
