//! Pluggable stream dialing: how byte streams to `(host, port)` are
//! produced.
//!
//! The HTTP and WebSocket clients, the telnet transport and the
//! network-native sessions all go through [`dial`] and therefore
//! compile on *every* target.  On native hosts the default is a
//! plain `TcpStream` (behind the `net` feature); on hosts without
//! TCP sockets — wasm — the embedding application installs a
//! [`StreamConnector`] up front that produces the stream from
//! whatever the platform provides (a WebSocket-to-TCP bridge, a
//! Web-Serial-backed socket, a in-process test server, ...).
//!
//! Without a connector (and without `net`) every dial fails with a
//! descriptive error instead of the module being compiled out — the
//! session code is identical on both sides.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncRead, AsyncWrite};

/// A byte stream usable by the HTTP/WS codecs and the transports:
/// object-safe umbrella over `AsyncRead + AsyncWrite + Unpin + Send`.
pub trait ByteStream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> ByteStream for T {}

/// A platform-agnostic byte stream: what [`dial`] produces and what
/// the HTTP/WS codecs are generic over.
pub type DynStream = Box<dyn ByteStream>;

/// Produces byte streams connected to a `(host, port)` endpoint.
///
/// The returned stream must behave like a raw TCP connection to
/// that endpoint: bytes written are delivered to the endpoint and
/// bytes read are what the endpoint sends back.  (For a browser
/// embedding, a WebSocket-to-TCP bridge has exactly this
/// behavior.)
pub trait StreamConnector: Send + Sync {
    fn connect(
        &self,
        host: &str,
        port: u16,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<DynStream>> + Send + '_>>;
}

static CONNECTOR: Mutex<Option<Arc<dyn StreamConnector>>> = Mutex::new(None);

/// Install (or remove, with `None`) the process-wide stream
/// connector.  Installs take effect immediately for subsequent
/// dials.
pub fn set_stream_connector(connector: Option<Arc<dyn StreamConnector>>) {
    *CONNECTOR.lock().unwrap() = connector;
}

/// The installed connector, if any.
pub fn stream_connector() -> Option<Arc<dyn StreamConnector>> {
    CONNECTOR.lock().unwrap().clone()
}

/// Open a byte stream to `(host, port)`.
///
/// Order of preference: an installed [`StreamConnector`], then (on
/// native builds with the `net` feature) a plain TCP connection.
pub(crate) async fn dial(host: &str, port: u16) -> std::io::Result<DynStream> {
    let connector = stream_connector();
    if let Some(connector) = connector {
        return connector.connect(host, port).await;
    }
    dial_default(host, port).await
}

#[cfg(feature = "net")]
async fn dial_default(host: &str, port: u16) -> std::io::Result<DynStream> {
    let stream = tokio::net::TcpStream::connect((host, port)).await?;
    Ok(Box::new(stream))
}

#[cfg(not(feature = "net"))]
async fn dial_default(_host: &str, _port: u16) -> std::io::Result<DynStream> {
    Err(std::io::Error::other(
        "no stream connector installed: call \
         core::connector::set_stream_connector from the embedding \
         application (or enable the `net` feature on native targets)",
    ))
}
