//! Byte transports for talking to Grbl devices.
//!
//! Platform-dependent transports keep their canonical module path in
//! every build: without the `serial` feature the `serial` module
//! compiles as a stub whose operations fail at runtime, so
//! [`TransportKind::build`] stays intact.  The telnet transport
//! dials through [`crate::driver::core::connector::dial`] and therefore runs
//! on every target; bluetooth is platform-gated with an `unsupported`
//! backend elsewhere.

pub mod bluetooth;
pub mod mock;

#[cfg(feature = "serial")]
pub mod serial;
#[cfg(not(feature = "serial"))]
#[path = "serial_unsupported.rs"]
pub mod serial;

pub mod telnet;

pub(crate) mod stream_base;

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::driver::core::transport::bluetooth::BluetoothTransport;
use crate::driver::core::transport::mock::MockInner;
use crate::driver::core::transport::serial::SerialTransport;
use crate::driver::core::transport::telnet::TelnetTransport;
use crate::driver::core::types::TransportStatus;

/// A connection-orientiated byte transport to a device.
///
/// Implementations push received bytes into the channel handed to
/// [`Transport::open`] — the session runs the receive loop.  Writes
/// must be safe to call concurrently with reads.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Open the connection.  Received bytes are delivered to `tx`
    /// until the connection is closed.  Errors map to
    /// [`TransportError::Connection`].
    async fn open(
        &self,
        tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Result<(), TransportError>;

    /// Close the connection, stopping any receive task.
    async fn close(&self);

    /// Write bytes to the device.
    async fn write(&self, data: &[u8]) -> Result<(), TransportError>;

    /// True while the connection is open.
    fn is_open(&self) -> bool;

    /// Human-readable resource identifier for this transport, e.g.
    /// `serial:///dev/ttyUSB0`.
    fn resource_uri(&self) -> Option<String> {
        None
    }

    /// Transport-level status hint after a failed operation.
    fn status_on_error(&self) -> TransportStatus {
        TransportStatus::Error
    }

    /// The job-upload capability, when the device accepts whole jobs
    /// instead of a streamed G-code body (e.g. ESP3D uploads a file
    /// and starts execution over HTTP).
    fn network_job_runner(&self) -> Option<&dyn NetworkJobRunner> {
        None
    }
}

/// Upload-and-run capability for network devices that do not stream
/// G-code line by line.
#[async_trait]
pub trait NetworkJobRunner: Send + Sync {
    async fn run_network_job(&self, gcode: &str) -> Result<(), TransportError>;
}

/// Which physical transport to use.
///
/// This is the transport registry: adding a transport means adding a
/// variant plus a [`TransportKind::build`] arm — sessions and the
/// Python config parsing stay untouched.
#[derive(Clone)]
pub enum TransportKind {
    Serial {
        port: String,
        baudrate: u32,
    },
    Telnet {
        host: String,
        port: u16,
    },
    Bluetooth {
        address: String,
        channel: u8,
    },
    Mock(Arc<MockInner>),
    /// A transport constructed by a protocol module (e.g. ESP3D).
    Custom(Arc<dyn Transport>),
}

impl TransportKind {
    /// Construct the transport implementation for this kind.
    pub fn build(&self) -> Arc<dyn Transport> {
        match self {
            TransportKind::Serial { port, baudrate } => {
                SerialTransport::new(port, *baudrate)
            }
            TransportKind::Telnet { host, port } => {
                TelnetTransport::new(host, *port)
            }
            TransportKind::Bluetooth { address, channel } => {
                BluetoothTransport::new(address, *channel)
            }
            TransportKind::Mock(inner) => inner.clone(),
            TransportKind::Custom(transport) => transport.clone(),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum TransportError {
    #[error("connection error: {0}")]
    Connection(String),
    #[error("not connected")]
    NotConnected,
}

impl From<std::io::Error> for TransportError {
    fn from(err: std::io::Error) -> Self {
        Self::Connection(err.to_string())
    }
}
