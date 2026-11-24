// Adapted from the TraceDraw project, crates/cdr-format/src/container.rs
// (https://github.com/tracedraw/tracedraw), MIT OR Apache-2.0,
// Copyright (c) 2026 TraceDraw members and contributors.
// Modified for raygeo: local error type, own ZIP reader, logging removed,
// helpers unrelated to reading the drawing dropped.

//! Container detection: a bare RIFF file, or a ZIP package wrapping one.

use crate::formats::coreldraw::error::{Error, Result};
use crate::formats::coreldraw::zip::ZipArchive;

/// CDR major version, as encoded in the RIFF form type (`CDR9`,
/// `CDRA` = 10, ... `CDRE` = 14, `CDRH` = 17, `CDRJ` = 18,
/// `CDRK` = 19, and so on; the letter `I` is not used).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Version(pub u16);

impl Version {
    pub fn from_form_type(form: &[u8; 4]) -> Option<Version> {
        let c = form[3];
        // Templates carry `CDT` and their own letters: `CDTQ` holds a
        // version 21 template (its `vrsn` 2100). The `vrsn` chunk, read
        // later, has the last word.
        if &form[..3] == b"CDT" {
            return match c {
                b'A'..=b'Z' => Some(Version(5 + (c - b'A') as u16)),
                _ => None,
            };
        }
        if &form[..3] != b"CDR" && &form[..3] != b"cdr" {
            return None;
        }
        // Digits are versions 1 to 9; letters continue from `A` = 10 with
        // `I` skipped: `H` = 17, `J` = 18, `N` = 22, `O` = 23. From there
        // some updates took a letter of their own: `P` = 24, `Q` = 24 (its
        // later update), `R` = 25, `S` = 25 (update), `T` = 26, `U` = 27.
        // Version 3 files carry a space. The `vrsn` chunk has the last word.
        match c {
            b' ' => Some(Version(3)),
            b'1'..=b'9' => Some(Version((c - b'0') as u16)),
            b'A'..=b'H' => Some(Version(10 + (c - b'A') as u16)),
            b'J'..=b'O' => Some(Version(9 + (c - b'A') as u16)),
            b'P' | b'Q' => Some(Version(24)),
            b'R' | b'S' => Some(Version(25)),
            b'T'..=b'Z' => Some(Version(26 + (c - b'T') as u16)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Container {
    pub kind: ContainerKind,
    pub version: Version,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ContainerKind {
    Riff,
    /// ZIP archive; the RIFF stream is the named member. Version 16 and
    /// later store most chunk payloads outside the RIFF stream, in the
    /// members listed by `content/dataFileList.dat` (`data_files`, in that
    /// order).
    Zip {
        riff_member: String,
        data_files: Vec<String>,
    },
}

const RIFF_MEMBERS: &[&str] = &[
    "content/riffData.cdr",
    "content/riffdata.cdr",
    "riffData.cdr",
    "content/root.dat",
];

const DATA_FILE_LIST: &str = "content/dataFileList.dat";

/// Detect the container of a CorelDRAW file.
pub(crate) fn detect(bytes: &[u8]) -> Result<Container> {
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" {
        let form: [u8; 4] =
            bytes[8..12].try_into().map_err(|_| Error::NotCdr)?;
        let version = Version::from_form_type(&form).ok_or(Error::NotCdr)?;
        return Ok(Container {
            kind: ContainerKind::Riff,
            version,
        });
    }
    if bytes.len() >= 4 && &bytes[0..4] == b"PK\x03\x04" {
        let archive = ZipArchive::new(bytes)?;
        let member = RIFF_MEMBERS
            .iter()
            .find(|m| archive.has(m))
            .map(|m| m.to_string())
            .or_else(|| {
                archive
                    .names()
                    .find(|n| n.to_ascii_lowercase().ends_with("riffdata.cdr"))
                    .map(String::from)
            })
            .ok_or_else(|| {
                Error::Unsupported(
                    "ZIP package without a riffData.cdr member".into(),
                )
            })?;
        let head = archive.read(&member).ok_or_else(|| {
            Error::Unsupported(format!("{member} unreadable"))
        })?;
        if head.get(0..4) != Some(b"RIFF") {
            return Err(Error::Unsupported(format!(
                "{member} is not a RIFF stream"
            )));
        }
        let form: [u8; 4] = head
            .get(8..12)
            .and_then(|s| s.try_into().ok())
            .ok_or(Error::NotCdr)?;
        let version = Version::from_form_type(&form).ok_or(Error::NotCdr)?;
        // Version 16+: the order of external data streams.
        let data_files = archive
            .read(DATA_FILE_LIST)
            .map(|list| {
                String::from_utf8_lossy(&list)
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        return Ok(Container {
            kind: ContainerKind::Zip {
                riff_member: member,
                data_files,
            },
            version,
        });
    }
    Err(Error::NotCdr)
}

/// The RIFF stream as a contiguous byte vector.
pub(crate) fn riff_stream(
    bytes: &[u8],
    container: &Container,
) -> Result<Vec<u8>> {
    match &container.kind {
        ContainerKind::Riff => Ok(bytes.to_vec()),
        ContainerKind::Zip { riff_member, .. } => {
            let archive = ZipArchive::new(bytes)?;
            archive.read(riff_member).ok_or_else(|| {
                Error::Unsupported(format!("{riff_member} unreadable"))
            })
        }
    }
}

/// The external data streams of a version 16+ file, in
/// `dataFileList.dat` order. A missing member yields an empty stream so
/// the indices stay aligned.
pub(crate) fn external_streams(
    bytes: &[u8],
    container: &Container,
) -> Vec<Vec<u8>> {
    let ContainerKind::Zip { data_files, .. } = &container.kind else {
        return Vec::new();
    };
    let archive = ZipArchive::new(bytes).ok();
    data_files
        .iter()
        .map(|name| {
            let archive = archive.as_ref()?;
            [
                format!("content/data/{name}"),
                format!("content/{name}"),
                name.to_string(),
            ]
            .iter()
            .find_map(|candidate| archive.read(candidate))
        })
        .map(Option::unwrap_or_default)
        .collect()
}
