//! Generic binary trace file writer.
//!
//! Writes a single self-contained binary trace file with the layout:
//!
//! ```text
//!   header:   magic[4] "RGEO" + ver(u16) + reserved(u16) + record_count(u32) = 12 bytes
//!   records:  record_count × length-prefixed msgpack blobs
//!            (4-byte LE length + msgpack-serialized record struct)
//! ```
//!
//! Gated by ``cfg(debug_assertions)`` so tracing has zero cost in release
//! builds.  Callers should guard all [`Tracer`] usage with the same cfg or
//! use a file-local macro.

use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;

use serde::Serialize;

/// Opaque binary trace file writer.
///
/// Records are length-prefixed msgpack blobs buffered in memory and
/// flushed at [`finish`] time.
///
/// A [`Drop`] implementation ensures that buffered records are always
/// flushed — even when the caller returns early with an error or when a
/// panic unwinds — so the trace file can be inspected for debugging
/// purposes.
pub(crate) struct Tracer {
    file: std::fs::File,
    count: u32,
    /// Buffered length-prefixed msgpack blobs, flushed in [`finish`] (or [`Drop`]).
    records: Vec<u8>,
    /// Set by [`finish`]; when `false` the [`Drop`] impl writes buffered
    /// records and patches the record count.
    finalized: bool,
}

impl Tracer {
    /// Open a new trace file, write the 12-byte header (magic + version +
    /// reserved + record-count placeholder).
    pub(crate) fn open(path: &PathBuf) -> std::io::Result<Self> {
        let mut file = std::fs::File::create(path)?;
        file.write_all(b"RGEO")?;
        file.write_all(&2u16.to_le_bytes())?; // format version = 2
        file.write_all(&0u16.to_le_bytes())?; // reserved
        file.write_all(&0u32.to_le_bytes())?; // record count placeholder
        Ok(Self {
            file,
            count: 0,
            records: Vec::new(),
            finalized: false,
        })
    }

    /// Buffer one msgpack-serialized record.  The record is serialized
    /// with `rmp_serde` and stored as a 4-byte length prefix followed by
    /// the msgpack bytes.  Records are written to disk in [`finish`].
    pub(crate) fn write<T: Serialize>(&mut self, record: &T) {
        let bytes = rmp_serde::to_vec_named(record).unwrap_or_else(|e| {
            panic!("failed to serialize trace record: {e}")
        });
        self.records
            .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        self.records.extend_from_slice(&bytes);
        self.count += 1;
    }

    /// Finalise the file: flush buffered records, then patch the
    /// record count into the header.
    pub(crate) fn finish(&mut self) -> std::io::Result<()> {
        self.file.write_all(&self.records)?;
        self.file.seek(SeekFrom::Start(8))?;
        self.file.write_all(&self.count.to_le_bytes())?;
        self.finalized = true;
        Ok(())
    }
}

impl Drop for Tracer {
    fn drop(&mut self) {
        if self.finalized {
            return;
        }
        let _ = self.file.write_all(&self.records);
        let _ = self.file.seek(SeekFrom::Start(8));
        let _ = self.file.write_all(&self.count.to_le_bytes());
    }
}
