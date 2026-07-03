//! Generic binary trace file writer.
//!
//! Writes a single self-contained binary trace file with the layout:
//!
//! ```text
//!   header:   magic[4] "ADPT" + reserved(u32) + record_count(u32) = 12 bytes
//!   geometry: tool_radius(f64)
//!             + boundary_vert_count(u32) + boundary verts (x,y f64 each)
//!             + island_count(u32)
//!               + per island: vert_count(u32) + verts (x,y f64 each)
//!             + seed_count(u32)
//!               + per seed: vert_count(u32) + verts (x,y f64 each)
//!   mat:      present(u8) — 0 = no MAT, 1 = MAT follows
//!               + node_count(u32)
//!                 + per node: x(f64) y(f64) clearance(f64)
//!               + edge_count(u32)
//!                 + per edge: from(u32) to(u32)
//!               + root(u32)
//!   toolpath: tp_count(u32) + per point: x(f64) y(f64) is_travel(u8) + 3 pad
//!   records:  record_count × 128 bytes (1 kind byte + 127 payload bytes)
//! ```
//!
//! Gated by ``cfg(debug_assertions)`` so tracing has zero cost in release
//! builds.  Callers should guard all [`Tracer`] usage with the same cfg or
//! use a file-local macro.

use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;

use crate::geo::algo::medial_axis::MedialAxis;
use crate::types::Polygon;

/// Magic header bytes: ``b"ADPT"``.
const TRACE_MAGIC: [u8; 4] = *b"ADPT";
/// Fixed size of each record (including the 1-byte kind).
const TRACE_RECORD_SIZE: usize = 128;
/// Number of payload bytes per record (everything after the kind byte).
pub(crate) const PAYLOAD_SIZE: usize = TRACE_RECORD_SIZE - 1;

/// Lightweight serializable snapshot of the Medial Axis Transform.
///
/// Only the data needed for visualisation (nodes, clearances, edges, root)
/// — no LCA cache, no branches.
#[derive(Clone, Debug)]
pub(crate) struct MatTrace {
    pub nodes: Vec<(f64, f64)>,
    pub clearances: Vec<f64>,
    pub edges: Vec<(u32, u32)>,
    pub root: u32,
}

impl From<&MedialAxis> for MatTrace {
    fn from(ma: &MedialAxis) -> Self {
        Self {
            nodes: ma.nodes.iter().map(|n| (n.point.x, n.point.y)).collect(),
            clearances: ma.nodes.iter().map(|n| n.clearance).collect(),
            edges: ma
                .edges
                .iter()
                .map(|&(i, j)| (i as u32, j as u32))
                .collect(),
            root: ma.root as u32,
        }
    }
}

/// Geometry + toolpath block written between the header and the records.
#[derive(Clone, Debug)]
pub(crate) struct TraceContext {
    pub tool_radius: f64,
    pub boundary: Polygon,
    pub islands: Vec<Polygon>,
    /// Initial cleared polygons (seeds).
    pub seeds: Vec<Polygon>,
}

/// Opaque binary trace file writer.
///
/// Records are buffered in memory and flushed at [`finish`] time, after
/// the toolpath block is written.  This lets the toolpath (only known
/// once the run is complete) appear before the records in the file while
/// still allowing records to be appended incrementally during the run.
///
/// A [`Drop`] implementation ensures that buffered records are always
/// flushed — even when the caller returns early with an error or when a
/// panic unwinds — so the trace file can be inspected for debugging
/// purposes.  On these partial paths an empty toolpath block is written
/// so the file remains structurally valid for the Python reader.
pub(crate) struct Tracer {
    file: std::fs::File,
    count: u32,
    /// Buffered 128-byte records, flushed in [`finish`] (or [`Drop`]).
    records: Vec<u8>,
    /// Set by [`finish`]; when `false` the [`Drop`] impl writes an empty
    /// toolpath block + buffered records and patches the record count.
    finalized: bool,
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
        file.write_all(&0u32.to_le_bytes())?; // reserved
        file.write_all(&0u32.to_le_bytes())?; // record count placeholder
        write_geometry(&mut file, ctx)?;
        Ok(Self {
            file,
            count: 0,
            records: Vec::new(),
            finalized: false,
        })
    }

    /// Write the MAT block.  Must be called after [`open`] and before
    /// [`write_toolpath`].  When `mat` is `None` a zero flag is written
    /// so the Python reader always knows where the MAT block ends.
    pub(crate) fn write_mat(&mut self, mat: Option<MatTrace>) {
        if let Some(m) = mat {
            let _ = self.file.write_all(&[1u8]);
            let _ = self.file.write_all(&(m.nodes.len() as u32).to_le_bytes());
            for i in 0..m.nodes.len() {
                let (x, y) = m.nodes[i];
                let c = m.clearances[i];
                let _ = self.file.write_all(&x.to_le_bytes());
                let _ = self.file.write_all(&y.to_le_bytes());
                let _ = self.file.write_all(&c.to_le_bytes());
            }
            let _ = self.file.write_all(&(m.edges.len() as u32).to_le_bytes());
            for &(i, j) in &m.edges {
                let _ = self.file.write_all(&i.to_le_bytes());
                let _ = self.file.write_all(&j.to_le_bytes());
            }
            let _ = self.file.write_all(&m.root.to_le_bytes());
        } else {
            let _ = self.file.write_all(&[0u8]);
        }
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
        self.finalized = true;
        Ok(())
    }
}

impl Drop for Tracer {
    fn drop(&mut self) {
        if self.finalized {
            return;
        }
        // On early-exit or panic paths the toolpath block was never
        // written.  Write an empty one so the file stays valid.
        write_toolpath_block(&mut self.file, &[]);
        // Flush any buffered records that were accumulated.
        let _ = self.file.write_all(&self.records);
        // Patch the record count into the header.
        let _ = self.file.seek(SeekFrom::Start(8));
        let _ = self.file.write_all(&self.count.to_le_bytes());
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
    file.write_all(&(ctx.seeds.len() as u32).to_le_bytes())?;
    for seed in &ctx.seeds {
        write_polygon(file, seed)?;
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
