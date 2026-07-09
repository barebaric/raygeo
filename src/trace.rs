//! Generic binary trace file writer with span-aware facade.
//!
//! Writes a single self-contained binary trace file with the layout:
//!
//! ```text
//!   header:   magic[4] "RGEO" + ver(u16) + reserved(u16) + record_count(u32) = 12 bytes
//!   records:  record_count × length-prefixed msgpack blobs
//!            (4-byte LE length + msgpack-serialized record struct)
//! ```
//!
//! When no path is given, the tracer becomes a no-op handle — all
//! methods do nothing.
//!
//! Gated by ``cfg(debug_assertions)`` so tracing has zero cost in release
//! builds.  Callers should guard all [`Tracer`] usage with the same cfg or
//! use a file-local macro.

use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;

use serde::Serialize;

use crate::trace_types::{
    EventKind, Meta, MoveKind, ProgressSnapshot, SpanRecord, ToolSnapshot,
    TraceEvent,
};

/// Internal state for an active trace file.
struct TracerInner {
    file: std::fs::File,
    count: u32,
    /// Buffered length-prefixed msgpack blobs, flushed in [`finish`] (or [`Drop`]).
    records: Vec<u8>,
    /// Set by [`flush`]; when `false` the [`Drop`] impl writes buffered
    /// records and patches the record count.
    finalized: bool,
}

impl TracerInner {
    fn new(path: &PathBuf) -> std::io::Result<Self> {
        let mut file = std::fs::File::create(path)?;
        file.write_all(b"RGEO")?;
        file.write_all(&3u16.to_le_bytes())?; // format version = 3
        file.write_all(&0u16.to_le_bytes())?; // reserved
        file.write_all(&0u32.to_le_bytes())?; // record count placeholder
        Ok(Self {
            file,
            count: 0,
            records: Vec::new(),
            finalized: false,
        })
    }

    fn write<T: Serialize>(&mut self, record: &T) {
        let bytes = rmp_serde::to_vec_named(record).unwrap_or_else(|e| {
            panic!("failed to serialize trace record: {e}")
        });
        self.records
            .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        self.records.extend_from_slice(&bytes);
        self.count += 1;
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.finalized {
            return Ok(());
        }
        self.file.write_all(&self.records)?;
        self.file.seek(SeekFrom::Start(8))?;
        self.file.write_all(&self.count.to_le_bytes())?;
        self.finalized = true;
        Ok(())
    }
}

impl Drop for TracerInner {
    fn drop(&mut self) {
        if self.finalized {
            return;
        }
        let _ = self.file.write_all(&self.records);
        let _ = self.file.seek(SeekFrom::Start(8));
        let _ = self.file.write_all(&self.count.to_le_bytes());
    }
}

/// Span-aware trace file writer.
///
/// All methods are no-ops when the tracer was opened with `None`.
pub(crate) struct Tracer {
    inner: Option<TracerInner>,
    next_span_id: u32,
    next_seq: u32,
}

impl Tracer {
    /// Open a tracer. `None` path => a no-op handle (all methods do nothing).
    pub(crate) fn open(path: Option<PathBuf>) -> Self {
        let inner = match path {
            Some(p) => TracerInner::new(&p).ok(),
            None => None,
        };
        Self {
            inner,
            next_span_id: 1,
            next_seq: 0,
        }
    }

    /// Begin a span. Returns the span id; caller must pair with [`exit`].
    pub(crate) fn enter(
        &mut self,
        parent: u32,
        source: &str,
        label: &str,
        attrs: Option<Meta>,
    ) -> u32 {
        let Some(ref mut inner) = self.inner else {
            return 0;
        };
        let id = self.next_span_id;
        self.next_span_id += 1;
        inner.write(&SpanRecord {
            kind: EventKind::SpanStart as u8,
            id,
            parent,
            source: source.to_string(),
            label: label.to_string(),
            attrs,
        });
        id
    }

    /// Close a span previously opened with [`enter`].
    pub(crate) fn exit(&mut self, span: u32, source: &str) {
        let Some(ref mut inner) = self.inner else {
            return;
        };
        inner.write(&SpanRecord {
            kind: EventKind::SpanEnd as u8,
            id: span,
            parent: 0,
            source: source.to_string(),
            label: String::new(),
            attrs: None,
        });
    }

    /// Emit an Init event (initial tool state of a span).
    pub(crate) fn init(
        &mut self,
        span: u32,
        source: &str,
        tool: ToolSnapshot,
        progress: ProgressSnapshot,
        meta: Option<Meta>,
    ) {
        let Some(ref mut inner) = self.inner else {
            return;
        };
        let seq = self.next_seq;
        self.next_seq += 1;
        inner.write(&TraceEvent {
            kind: EventKind::Init as u8,
            seq,
            span,
            source: source.to_string(),
            move_kind: None,
            tool: Some(tool),
            progress: Some(progress),
            meta,
        });
    }

    /// Emit a Move event (one toolpath point).
    pub(crate) fn move_point(
        &mut self,
        span: u32,
        source: &str,
        kind: MoveKind,
        tool: ToolSnapshot,
        progress: Option<ProgressSnapshot>,
        meta: Option<Meta>,
    ) {
        let Some(ref mut inner) = self.inner else {
            return;
        };
        let seq = self.next_seq;
        self.next_seq += 1;
        inner.write(&TraceEvent {
            kind: EventKind::Move as u8,
            seq,
            span,
            source: source.to_string(),
            move_kind: Some(kind as u8),
            tool: Some(tool),
            progress,
            meta,
        });
    }

    /// Emit a generic event (Resume, Exit, etc.).
    pub(crate) fn event(
        &mut self,
        span: u32,
        source: &str,
        kind: EventKind,
        tool: Option<ToolSnapshot>,
        meta: Option<Meta>,
    ) {
        let Some(ref mut inner) = self.inner else {
            return;
        };
        let seq = self.next_seq;
        self.next_seq += 1;
        inner.write(&TraceEvent {
            kind: kind as u8,
            seq,
            span,
            source: source.to_string(),
            move_kind: None,
            tool,
            progress: None,
            meta,
        });
    }

    /// Flush buffered records to disk and patch the count. Idempotent.
    pub(crate) fn finish(&mut self) {
        if let Some(ref mut inner) = self.inner {
            let _ = inner.flush();
        }
    }
}
