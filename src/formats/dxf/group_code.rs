//! Group codes: the raw level of a DXF file.
//!
//! A DXF file is a flat sequence of pairs: a group code on one line and
//! its value on the next. The code says how the value is to be read (a
//! string, an integer, a coordinate) and which entity field it fills.
//! Everything above this layer works with [`Item`]s; nothing above it
//! deals with line splitting or number parsing.

use crate::error::{RaygeoError, RaygeoResult};

/// One `(group code, value)` pair from a DXF file.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// The group code, e.g. `10` for an X coordinate or `0` for an
    /// entity type name.
    pub code: i32,
    /// The value exactly as it appeared in the file, trimmed of the
    /// leading and trailing whitespace DXF writers pad with.
    pub value: String,
}

impl Item {
    /// The value parsed as a float, for the numeric group codes.
    pub fn as_f64(&self) -> RaygeoResult<f64> {
        self.value.trim().parse::<f64>().map_err(|_| {
            RaygeoError::DxfInvalidValue {
                code: self.code,
                value: self.value.clone(),
            }
        })
    }

    /// The value parsed as a float, or `fallback` when it is absent or
    /// malformed.
    ///
    /// Used for optional fields, where a bad value should not discard an
    /// otherwise good entity.
    pub fn as_f64_or(&self, fallback: f64) -> f64 {
        self.value.trim().parse::<f64>().unwrap_or(fallback)
    }

    /// The value parsed as an integer, for the integer group codes.
    pub fn as_i64(&self) -> RaygeoResult<i64> {
        self.value.trim().parse::<i64>().map_err(|_| {
            RaygeoError::DxfInvalidValue {
                code: self.code,
                value: self.value.clone(),
            }
        })
    }

    /// The value parsed as an integer, or `fallback` when it is absent
    /// or malformed.
    pub fn as_i64_or(&self, fallback: i64) -> i64 {
        self.value.trim().parse::<i64>().unwrap_or(fallback)
    }

    /// The value as a trimmed string slice.
    pub fn as_str(&self) -> &str {
        self.value.trim()
    }
}

/// Splits a DXF file into its group-code items.
///
/// Accepts both LF and CRLF line endings and tolerates a missing final
/// newline. A file with an odd number of lines has a dangling group code;
/// that last code is dropped rather than failing the whole document,
/// because the items parsed up to that point are still usable.
///
/// Returns an error only when the stream does not start with a group
/// code at all, which means the input is not DXF.
pub fn tokenize(data: &str) -> RaygeoResult<Vec<Item>> {
    let mut lines = data.lines().map(|line| line.trim_end_matches('\r'));
    let mut items = Vec::new();
    while let Some(code_line) = lines.next() {
        if code_line.trim().is_empty() {
            continue;
        }
        let code = match code_line.trim().parse::<i32>() {
            Ok(code) => code,
            Err(_) => {
                if items.is_empty() {
                    return Err(RaygeoError::DxfParseError(format!(
                        "expected a group code, found {:?}",
                        code_line.trim()
                    )));
                }
                return Err(RaygeoError::DxfParseError(format!(
                    "expected a group code after item {}, found {:?}",
                    items.len(),
                    code_line.trim()
                )));
            }
        };
        let Some(value_line) = lines.next() else {
            break;
        };
        items.push(Item {
            code,
            value: value_line.trim().to_string(),
        });
    }
    Ok(items)
}

/// Reads a sequence of items as length-prefixed records.
///
/// In DXF, code `0` always starts a new entity or section, so the items
/// between two `0` codes belong together. This groups them that way.
pub fn split_records(items: &[Item]) -> Vec<&[Item]> {
    let mut records = Vec::new();
    let mut start = None;
    for (index, item) in items.iter().enumerate() {
        if item.code == 0 {
            if let Some(from) = start {
                records.push(&items[from..index]);
            }
            start = Some(index);
        }
    }
    if let Some(from) = start {
        records.push(&items[from..]);
    }
    records
}

/// The first item in `record` with the given group code.
pub fn find(record: &[Item], code: i32) -> Option<&Item> {
    record.iter().find(|item| item.code == code)
}

/// The float value of the first item with the given group code.
pub fn find_f64(record: &[Item], code: i32) -> Option<f64> {
    find(record, code).map(|item| item.as_f64_or(0.0))
}
