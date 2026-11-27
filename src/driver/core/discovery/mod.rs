//! Transport-level device discovery scanners.
//!
//! Protocol-neutral scans that capture raw evidence about attached
//! devices: serial port output, mDNS service announcements and
//! reachable TCP hosts on a subnet. Interpreting the evidence —
//! deciding which driver could talk to a device — lives with the
//! embedding application's drivers, never here.
//!
//! The TCP-based scanners ([`daemon`], [`probe`], [`subnet`]) need
//! the `net` feature; [`mdns`] needs `mdns`.  Serial enumeration and
//! capture compile everywhere (they degrade to errors without a
//! serial backend).

#[cfg(feature = "net")]
pub mod daemon;
#[cfg(feature = "mdns")]
pub mod mdns;
pub mod ports;
#[cfg(feature = "net")]
pub mod probe;
pub mod serial;
#[cfg(feature = "net")]
pub mod subnet;
pub mod tokens;
pub mod types;
