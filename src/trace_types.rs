//! Shared trace types available in all builds (not gated by debug_assertions).
//!
//! Both the writer (`trace.rs`, debug-only) and the Python reader
//! (`python/trace/`) depend on the types defined here.

use num_enum::TryFromPrimitive;

/// Operation-agnostic move-type classification shared across all trace
/// producers.  Every toolpath point is tagged with one of these so the
/// generic inspector can colour and categorise moves without knowing
/// about the originating operation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MoveKind {
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

#[expect(dead_code)]
impl MoveKind {
    pub fn is_travel(self) -> bool {
        self as u8 >= MoveKind::Travel as u8
    }

    pub fn is_cut(self) -> bool {
        self as u8 == MoveKind::Cut as u8
    }
}

/// In-memory representation of a complete trace file.
///
/// Records are stored as raw msgpack blobs; decoding to a concrete
/// type or Python dict is deferred to the caller.
#[derive(Clone, Debug)]
pub(crate) struct TraceFileData {
    pub ver: u16,
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

/// Record-kind byte values stored in every trace record.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, TryFromPrimitive)]
pub(crate) enum TraceKind {
    Init = 0,
    Cut = 1,
    ResumeStall = 2,
    ResumeStuck = 3,
    Exit = 4,
}
