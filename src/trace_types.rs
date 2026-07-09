//! Shared trace types available in all builds (not gated by debug_assertions).
//!
//! Both the writer (`trace.rs`, debug-only) and the Python reader
//! (`python/trace/`) depend on the types defined here.

use std::collections::BTreeMap;

use num_enum::FromPrimitive;
use serde::{Deserialize, Serialize};

/// Operation-agnostic move-type classification shared across all trace
/// producers.  Every toolpath point is tagged with one of these so the
/// generic inspector can colour and categorise moves without knowing
/// about the originating operation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveKind {
    /// Material-removing feed move.
    Cut = 0,
    /// Rapid reposition (usually in the safe / cleared area).
    Travel = 1,
    /// Axial entry into material (ramp, helix, peck, …).
    Plunge = 2,
    /// Ramp / lead-in at the start of a cutting pass.
    LeadIn = 3,
    /// Ramp / lead-out at the end of a cutting pass.
    LeadOut = 4,
    /// Safe linking move between disconnected segments.
    Link = 5,
    /// Dedicated positioning move to a resume target.
    Resume = 6,
    /// Routed path between two arbitrary points.
    Route = 7,
}

/// Record-kind byte values. Replaces the old TraceKind. Motion is unified
/// into a single `Move` kind tagged with a MoveKind; resume/exit keep
/// their own kinds for fast filtering by the inspector.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive)]
pub enum EventKind {
    SpanStart = 10,
    SpanEnd = 11,
    Init = 12,
    Move = 13,
    Resume = 14,
    #[num_enum(default)]
    Exit = 15,
}

/// Generic tool-state snapshot embedded in motion/init events. Always
/// present for events that carry position info.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ToolSnapshot {
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub heading: f64,
    pub prev_x: f64,
    pub prev_y: f64,
    pub prev_z: f64,
}

/// Generic progress snapshot.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ProgressSnapshot {
    pub step_idx: u32,
    pub ops_len: u32,
    pub status: u8,
}

/// Self-describing value used in the opaque `meta` map. This lets the
/// Python reader render arbitrary assembler metadata as a key/value table
/// without knowing the assembler. NO new dependencies — define our own.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub enum MetaValue {
    F64(f64),
    I64(i64),
    U32(u32),
    Bool(bool),
    Str(String),
    List(Vec<MetaValue>),
    Map(BTreeMap<String, MetaValue>),
}

impl From<f64> for MetaValue {
    fn from(v: f64) -> Self {
        MetaValue::F64(v)
    }
}
impl From<u32> for MetaValue {
    fn from(v: u32) -> Self {
        MetaValue::U32(v)
    }
}
impl From<i64> for MetaValue {
    fn from(v: i64) -> Self {
        MetaValue::I64(v)
    }
}
impl From<bool> for MetaValue {
    fn from(v: bool) -> Self {
        MetaValue::Bool(v)
    }
}
impl From<String> for MetaValue {
    fn from(v: String) -> Self {
        MetaValue::Str(v)
    }
}
impl From<&str> for MetaValue {
    fn from(v: &str) -> Self {
        MetaValue::Str(v.to_string())
    }
}
impl From<Vec<MetaValue>> for MetaValue {
    fn from(v: Vec<MetaValue>) -> Self {
        MetaValue::List(v)
    }
}

/// Convenience alias for the meta map.
pub type Meta = BTreeMap<String, MetaValue>;

/// One traceable event. This is the universal record every assembler
/// emits. Assembler-specific data lives in `meta`, NOT in core fields.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct TraceEvent {
    pub kind: u8,       // EventKind
    pub seq: u32,       // monotonic event sequence number
    pub span: u32,      // owning span id (0 = root/file-level)
    pub source: String, // assembler name: "adaptive","helix","workplan",...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub move_kind: Option<u8>, // MoveKind, only meaningful for EventKind::Move
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<ProgressSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
}

/// Span open/close record. Spans form a tree (parent points to enclosing
/// span; 0 = root). Setup snapshots (geometry, options) go in `attrs`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct SpanRecord {
    pub kind: u8, // EventKind::SpanStart or EventKind::SpanEnd
    pub id: u32,
    pub parent: u32,
    pub source: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Meta>,
}

/// In-memory representation of a complete trace file.
///
/// Records are stored as raw msgpack blobs; decoding to a concrete
/// type or Python dict is deferred to the caller.
#[derive(Clone, Debug)]
pub(crate) struct TraceFileData {
    pub ver: u16,
    #[expect(dead_code)]
    pub record_count: u32,
    pub records: Vec<Vec<u8>>,
}

impl TraceFileData {
    /// Open a trace file from disk and parse the header + records.
    ///
    /// Validates the magic bytes (`RGEO`) so callers can rely on the
    /// version being recognised.
    pub fn open(path: &std::path::Path) -> std::io::Result<Self> {
        use std::io::Read;

        let mut buf = Vec::new();
        std::fs::File::open(path)?.read_to_end(&mut buf)?;
        let mut cursor = std::io::Cursor::new(&buf);
        Self::read(&mut cursor)
    }

    fn read<R: std::io::Read + std::io::Seek>(
        r: &mut R,
    ) -> std::io::Result<Self> {
        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)?;
        if &magic != b"RGEO" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("bad trace magic: {magic:?}"),
            ));
        }

        let mut ver = [0u8; 2];
        r.read_exact(&mut ver)?;
        let ver = u16::from_le_bytes(ver);

        let mut reserved = [0u8; 2];
        r.read_exact(&mut reserved)?;

        let mut count = [0u8; 4];
        r.read_exact(&mut count)?;
        let record_count = u32::from_le_bytes(count);

        let mut records = Vec::with_capacity(record_count as usize);
        for _ in 0..record_count {
            let mut len_buf = [0u8; 4];
            r.read_exact(&mut len_buf)?;
            let rec_len = u32::from_le_bytes(len_buf) as usize;
            let mut rec = vec![0u8; rec_len];
            r.read_exact(&mut rec)?;
            records.push(rec);
        }

        Ok(Self {
            ver,
            record_count,
            records,
        })
    }
}
