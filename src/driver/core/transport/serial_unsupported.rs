//! Serial transport stub for builds without the `serial` feature
//! (e.g. wasm).  The type keeps the canonical module path so
//! [`TransportKind::build`](super::TransportKind::build) stays
//! intact; every operation fails at runtime.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::{Transport, TransportError};

const UNSUPPORTED: &str =
    "serial transport not available in this build (missing `serial` feature)";

pub struct SerialTransport {
    port: String,
    /// Kept for API parity with the real transport; unused because
    /// every operation fails before a port is opened.
    _baudrate: u32,
    open: AtomicBool,
}

impl SerialTransport {
    pub fn new(port: &str, baudrate: u32) -> Arc<Self> {
        Arc::new(Self {
            port: port.to_string(),
            _baudrate: baudrate,
            open: AtomicBool::new(false),
        })
    }
}

#[async_trait]
impl Transport for SerialTransport {
    async fn open(
        &self,
        _tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Result<(), TransportError> {
        Err(TransportError::Connection(UNSUPPORTED.to_string()))
    }

    async fn close(&self) {
        self.open.store(false, Ordering::SeqCst);
    }

    async fn write(&self, _data: &[u8]) -> Result<(), TransportError> {
        Err(TransportError::Connection(UNSUPPORTED.to_string()))
    }

    fn is_open(&self) -> bool {
        self.open.load(Ordering::SeqCst)
    }

    fn resource_uri(&self) -> Option<String> {
        Some(format!("serial://{}", self.port))
    }
}

/// Enumerate the serial ports this host can reach, for device
/// pickers and the remote daemon's device listing.
///
/// Independent of the `serial` feature: port enumeration goes through
/// the `serialport` crate, which compiles everywhere (returning an
/// error on platforms without serial support).
pub fn list_serial_ports(
) -> Result<Vec<serialport::SerialPortInfo>, TransportError> {
    serialport::available_ports()
        .map_err(|e| TransportError::Connection(e.to_string()))
}
