//! Discovery result types shared by all channels.
//!
//! A [`DiscoveredDevice`] is what every scan yields; a
//! [`DeviceIdentity`] bundles everything known about a device that
//! can be used to match it against a known device profile. Owning
//! these here lets embedding applications consume scan results
//! directly, without converting them into their own copies.

use std::collections::{BTreeSet, HashMap};

/// A connection-parameter value: serial ports and hosts are strings,
/// baud rates and TCP ports are integers.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ParamValue {
    Str(String),
    Int(i64),
}

/// Everything known about a discovered device that can be used to
/// match it against a known device profile.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DeviceIdentity {
    pub firmware: Option<String>,
    pub banner: Option<String>,
    pub usb_vid: Option<u16>,
    pub usb_pid: Option<u16>,
    pub tokens: BTreeSet<String>,
}

/// Stable identity for a device from its driver and connection
/// parameters.
///
/// Serial devices are identified by their port path, network devices
/// by host and TCP port (so two servers that both listen on 80 stay
/// distinguishable), and remote devices by daemon endpoint plus the
/// device resource (so the same port path on two daemons, or local
/// and remote, stay distinguishable). Shared by
/// [`DiscoveredDevice::key`] and discovery UIs, which compute the
/// same key from a configured machine's stored connection arguments.
pub fn device_key(
    driver_name: &str,
    params: &HashMap<String, ParamValue>,
) -> String {
    let get_str = |key: &str| match params.get(key) {
        Some(ParamValue::Str(value)) if !value.is_empty() => {
            Some(value.clone())
        }
        _ => None,
    };
    if let Some(remote_host) = get_str("remote_host") {
        let resource = get_str("device")
            .or_else(|| get_str("port"))
            .or_else(|| get_str("host"))
            .unwrap_or_default();
        let remote_port = match params.get("remote_port") {
            Some(ParamValue::Int(value)) => value.to_string(),
            _ => "None".to_string(),
        };
        return format!(
            "{driver_name}:raydriver://{remote_host}:{remote_port}:{resource}"
        );
    }
    if let Some(address) = get_str("address") {
        return format!("{driver_name}:bt://{address}");
    }
    if let Some(serial_port) = get_str("port") {
        return format!("{driver_name}:{serial_port}");
    }
    if let Some(host) = get_str("host") {
        let port = match params.get("port") {
            Some(ParamValue::Int(value)) => value.to_string(),
            Some(ParamValue::Str(value)) => value.clone(),
            _ => "None".to_string(),
        };
        return format!("{driver_name}:{host}:{port}");
    }
    let port_repr = match params.get("port") {
        Some(ParamValue::Int(value)) => value.to_string(),
        Some(ParamValue::Str(value)) => format!("'{value}'"),
        None => "None".to_string(),
    };
    format!("{driver_name}:{port_repr}")
}

/// A device found by evaluating scan output against a driver's
/// discovery declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredDevice {
    pub driver_name: String,
    /// Connection parameters that seed ``driver_args`` (port,
    /// baudrate, ...).
    pub params: HashMap<String, ParamValue>,
    /// Short display name, e.g. "GRBL device".
    pub label: String,
    /// One-line connection detail, e.g. "/dev/ttyUSB0 at 115200 baud".
    pub detail: String,
    pub identity: DeviceIdentity,
}

impl DiscoveredDevice {
    /// Stable identity for deduplication in the UI.
    pub fn key(&self) -> String {
        device_key(&self.driver_name, &self.params)
    }
}
