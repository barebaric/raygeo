// Adapted from the TraceDraw project, crates/cdr-format/src/loda.rs
// (https://github.com/tracedraw/tracedraw), MIT OR Apache-2.0,
// Copyright (c) 2026 TraceDraw members and contributors.
// Modified for raygeo: tests removed (tests live under tests/).

//! The argument table of a `loda` (or `lobj`) chunk: the record that
//! describes an object, a layer or a page by a list of typed arguments.
//!
//! Layout: the length, the argument count, the offset of the offsets
//! table, the offset of the types table and the object type, each a u32
//! (a u16 before version 6). The offsets table has one entry more than
//! the count (the extra one ends the last argument); the types table is
//! stored in reverse order, so the first offset pairs with the last type.

/// One entry of the table: the argument's type and where its bytes lie
/// in the chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Arg {
    pub ty: u32,
    pub off: usize,
    pub end: usize,
}

/// A decoded table: the object type and the arguments in table order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Table {
    pub kind: u32,
    pub args: Vec<Arg>,
}

/// Most arguments read from one table.
const MAX_ARGS: usize = 64;

/// An unsigned integer of the version's natural width at `at`: a u16
/// before version 6, a u32 from then on.
fn uint_at(d: &[u8], at: usize, version: u16) -> Option<u32> {
    if version < 6 {
        let s = d.get(at..at.checked_add(2)?)?;
        Some(u32::from(u16::from_le_bytes([s[0], s[1]])))
    } else {
        let s = d.get(at..at.checked_add(4)?)?;
        Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
}

/// Decode the table of a `loda` chunk's payload `d` written by `version`.
/// `None` when the header or an offset runs past the payload. An
/// argument whose end offset is out of order runs to the end of the
/// payload.
pub(crate) fn table(d: &[u8], version: u16) -> Option<Table> {
    let step = if version < 6 { 2 } else { 4 };
    let _len = uint_at(d, 0, version)?;
    let num_args = uint_at(d, step, version)? as usize;
    let start_args = uint_at(d, 2 * step, version)? as usize;
    let start_types = uint_at(d, 3 * step, version)? as usize;
    let kind = uint_at(d, 4 * step, version)?;
    let n = num_args.min(MAX_ARGS);
    let mut offsets = Vec::with_capacity(n + 1);
    for i in 0..=n {
        match uint_at(d, start_args.checked_add(i * step)?, version) {
            Some(o) => offsets.push(o as usize),
            None if i == n => offsets.push(d.len()),
            None => return None,
        }
    }
    let mut args = Vec::with_capacity(n);
    for i in 0..n {
        let ty =
            uint_at(d, start_types.checked_add((n - 1 - i) * step)?, version)?;
        let off = offsets[i].min(d.len());
        let next = offsets[i + 1];
        let end = if next > off && next <= d.len() {
            next
        } else {
            d.len()
        };
        args.push(Arg { ty, off, end });
    }
    Some(Table { kind, args })
}
