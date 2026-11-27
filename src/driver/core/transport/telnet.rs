//! Raw TCP transport (telnet-style) for network-attached devices
//! that speak a line protocol.
//!
//! The open/writer/reader plumbing is shared with the serial
//! transport via [`super::stream_base`]; dialing goes through
//! [`crate::driver::core::connector::dial`], so the transport compiles and
//! runs on every target — native hosts dial a plain TCP connection,
//! and hosts without TCP sockets (wasm) get whatever stream the
//! embedding application's connector produces.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::stream_base::StreamTransportBase;
use super::{Transport, TransportError};
use crate::driver::core::connector::DynStream;

/// The dial deadline; an unreachable host must not block `open`
/// indefinitely (session-level timeouts start after connect).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct TelnetTransport {
    base: StreamTransportBase<DynStream>,
    host: String,
    port: u16,
}

impl TelnetTransport {
    pub fn new(host: &str, port: u16) -> Arc<Self> {
        Arc::new(Self {
            base: StreamTransportBase::new(),
            host: host.to_string(),
            port,
        })
    }
}

#[async_trait]
impl Transport for TelnetTransport {
    async fn open(
        &self,
        tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Result<(), TransportError> {
        let stream = tokio::time::timeout(
            CONNECT_TIMEOUT,
            crate::driver::core::connector::dial(&self.host, self.port),
        )
        .await
        .map_err(|_| {
            TransportError::Connection(format!(
                "connecting to {}:{} timed out",
                self.host, self.port
            ))
        })?
        .map_err(|e| TransportError::Connection(e.to_string()))?;
        self.base.start(stream, "Telnet", tx).await;
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
        Some(format!("tcp://{}:{}", self.host, self.port))
    }
}
