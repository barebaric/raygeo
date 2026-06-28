//! Adaptive-clearing trace format.
//!
//! Defines the record layout consumed by ``tools/adaptive_inspector.py``:
//! a 127-byte payload with fixed-offset fields, plus a companion ``.tp``
//! toolpath file.  The generic [`crate::trace::Tracer`] and
//! [`crate::trace::DebugTracer`] handle the actual file I/O and release-
//! mode no-op behaviour.

use std::io::Write;

use crate::ops::container::Ops;
use crate::ops::cut::StepStatus;
use crate::types::Point;

// ── TraceKind ───────────────────────────────────────────────────────

/// Record kind byte values.
#[repr(u8)]
#[derive(Clone, Copy)]
pub(crate) enum TraceKind {
    Init = 0,
    Cut = 1,
    ResumeStall = 2,
    ResumeStuck = 3,
    Exit = 4,
}

// ── RecordBuf ───────────────────────────────────────────────────────

/// 127-byte payload buffer with typed setters at the correct offsets.
///
/// Offsets match the binary format expected by the Python inspector.
/// Fields not explicitly set remain zero.
pub(crate) struct RecordBuf([u8; crate::trace::PAYLOAD_SIZE]);

impl Default for RecordBuf {
    fn default() -> Self {
        Self([0u8; crate::trace::PAYLOAD_SIZE])
    }
}

impl RecordBuf {
    // ── private helpers ──────────────────────────────────────────

    fn u8(&mut self, o: usize, v: u8) {
        self.0[o] = v;
    }
    fn u32(&mut self, o: usize, v: u32) {
        self.0[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn f64(&mut self, o: usize, v: f64) {
        self.0[o..o + 8].copy_from_slice(&v.to_le_bytes());
    }
    fn point(&mut self, o: usize, v: Point) {
        self.f64(o, v.x);
        self.f64(o + 8, v.y);
    }

    // ── field setters ────────────────────────────────────────────
    // Offsets are payload-relative (byte 1 in the full record).

    pub fn status(&mut self, v: StepStatus) {
        let b: u8 = match v {
            StepStatus::Ok => 0,
            StepStatus::BoundaryHit => 1,
            StepStatus::LostEngagement => 2,
            StepStatus::NoConvergence => 3,
        };
        self.u8(0, b);
    }

    pub fn step_idx(&mut self, v: u32) {
        self.u32(1, v);
    }

    pub fn iters(&mut self, v: u32) {
        self.u32(5, v);
    }

    pub fn pos(&mut self, v: Point) {
        self.point(9, v);
    }

    pub fn heading(&mut self, v: f64) {
        self.f64(25, v);
    }

    pub fn smoothed_heading(&mut self, v: f64) {
        self.f64(33, v);
    }

    pub fn predicted_angle(&mut self, v: f64) {
        self.f64(41, v);
    }

    pub fn iteration_angle(&mut self, v: f64) {
        self.f64(49, v);
    }

    pub fn eng_angle(&mut self, v: f64) {
        self.f64(57, v);
    }

    pub fn eng_area(&mut self, v: f64) {
        self.f64(65, v);
    }

    pub fn eng_chord(&mut self, v: f64) {
        self.f64(73, v);
    }

    pub fn cut_area(&mut self, v: f64) {
        self.f64(81, v);
    }

    pub fn total_area(&mut self, v: f64) {
        self.f64(89, v);
    }

    pub fn remaining_area(&mut self, v: f64) {
        self.f64(97, v);
    }

    pub fn prev_pos(&mut self, v: Point) {
        self.point(105, v);
    }

    pub fn ops_len(&mut self, v: u32) {
        self.u32(121, v);
    }

    pub fn pack(&self) -> &[u8; crate::trace::PAYLOAD_SIZE] {
        &self.0
    }
}

// ── Toolpath companion file ─────────────────────────────────────────

/// Write the companion ``.tp`` toolpath next to the trace file.
///
/// Format: count (u32 LE) followed by count records of 20 bytes each:
///   x (f64 LE), y (f64 LE), is_travel (u8), 3 zero pad bytes.
pub(crate) fn write_toolpath(trace_path: &std::path::Path, ops: &Ops) {
    let tp_path = trace_path.with_extension("tp");
    let mut f = match std::fs::File::create(&tp_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("trace: failed to write toolpath: {}", e);
            return;
        }
    };
    let mut count: u32 = 0;
    for i in 0..ops.len() {
        if ops.is_travel(i) || ops.is_cutting(i) {
            count += 1;
        }
    }
    let _ = f.write_all(&count.to_le_bytes());
    let mut buf = [0u8; 20];
    for i in 0..ops.len() {
        let is_travel = ops.is_travel(i);
        let is_cutting = ops.is_cutting(i);
        if !is_travel && !is_cutting {
            continue;
        }
        let ep = ops.endpoint(i);
        buf[0..8].copy_from_slice(&ep.x.to_le_bytes());
        buf[8..16].copy_from_slice(&ep.y.to_le_bytes());
        buf[16] = if is_travel { 1 } else { 0 };
        buf[17] = 0;
        buf[18] = 0;
        buf[19] = 0;
        let _ = f.write_all(&buf);
    }
}
