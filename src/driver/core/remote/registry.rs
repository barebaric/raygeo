//! The typed driver table a daemon can host.
//!
//! Core stays protocol-neutral: the *set* of hostable drivers is
//! supplied by the embedding application as a [`Registry`] at daemon
//! construction.  Each entry is a driver's own typed [`DriverInfo`];
//! there are no method-name strings to drift from the implementation.

use serde_json::{json, Value};

use crate::driver::core::driver::DriverInfo;

/// The drivers this daemon can host, in serving order.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    drivers: Vec<&'static DriverInfo>,
}

impl Registry {
    /// Build a registry from driver metadata, in serving order.
    pub fn new(drivers: Vec<&'static DriverInfo>) -> Self {
        Self { drivers }
    }

    /// True when *name* is hostable.
    pub fn is_known(&self, name: &str) -> bool {
        self.driver_info(name).is_some()
    }

    /// Look up a driver's metadata by class name.
    pub fn driver_info(&self, name: &str) -> Option<&'static DriverInfo> {
        self.drivers.iter().copied().find(|info| info.name == name)
    }

    /// The driver table, in serving order.
    pub fn drivers(&self) -> &[&'static DriverInfo] {
        &self.drivers
    }

    /// The `driver/list` payload: every driver's metadata.
    pub fn drivers_payload(&self) -> Value {
        json!({
            "drivers": self
                .drivers
                .iter()
                .map(|info| info.to_json())
                .collect::<Vec<_>>(),
        })
    }
}
