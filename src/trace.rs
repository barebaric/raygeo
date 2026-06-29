//! Generic binary trace file writer.
//!
//! Writes a single self-contained binary trace file with the layout:
//!
//! ```text
//!   header:   magic[4] "ADPT" + version[4] + record_count[4]   = 12 bytes
//!   geometry: tool_radius(f64)
//!             + boundary_vert_count(u32) + boundary verts (x,y f64 each)
//!             + island_count(u32)
//!               + per island: vert_count(u32) + verts (x,y f64 each)
//!   toolpath: tp_count(u32) + per point: x(f64) y(f64) is_travel(u8) + 3 pad
//!   records:  record_count × 128 bytes (1 kind byte + 127 payload bytes)
//! ```
//!
//! Version 1 wrote only the header + records and relied on a companion
//! ``.tp`` file for the toolpath; geometry was supplied out-of-band by the
//! caller.  Version 2 embeds the geometry and toolpath so the file is fully
//! self-contained.
//!
//! Gated by ``cfg(debug_assertions)`` so tracing has zero cost in release
//! builds.  Callers should guard all [`Tracer`] usage with the same cfg or
//! use a file-local macro.

use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;

use crate::types::Polygon;

/// Magic header bytes: ``b"ADPT"``.
const TRACE_MAGIC: [u8; 4] = *b"ADPT";
/// Trace format version.
const TRACE_VERSION: u32 = 2;
/// Fixed size of each record (including the 1-byte kind).
const TRACE_RECORD_SIZE: usize = 128;
/// Number of payload bytes per record (everything after the kind byte).
pub(crate) const PAYLOAD_SIZE: usize = TRACE_RECORD_SIZE - 1;

/// Geometry + toolpath block written between the header and the records.
#[derive(Clone, Debug)]
pub(crate) struct TraceContext {
    pub tool_radius: f64,
    pub boundary: Polygon,
    pub islands: Vec<Polygon>,
}

/// Opaque binary trace file writer.
///
/// Records are buffered in memory and flushed at [`finish`] time, after
/// the toolpath block is written.  This lets the toolpath (only known
/// once the run is complete) appear before the records in the file while
/// still allowing records to be appended incrementally during the run.
pub(crate) struct Tracer {
    file: std::fs::File,
    count: u32,
    /// Buffered 128-byte records, flushed in [`finish`].
    records: Vec<u8>,
}

impl Tracer {
    /// Open a new trace file, write the 12-byte header followed by the
    /// geometry block.  `ctx` is embedded verbatim so the file is
    /// self-contained.  The toolpath is appended later via
    /// [`write_toolpath`].
    pub(crate) fn open(
        path: &PathBuf,
        ctx: &TraceContext,
    ) -> std::io::Result<Self> {
        let mut file = std::fs::File::create(path)?;
        file.write_all(&TRACE_MAGIC)?;
        file.write_all(&TRACE_VERSION.to_le_bytes())?;
        file.write_all(&0u32.to_le_bytes())?; // record count placeholder
        write_geometry(&mut file, ctx)?;
        Ok(Self {
            file,
            count: 0,
            records: Vec::new(),
        })
    }

    /// Write the toolpath block.  Must be called exactly once, after the
    /// main run is complete and before [`finish`].  Records buffered via
    /// [`write`] are flushed after the toolpath in [`finish`].
    pub(crate) fn write_toolpath(&mut self, tp: &[TracePoint]) {
        write_toolpath_block(&mut self.file, tp);
    }

    /// Buffer one 128-byte record.  *kind* occupies byte 0; *payload*
    /// fills bytes 1..128.  The caller is responsible for packing fields
    /// into the payload at the correct offsets.  Records are written to
    /// disk in [`finish`], after the toolpath block.
    pub(crate) fn write(&mut self, kind: u8, payload: &[u8; PAYLOAD_SIZE]) {
        let off = self.records.len();
        self.records.resize(off + TRACE_RECORD_SIZE, 0);
        self.records[off] = kind;
        self.records[off + 1..off + TRACE_RECORD_SIZE].copy_from_slice(payload);
        self.count += 1;
    }

    /// Finalise the file: the toolpath block must already have been
    /// written via [`write_toolpath`].  Flush the buffered records, then
    /// patch the record count into the header.
    pub(crate) fn finish(&mut self) -> std::io::Result<()> {
        self.file.write_all(&self.records)?;
        self.file.seek(SeekFrom::Start(8))?;
        self.file.write_all(&self.count.to_le_bytes())?;
        Ok(())
    }
}

/// A single toolpath point (centre position + travel flag).
#[derive(Clone, Copy, Debug)]
pub(crate) struct TracePoint {
    pub x: f64,
    pub y: f64,
    pub is_travel: bool,
}

fn write_geometry(
    file: &mut std::fs::File,
    ctx: &TraceContext,
) -> std::io::Result<()> {
    file.write_all(&ctx.tool_radius.to_le_bytes())?;
    write_polygon(file, &ctx.boundary)?;
    file.write_all(&(ctx.islands.len() as u32).to_le_bytes())?;
    for isl in &ctx.islands {
        write_polygon(file, isl)?;
    }
    Ok(())
}

fn write_polygon(
    file: &mut std::fs::File,
    poly: &Polygon,
) -> std::io::Result<()> {
    file.write_all(&(poly.len() as u32).to_le_bytes())?;
    for p in poly {
        file.write_all(&p.x.to_le_bytes())?;
        file.write_all(&p.y.to_le_bytes())?;
    }
    Ok(())
}

fn write_toolpath_block(file: &mut std::fs::File, tp: &[TracePoint]) {
    let _ = file.write_all(&(tp.len() as u32).to_le_bytes());
    let mut buf = [0u8; 17];
    for p in tp {
        buf[0..8].copy_from_slice(&p.x.to_le_bytes());
        buf[8..16].copy_from_slice(&p.y.to_le_bytes());
        buf[16] = if p.is_travel { 1 } else { 0 };
        let _ = file.write_all(&buf);
    }
}
