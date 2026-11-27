//! GRBL character-counting flow control.
//!
//! Port of Rayforge's `GrblSerialTransport`: tracks how many bytes
//! are outstanding against the device's RX buffer and provides
//! backpressure so callers never overflow it.  Also performs the
//! low-level response extraction — `ok`/`error:` are pulled out of
//! the raw byte stream for buffer accounting *before* line-based
//! parsing, so acknowledgements interleaved with status reports by
//! buggy firmware are never lost.
//!
//! All state lives behind a `std::sync::Mutex` held only for short,
//! await-free critical sections; waiters park on `Notify` objects so
//! a reader parsing incoming bytes is never blocked by a sender
//! waiting for buffer space.

use std::collections::VecDeque;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use regex::bytes::Regex;
use tokio::sync::Notify;

/// Error returned by the timed waits: the deadline elapsed.
///
/// (tokio's `Elapsed` cannot be constructed outside the crate.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedOut;

/// Default RX buffer size for standard Grbl 1.1 (128-byte buffer,
/// safe limit 127).  Custom firmwares may report a smaller size via
/// the $I OPT line.
pub const DEFAULT_GRBL_RX_BUFFER_SIZE: usize = 127;

static OK_ACK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"ok\r*\n").expect("valid regex"));

/// A command sent with buffer accounting, awaiting its ack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingCommand {
    pub length: usize,
    pub op_index: Option<i64>,
    pub command: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrblResponseType {
    Ok,
    Error,
    Line,
}

#[derive(Debug, Clone)]
pub struct GrblResponse {
    pub rtype: GrblResponseType,
    pub text: String,
    pub pending: Option<PendingCommand>,
}

#[derive(Debug, Default)]
struct FlowInner {
    rx_count: usize,
    rx_size: Option<usize>,
    status_buffer: Vec<u8>,
    pending: VecDeque<PendingCommand>,
    acked_since_flush: bool,
    drained_since_flush: bool,
}

/// Buffer accounting and low-level response parsing state.
#[derive(Debug, Default)]
pub struct FlowControl {
    inner: Mutex<FlowInner>,
    space_available: Notify,
    pending_drained: Notify,
}

impl FlowControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn rx_buffer_size(&self) -> usize {
        self.inner
            .lock()
            .unwrap()
            .rx_size
            .unwrap_or(DEFAULT_GRBL_RX_BUFFER_SIZE)
    }

    pub fn buffer_count(&self) -> usize {
        self.inner.lock().unwrap().rx_count
    }

    pub fn pending_queue(&self) -> VecDeque<PendingCommand> {
        self.inner.lock().unwrap().pending.clone()
    }

    /// True when *needed* more bytes would overflow the RX buffer.
    pub fn needs_space(&self, needed: usize) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.rx_count + needed
            > inner.rx_size.unwrap_or(DEFAULT_GRBL_RX_BUFFER_SIZE)
    }

    /// Update the device's RX buffer size (as reported by the $I
    /// OPT line).  Must be called before streaming begins.
    pub fn set_rx_buffer_size(&self, size: usize) {
        if size > 0 {
            log_crate::info!("GRBL RX buffer size set to {size} bytes");
            self.inner.lock().unwrap().rx_size = Some(size);
        }
    }

    /// Parse raw serial bytes into GRBL responses.
    ///
    /// Scans for 'ok' and 'error:' responses in the byte stream,
    /// handling buffer accounting for them, before line-splitting.
    /// Returns responses for non-ok/error lines (status reports,
    /// alarms, info messages) as well.
    pub fn parse_incoming(&self, data: &[u8]) -> Vec<GrblResponse> {
        let mut responses = Vec::new();
        {
            let mut inner = self.inner.lock().unwrap();
            inner.status_buffer.extend_from_slice(data);
            Self::extract_acks_from_buffer(&mut inner, &mut responses);
            while let Some(nl) =
                inner.status_buffer.iter().position(|b| *b == b'\n')
            {
                let message_bytes: Vec<u8> =
                    inner.status_buffer.drain(..=nl).collect();
                let Ok(message) = String::from_utf8(message_bytes) else {
                    log_crate::warn!("Dropped invalid UTF-8 bytes");
                    continue;
                };
                for line in message.trim().lines() {
                    if line.is_empty() {
                        continue;
                    }
                    if line == "ok" {
                        let pending = Self::ack_ok_locked(&mut inner);
                        inner.acked_since_flush = true;
                        inner.drained_since_flush |= inner.pending.is_empty();
                        responses.push(GrblResponse {
                            rtype: GrblResponseType::Ok,
                            text: "ok".to_string(),
                            pending,
                        });
                    } else if line.starts_with("error:") {
                        Self::ack_ok_locked(&mut inner);
                        inner.acked_since_flush = true;
                        inner.drained_since_flush |= inner.pending.is_empty();
                        responses.push(GrblResponse {
                            rtype: GrblResponseType::Error,
                            text: line.to_string(),
                            pending: None,
                        });
                    } else {
                        responses.push(GrblResponse {
                            rtype: GrblResponseType::Line,
                            text: line.to_string(),
                            pending: None,
                        });
                    }
                }
            }
        }
        // Notifications are deferred to flush_notifications() so the
        // session finishes processing the responses (progress
        // callbacks, request completion) before a streaming task
        // wakes up and observes the freed buffer space — mirroring
        // asyncio's single-threaded scheduling in the Python driver.
        responses
    }

    /// Deliver the flow-control notifications accumulated by
    /// [`FlowControl::parse_incoming`].  The session calls this
    /// after processing the parsed responses.
    pub fn flush_notifications(&self) {
        let (space, drained) = {
            let mut inner = self.inner.lock().unwrap();
            (
                std::mem::take(&mut inner.acked_since_flush),
                std::mem::take(&mut inner.drained_since_flush),
            )
        };
        if space {
            self.signal_space_available();
        }
        if drained {
            self.notify_drained();
        }
    }
    fn extract_acks_from_buffer(
        inner: &mut FlowInner,
        responses: &mut Vec<GrblResponse>,
    ) {
        while let Some(m) = OK_ACK_RE.find(&inner.status_buffer) {
            let start = m.start();
            let end = m.end();
            inner.status_buffer.drain(start..end);
            let pending = Self::ack_ok_locked(inner);
            inner.acked_since_flush = true;
            inner.drained_since_flush |= inner.pending.is_empty();
            responses.push(GrblResponse {
                rtype: GrblResponseType::Ok,
                text: "ok".to_string(),
                pending,
            });
        }

        // Detect NULL-byte corrupted 'ok' (hardware fault).
        while let Some((start, end)) =
            Self::find_null_corrupted_ok(&inner.status_buffer)
        {
            inner.status_buffer.drain(start..end);
            log_crate::error!(
                "HARDWARE FAULT DETECTED: A corrupted 'ok' \
                 acknowledgement with NULL bytes was received. This \
                 indicates a critical problem with the USB cable, \
                 electrical noise (EMI), or power supply. The hardware \
                 connection MUST be fixed for reliable operation."
            );
            let pending = Self::ack_ok_locked(inner);
            inner.acked_since_flush = true;
            inner.drained_since_flush |= inner.pending.is_empty();
            responses.push(GrblResponse {
                rtype: GrblResponseType::Ok,
                text: "ok".to_string(),
                pending,
            });
        }

        const ERROR_MARKER: &[u8] = b"error:";
        let mut search_from = 0usize;
        while let Some(rel) =
            find_subslice(&inner.status_buffer[search_from..], ERROR_MARKER)
        {
            let start = search_from + rel;
            let Some(end_rel) = inner.status_buffer[start..]
                .iter()
                .position(|b| *b == b'\n')
            else {
                break;
            };
            let end = start + end_rel + 1;
            let error_bytes: Vec<u8> =
                inner.status_buffer.drain(start..end).collect();
            let Ok(error_bytes) = String::from_utf8(error_bytes) else {
                continue;
            };
            let error_text = error_bytes.trim().to_string();
            log_crate::debug!(
                "Extracted '{error_text}' from raw buffer \
                 (interleaved recovery)"
            );
            Self::ack_ok_locked(inner);
            inner.acked_since_flush = true;
            inner.drained_since_flush |= inner.pending.is_empty();
            responses.push(GrblResponse {
                rtype: GrblResponseType::Error,
                text: error_text,
                pending: None,
            });
            search_from = start;
        }
    }

    /// Search `status_buffer` for a pattern like `o\0k\r*\n` where
    /// NULL bytes are interspersed in 'ok'.  Returns `(start, end)`
    /// byte indices or `None`.
    fn find_null_corrupted_ok(buf: &[u8]) -> Option<(usize, usize)> {
        let len = buf.len();
        let mut i = 0;
        while i < len {
            if buf[i] != b'o' {
                i += 1;
                continue;
            }
            let mut j = i + 1;
            while j < len && buf[j] == 0 {
                j += 1;
            }
            if j < len && buf[j] == b'k' && j > i + 1 {
                let mut k = j + 1;
                while k < len && buf[k] == b'\r' {
                    k += 1;
                }
                if k < len && buf[k] == b'\n' {
                    return Some((i, k + 1));
                }
            }
            i += 1;
        }
        None
    }

    fn ack_ok_locked(inner: &mut FlowInner) -> Option<PendingCommand> {
        let pending = inner.pending.pop_front()?;
        log_crate::debug!(
            "Buffer ack: freeing {} bytes for {:?} (op_index={:?}, \
             remaining: {})",
            pending.length,
            pending.command,
            pending.op_index,
            inner.pending.len()
        );
        inner.rx_count = inner.rx_count.saturating_sub(pending.length);
        Some(pending)
    }

    fn notify_drained(&self) {
        self.pending_drained.notify_waiters();
        self.pending_drained.notify_one();
    }

    /// Wake parked senders (e.g. on cancel).  Uses permits so a
    /// waiter that has not yet registered still observes the wake-up
    /// (issue #428 in Rayforge).
    pub fn signal_space_available(&self) {
        self.space_available.notify_waiters();
        self.space_available.notify_one();
    }

    /// Wait until *needed* bytes fit into the RX buffer, at most
    /// `timeout`.
    pub async fn wait_for_space(
        &self,
        needed: usize,
        timeout: Duration,
    ) -> Result<(), TimedOut> {
        let deadline = tokio::time::Instant::now() + timeout;
        while self.needs_space(needed) {
            let remaining =
                deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(TimedOut);
            }
            let notified = self.space_available.notified();
            if !self.needs_space(needed) {
                return Ok(());
            }
            tokio::select! {
                _ = notified => {},
                _ = tokio::time::sleep_until(deadline) => {
                    if !self.needs_space(needed) {
                        return Ok(());
                    }
                    return Err(TimedOut);
                }
            }
        }
        Ok(())
    }

    /// Wait until every pending command has been acknowledged.
    /// `timeout` of `None` waits indefinitely (mirroring Python's
    /// `pending_queue.join()`).
    pub async fn wait_pending_empty(
        &self,
        timeout: Option<Duration>,
    ) -> Result<(), TimedOut> {
        let deadline = timeout.map(|t| tokio::time::Instant::now() + t);
        while !self.pending_queue().is_empty() {
            let notified = self.pending_drained.notified();
            if self.pending_queue().is_empty() {
                return Ok(());
            }
            let Some(deadline) = deadline else {
                notified.await;
                continue;
            };
            let remaining =
                deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() && !self.pending_queue().is_empty() {
                return Err(TimedOut);
            }
            tokio::select! {
                _ = notified => {},
                _ = tokio::time::sleep_until(deadline) => {
                    if self.pending_queue().is_empty() {
                        return Ok(());
                    }
                    return Err(TimedOut);
                }
            }
        }
        Ok(())
    }

    /// Enqueue a pending command and account for its bytes.
    pub fn track_command(
        &self,
        length: usize,
        op_index: Option<i64>,
        command: &str,
    ) {
        let mut inner = self.inner.lock().unwrap();
        inner.pending.push_back(PendingCommand {
            length,
            op_index,
            command: command.to_string(),
        });
        inner.rx_count += length;
    }

    /// Reset flow-control state without clearing the parse buffer.
    pub fn reset_flow_control(&self) {
        {
            let mut inner = self.inner.lock().unwrap();
            inner.rx_count = 0;
            inner.pending.clear();
        }
        // Wake parked waiters rather than dropping the signal
        // objects: dropping would orphan tasks that are already
        // waiting, leaving them stranded until their stall timeout
        // fires (issue #428 in Rayforge).
        self.notify_drained();
        self.signal_space_available();
    }

    /// Reset all buffer state (cancel, reconnect, cleanup).
    pub fn reset(&self) {
        self.reset_flow_control();
        self.inner.lock().unwrap().status_buffer.clear();
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
