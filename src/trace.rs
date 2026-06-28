//! Generic binary trace file writer.
//!
//! Writes a fixed-size binary file with a 12-byte header (magic + version +
//! record count) followed by 128-byte records.  The caller provides an opaque
//! kind byte and a 127-byte payload; this module knows nothing about record
//! semantics or field layouts.
//!
//! Gated by ``cfg(debug_assertions)`` so tracing has zero cost in release
//! builds.  Callers should guard all [`Tracer`] usage with the same cfg or
//! use a file-local macro.

use std::io::{Seek, Write};
use std::path::PathBuf;

/// Magic header bytes: ``b"ADPT"``.
const TRACE_MAGIC: [u8; 4] = *b"ADPT";
/// Trace format version.
const TRACE_VERSION: u32 = 1;
/// Fixed size of each record (including the 1-byte kind).
const TRACE_RECORD_SIZE: usize = 128;
/// Number of payload bytes per record (everything after the kind byte).
pub(crate) const PAYLOAD_SIZE: usize = TRACE_RECORD_SIZE - 1;

/// Opaque binary trace file writer.
pub(crate) struct Tracer {
    file: std::fs::File,
    path: PathBuf,
    count: u32,
}

impl Tracer {
    /// Open a new trace file, writing the 12-byte header.
    pub(crate) fn open(path: &PathBuf) -> std::io::Result<Self> {
        let mut file = std::fs::File::create(path)?;
        file.write_all(&TRACE_MAGIC)?;
        file.write_all(&TRACE_VERSION.to_le_bytes())?;
        file.write_all(&0u32.to_le_bytes())?;
        Ok(Self {
            file,
            path: path.clone(),
            count: 0,
        })
    }

    /// Write one 128-byte record.  *kind* occupies byte 0; *payload* fills
    /// bytes 1..128.  The caller is responsible for packing fields into the
    /// payload at the correct offsets.
    pub(crate) fn write(&mut self, kind: u8, payload: &[u8; PAYLOAD_SIZE]) {
        let mut buf = [0u8; TRACE_RECORD_SIZE];
        buf[0] = kind;
        buf[1..].copy_from_slice(payload);
        let _ = self.file.write_all(&buf);
        self.count += 1;
    }

    /// Finalise the file by patching the record count into the header.
    pub(crate) fn finish(&mut self) -> std::io::Result<()> {
        self.file.seek(std::io::SeekFrom::Start(8))?;
        self.file.write_all(&self.count.to_le_bytes())?;
        Ok(())
    }

    /// Borrow the file path (e.g. to derive a companion ``.tp`` path).
    pub(crate) fn path(&self) -> &std::path::Path {
        self.path.as_path()
    }
}
