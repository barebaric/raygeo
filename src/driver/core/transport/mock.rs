//! Mock transport for tests, exposable to Python.
//!
//! Scripts received bytes on demand (`push`) and records everything
//! sent.  Write failures can be injected to simulate connection
//! loss.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::{Transport, TransportError};

pub struct MockInner {
    open: AtomicBool,
    tx: Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>,
    sent: Mutex<(Vec<Vec<u8>>, usize)>,
    write_error: Mutex<Option<String>>,
}

impl MockInner {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            open: AtomicBool::new(false),
            tx: Mutex::new(None),
            sent: Mutex::new((Vec::new(), 0)),
            write_error: Mutex::new(None),
        })
    }

    /// Deliver bytes to the session as if the device had sent them.
    pub fn push(&self, data: Vec<u8>) {
        if let Some(tx) = self.tx.lock().unwrap().as_ref() {
            let _ = tx.send(data);
        }
    }

    /// All bytes written to the device so far.
    pub fn sent(&self) -> Vec<Vec<u8>> {
        self.sent.lock().unwrap().0.clone()
    }

    /// Writes recorded since the previous call (or the last
    /// `clear_sent`).  A monotonic cursor, so clearing the log
    /// between calls never skips or replays chunks.
    pub fn take_new_sent(&self) -> Vec<Vec<u8>> {
        let mut guard = self.sent.lock().unwrap();
        let new = guard.0[guard.1..].to_vec();
        guard.1 = guard.0.len();
        new
    }

    /// Clear the recorded sent bytes.
    pub fn clear_sent(&self) {
        let mut guard = self.sent.lock().unwrap();
        guard.0.clear();
        guard.1 = 0;
    }

    /// Simulate connect/disconnect of the physical link.
    pub fn set_open(&self, open: bool) {
        self.open.store(open, Ordering::SeqCst);
    }

    /// Fail subsequent writes with this message (and mark the link
    /// down), simulating an unplugged cable.
    pub fn set_write_error(&self, message: Option<String>) {
        *self.write_error.lock().unwrap() = message;
    }
}

impl Default for MockInner {
    fn default() -> Self {
        Self {
            open: AtomicBool::new(false),
            tx: Mutex::new(None),
            sent: Mutex::new((Vec::new(), 0)),
            write_error: Mutex::new(None),
        }
    }
}

#[async_trait]
impl Transport for MockInner {
    async fn open(
        &self,
        tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Result<(), TransportError> {
        *self.tx.lock().unwrap() = Some(tx);
        self.open.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn close(&self) {
        self.open.store(false, Ordering::SeqCst);
        *self.tx.lock().unwrap() = None;
    }

    async fn write(&self, data: &[u8]) -> Result<(), TransportError> {
        if !self.is_open() {
            return Err(TransportError::NotConnected);
        }
        if let Some(message) = self.write_error.lock().unwrap().clone() {
            self.open.store(false, Ordering::SeqCst);
            return Err(TransportError::Connection(message));
        }
        self.sent.lock().unwrap().0.push(data.to_vec());
        Ok(())
    }

    fn is_open(&self) -> bool {
        self.open.load(Ordering::SeqCst)
    }
}
