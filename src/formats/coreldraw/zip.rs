//! Minimal reader for the ZIP package of a version 14 and later file.
//!
//! Only what the package's role in a CorelDRAW file needs: the central
//! directory, and reading one stored or deflated member by name. The
//! member list and sizes are bounds-checked, and a member may inflate to
//! at most [`MAX_MEMBER`] bytes.

use std::io::Read;

use crate::formats::coreldraw::error::{Error, Result};

/// The most one member may inflate to. The package holds the RIFF stream,
/// small data streams and colour profiles; pictures live in the drawing,
/// not in members we read, so half a gigabyte is far beyond any of them.
const MAX_MEMBER: u64 = 512 << 20;

const EOCD_SIGNATURE: u32 = 0x0605_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const LOCAL_SIGNATURE: u32 = 0x0403_4b50;

const EOCD_LEN: usize = 22;
const MAX_COMMENT: usize = u16::MAX as usize;
const CENTRAL_HEADER_LEN: usize = 46;
const LOCAL_HEADER_LEN: usize = 30;

struct Entry {
    name: String,
    method: u16,
    compressed: u64,
    size: u64,
    local_offset: u64,
}

/// The directory of a ZIP package, read over its raw bytes.
pub(crate) struct ZipArchive<'a> {
    data: &'a [u8],
    entries: Vec<Entry>,
}

fn u16le(b: &[u8], at: usize) -> Option<u16> {
    b.get(at..at + 2).map(|s| u16::from_le_bytes([s[0], s[1]]))
}

fn u32le(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

impl<'a> ZipArchive<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self> {
        let eocd = find_eocd(data).ok_or_else(|| {
            Error::Unsupported("ZIP package without a directory".into())
        })?;
        let count = u16le(data, eocd + 10).ok_or(Error::Truncated(eocd))?;
        let directory = u32le(data, eocd + 16).ok_or(Error::Truncated(eocd))?;
        let mut pos = directory as usize;
        let mut entries = Vec::with_capacity(count as usize);
        for _ in 0..count {
            if u32le(data, pos) != Some(CENTRAL_SIGNATURE) {
                return Err(Error::Unsupported("damaged ZIP directory".into()));
            }
            let flags = u16le(data, pos + 8).ok_or(Error::Truncated(pos))?;
            let method = u16le(data, pos + 10).ok_or(Error::Truncated(pos))?;
            let compressed =
                u32le(data, pos + 20).ok_or(Error::Truncated(pos))?;
            let size = u32le(data, pos + 24).ok_or(Error::Truncated(pos))?;
            let name_len =
                u16le(data, pos + 28).ok_or(Error::Truncated(pos))?;
            let extra_len =
                u16le(data, pos + 30).ok_or(Error::Truncated(pos))?;
            let comment_len =
                u16le(data, pos + 32).ok_or(Error::Truncated(pos))?;
            let local_offset =
                u32le(data, pos + 42).ok_or(Error::Truncated(pos))?;
            if flags & 0x0001 != 0 {
                return Err(Error::Unsupported("encrypted ZIP member".into()));
            }
            if [compressed, size, local_offset].contains(&u32::MAX) {
                return Err(Error::Unsupported("ZIP64 package".into()));
            }
            let name_at = pos + CENTRAL_HEADER_LEN;
            let name = data
                .get(name_at..name_at + name_len as usize)
                .ok_or(Error::Truncated(name_at))?;
            entries.push(Entry {
                name: String::from_utf8_lossy(name).into_owned(),
                method,
                compressed: u64::from(compressed),
                size: u64::from(size),
                local_offset: u64::from(local_offset),
            });
            pos = name_at + name_len as usize + extra_len as usize;
            pos += comment_len as usize;
        }
        Ok(Self { data, entries })
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|entry| entry.name.as_str())
    }

    pub fn has(&self, name: &str) -> bool {
        self.entries.iter().any(|entry| entry.name == name)
    }

    /// The inflated bytes of a member, or `None` when it is missing, uses
    /// an unsupported method, or is larger than [`MAX_MEMBER`].
    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        let entry = self.entries.iter().find(|entry| entry.name == name)?;
        if entry.size > MAX_MEMBER {
            return None;
        }
        let local = entry.local_offset as usize;
        if u32le(self.data, local) != Some(LOCAL_SIGNATURE) {
            return None;
        }
        let name_len = u16le(self.data, local + 26)? as usize;
        let extra_len = u16le(self.data, local + 28)? as usize;
        let start = local + LOCAL_HEADER_LEN + name_len + extra_len;
        let end = start.checked_add(entry.compressed as usize)?;
        let packed = self.data.get(start..end)?;
        let mut out = Vec::with_capacity(entry.size as usize);
        match entry.method {
            0 => out.extend_from_slice(packed),
            8 => {
                let decoder = flate2::read::DeflateDecoder::new(packed);
                decoder.take(MAX_MEMBER).read_to_end(&mut out).ok()?;
            }
            _ => return None,
        }
        Some(out)
    }
}

fn find_eocd(data: &[u8]) -> Option<usize> {
    let first = data.len().saturating_sub(EOCD_LEN + MAX_COMMENT);
    (first..data.len().saturating_sub(EOCD_LEN - 1))
        .rev()
        .find(|&at| u32le(data, at) == Some(EOCD_SIGNATURE))
}
