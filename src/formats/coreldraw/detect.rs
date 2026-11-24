//! Detection of CorelDRAW files by their leading bytes.

use crate::formats::coreldraw::container;

/// The kind of Corel document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CdrFormat {
    /// A CorelDRAW drawing.
    Cdr,
    /// A Corel Presentation Exchange file.
    Cmx,
}

/// Returns the kind of Corel document in `data`, or `None` when the data
/// is not one.
///
/// A CorelDRAW drawing is either a bare RIFF file or a ZIP package with
/// the drawing inside; the earliest releases used the `WL` format. CMX
/// files are recognized but not read.
pub fn detect_format(data: &[u8]) -> Option<CdrFormat> {
    match data.get(..4) {
        Some(b"RIFF") | Some(b"RIFX") => {
            let form = data.get(8..11)?;
            if form.eq_ignore_ascii_case(b"CMX") {
                return Some(CdrFormat::Cmx);
            }
            container::detect(data).ok().map(|_| CdrFormat::Cdr)
        }
        Some(b"PK\x03\x04") => {
            container::detect(data).ok().map(|_| CdrFormat::Cdr)
        }
        _ if data.starts_with(b"WL") => Some(CdrFormat::Cdr),
        _ => None,
    }
}
