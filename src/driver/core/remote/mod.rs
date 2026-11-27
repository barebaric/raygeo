//! Remote sessions: a device-host daemon and a client proxy.
//!
//! The wire protocol ([`protocol`]), the hosted-session traits
//! ([`session`]) and the advertised-surface metadata ([`registry`])
//! are pure and compile everywhere.  The client needs `net`, the
//! daemon needs `remote-daemon` (which implies `net`), and the mDNS
//! announce/browse needs `mdns`.

#[cfg(feature = "net")]
pub mod client;
#[cfg(feature = "remote-daemon")]
pub mod daemon;
#[cfg(feature = "net")]
pub mod inspect;
#[cfg(feature = "mdns")]
pub mod mdns;
pub mod protocol;
pub mod registry;
pub mod session;

/// The raydriver version reported in the daemon welcome.
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
