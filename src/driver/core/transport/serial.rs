//! Async serial port transport backed by `tokio-serial`.
//!
//! The open/writer/reader plumbing is shared with the telnet
//! transport via [`super::stream_base`].

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_serial::{SerialPortBuilderExt, SerialStream};

use super::stream_base::StreamTransportBase;
use super::{Transport, TransportError};

pub struct SerialTransport {
    base: StreamTransportBase<SerialStream>,
    port: String,
    baudrate: u32,
}

impl SerialTransport {
    pub fn new(port: &str, baudrate: u32) -> Arc<Self> {
        Arc::new(Self {
            base: StreamTransportBase::new(),
            port: port.to_string(),
            baudrate,
        })
    }
}

#[async_trait]
impl Transport for SerialTransport {
    async fn open(
        &self,
        tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Result<(), TransportError> {
        let builder = tokio_serial::new(&self.port, self.baudrate);
        let stream: SerialStream = builder
            .open_native_async()
            .map_err(|e| TransportError::Connection(e.to_string()))?;
        self.base.start(stream, "Serial", tx).await;
        Ok(())
    }

    async fn close(&self) {
        self.base.close().await;
    }

    async fn write(&self, data: &[u8]) -> Result<(), TransportError> {
        self.base.write(data).await
    }

    fn is_open(&self) -> bool {
        self.base.is_open()
    }

    fn resource_uri(&self) -> Option<String> {
        Some(format!("serial://{}", self.port))
    }
}

/// Enumerate the serial ports this host can reach, for device
/// pickers and the remote daemon's device listing.
pub fn list_serial_ports(
) -> Result<Vec<serialport::SerialPortInfo>, TransportError> {
    serialport::available_ports()
        .map_err(|e| TransportError::Connection(e.to_string()))
}
