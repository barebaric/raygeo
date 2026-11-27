//! Shared device-core primitives.
//!
//! Everything here is protocol-neutral: byte transports, the device
//! value types, dialect command-template formatting, the driver
//! event/error primitives, and the transport-level discovery
//! scanners.  The protocol modules (`grbl`, `smoothie`, `marlin`,
//! and the network-native `esp3d`/`octoprint`/`xtool`)
//! build their drivers on top of this layer.
//!
//! Host-dependent pieces are feature-gated so a wasm build can omit
//! what a browser physically cannot do (`mdns`, the remote daemon
//! listener, the remote client): those live behind `mdns`/`net`/
//! `remote-daemon`.  The HTTP/WS clients, the telnet transport and
//! the connector seam compile everywhere — on hosts without TCP
//! sockets the embedder installs a stream connector and the same
//! code runs against browser-provided streams (see docs/wasm.md).

pub mod connector;
pub mod dialect;
pub mod discovery;
pub mod driver;
pub mod http;
pub mod line_driver;
#[cfg(any(feature = "net", feature = "mdns"))]
pub mod remote;
pub mod transport;
pub mod types;
pub mod ws;
