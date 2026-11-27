//! Rust-native machine drivers.
//!
//! The driver layer is built on the shared geometry/ops core and is
//! split into a protocol-neutral core plus protocol modules:
//!
//! - [`core`]: shared primitives — byte transports, device value
//!   types, dialect command-template formatting, the driver
//!   event/error primitives, and the transport-level discovery
//!   scanners (serial capture, mDNS browse, subnet TCP scan).
//! - [`registry`]: the built-in driver list (one typed enum, one
//!   constructor per driver, no string registry).
//! - [`epilog`], [`esp3d`], [`grbl`], [`marlin`], [`octoprint`],
//!   [`ruida`], [`simulator`], [`smoothie`], [`xtool`]: the protocol
//!   drivers.
//!
//! The driver layer is pure Rust and must not depend on PyO3; the
//! optional `python` feature provides bindings under
//! `raygeo.driver.*`.
//!
//! Build features slice out what a wasm host physically cannot do
//! (`mdns`, the remote daemon listener, the remote client, OS serial
//! ports, the tungstenite-based M2 driver); everything else —
//! including the HTTP/WS device drivers — compiles and runs on wasm
//! against embedder-provided streams. The M2 driver's TLS roots are
//! chosen by `tls-native-roots` (default) or `tls-webpki-roots`.

pub mod core;
pub mod epilog;
pub mod esp3d;
pub mod grbl;
pub mod marlin;
pub mod octoprint;
pub mod registry;
pub mod ruida;
pub mod simulator;
pub mod smoothie;
pub mod xtool;
