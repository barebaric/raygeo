//! The built-in driver list: one typed enum, one constructor per
//! driver, no string registry.

pub mod resources;

use std::sync::Arc;

use serde_json::Value;

use crate::driver::core::dialect::Dialect;
use crate::driver::core::driver::{
    Driver, DriverError, DriverEvents, DriverInfo,
};
use crate::driver::core::transport::TransportKind;
use crate::driver::epilog::driver::{EpilogConfig, EpilogDriver};
use crate::driver::esp3d::transport::{Esp3dConfig, ProtocolVariant};
use crate::driver::grbl::bluetooth::GrblBluetoothDriver;
use crate::driver::grbl::network::GrblNetworkDriver;
use crate::driver::grbl::serial::GrblSerialDriver;
use crate::driver::grbl::simple::GrblSimpleConfig;
use crate::driver::grbl::stream::GrblStreamConfig;
use crate::driver::grbl::telnet::GrblTelnetDriver;
use crate::driver::marlin::driver::{MarlinConfig, MarlinSerialDriver};
use crate::driver::octoprint::driver::{OctoPrintConfig, OctoPrintDriver};
use crate::driver::ruida::driver::{RuidaConfig, RuidaDriver};
use crate::driver::ruida::transport::TransportKind as RuidaTransportKind;
use crate::driver::simulator::{NoDeviceConfig, NoDeviceDriver};
use crate::driver::smoothie::driver::{SmoothieConfig, SmoothieDriver};
use crate::driver::xtool_d1::driver::{XtoolConfig, XtoolNetworkDriver};

pub use resources::ResourceGuard;

/// One built-in driver class.
///
/// Adding a driver adds a variant; the compiler then forces its
/// metadata and constructor to be provided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverKind {
    GrblSerial,
    GrblTelnet,
    GrblBluetooth,
    GrblNetwork,
    GrblSerialSimple,
    MarlinSerial,
    Smoothie,
    #[cfg(feature = "m2")]
    XtoolM2,
    XtoolNetwork,
    OctoPrint,
    Epilog,
    Ruida,
    NoDevice,
}

impl DriverKind {
    /// Every driver this build can create.
    pub const ALL: &'static [DriverKind] = &[
        DriverKind::GrblSerial,
        DriverKind::GrblTelnet,
        DriverKind::GrblBluetooth,
        DriverKind::GrblNetwork,
        DriverKind::GrblSerialSimple,
        DriverKind::MarlinSerial,
        DriverKind::Smoothie,
        #[cfg(feature = "m2")]
        DriverKind::XtoolM2,
        DriverKind::XtoolNetwork,
        DriverKind::OctoPrint,
        DriverKind::Epilog,
        DriverKind::Ruida,
        DriverKind::NoDevice,
    ];

    /// Resolve a persisted driver class name.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|kind| kind.info().name == name)
    }

    /// The class name, which is also the persisted driver name.
    pub fn name(self) -> &'static str {
        self.info().name
    }

    /// The driver's typed metadata.
    pub fn info(self) -> &'static DriverInfo {
        match self {
            Self::GrblSerial => &crate::driver::grbl::serial::INFO,
            Self::GrblTelnet => &crate::driver::grbl::telnet::INFO,
            Self::GrblBluetooth => &crate::driver::grbl::bluetooth::INFO,
            Self::GrblNetwork => &crate::driver::grbl::network::INFO,
            Self::GrblSerialSimple => &crate::driver::grbl::simple::INFO,
            Self::MarlinSerial => &crate::driver::marlin::driver::INFO,
            Self::Smoothie => &crate::driver::smoothie::driver::INFO,
            #[cfg(feature = "m2")]
            Self::XtoolM2 => &crate::driver::xtool_m2::driver::INFO,
            Self::XtoolNetwork => &crate::driver::xtool_d1::driver::INFO,
            Self::OctoPrint => &crate::driver::octoprint::driver::INFO,
            Self::Epilog => &crate::driver::epilog::driver::INFO,
            Self::Ruida => &crate::driver::ruida::driver::INFO,
            Self::NoDevice => &crate::driver::simulator::INFO,
        }
    }

    /// Validate a config before a driver is built.
    pub fn precheck(self, config: &Value) -> Result<(), String> {
        match self {
            Self::GrblSerial | Self::GrblSerialSimple | Self::MarlinSerial => {
                require(config, "port")
            }
            Self::GrblTelnet
            | Self::GrblNetwork
            | Self::Smoothie
            | Self::OctoPrint
            | Self::XtoolNetwork
            | Self::Epilog => require(config, "host"),
            #[cfg(feature = "m2")]
            Self::XtoolM2 => require(config, "host"),
            Self::Ruida => {
                let host = string(config, "host").unwrap_or_default();
                let device = string(config, "usb_device").unwrap_or_default();
                if host.is_empty() && device.is_empty() {
                    return Err(
                        "a host or a USB device must be configured".into()
                    );
                }
                Ok(())
            }
            Self::NoDevice => Ok(()),
            Self::GrblBluetooth => {
                let address = string(config, "address").unwrap_or_default();
                crate::driver::core::transport::bluetooth::normalize_address(
                    &address,
                )
                .map_err(|err| err.to_string())?;
                let channel = int(config, "channel").unwrap_or(1);
                if !(1..=30).contains(&channel) {
                    return Err(
                        "Bluetooth channel must be between 1 and 30".into()
                    );
                }
                Ok(())
            }
        }
    }

    /// Build the driver from a JSON-style config.
    pub fn create(
        self,
        config: &Value,
        dialect: Dialect,
        events: Arc<dyn DriverEvents>,
    ) -> Result<Arc<dyn Driver>, DriverError> {
        match self {
            Self::GrblSerial => {
                let port = string(config, "port").unwrap_or_default();
                let baudrate =
                    uint(config, "baudrate").unwrap_or(115200) as u32;
                Ok(GrblSerialDriver::new(
                    port,
                    baudrate,
                    grbl_stream_config(config),
                    dialect,
                    events,
                ))
            }
            Self::GrblTelnet => Ok(GrblTelnetDriver::new(
                string(config, "host").unwrap_or_default(),
                uint(config, "tcp_port").unwrap_or(23) as u16,
                grbl_stream_config(config),
                dialect,
                events,
            )),
            Self::GrblBluetooth => {
                let address = string(config, "address").unwrap_or_default();
                let address =
                    crate::driver::core::transport::bluetooth::normalize_address(
                        &address,
                    )
                    .map_err(DriverError::Other)?;
                let channel = uint(config, "channel").unwrap_or(1);
                Ok(GrblBluetoothDriver::new(
                    address,
                    channel as u8,
                    grbl_stream_config(config),
                    dialect,
                    events,
                ))
            }
            Self::GrblNetwork => {
                let mut esp3d = Esp3dConfig::new(
                    &string(config, "host").unwrap_or_default(),
                );
                esp3d.http_port =
                    uint(config, "http_port").unwrap_or(80) as u16;
                esp3d.ws_port = uint(config, "ws_port").unwrap_or(81) as u16;
                esp3d.protocol = ProtocolVariant::from_name(
                    &string(config, "protocol").unwrap_or_default(),
                );
                if let Some(timeout) = float(config, "request_timeout") {
                    esp3d.request_timeout = timeout;
                }
                if let Some(timeout) = float(config, "connect_timeout") {
                    esp3d.connect_timeout = timeout;
                }
                Ok(GrblNetworkDriver::new(
                    esp3d,
                    grbl_stream_config(config),
                    dialect,
                    events,
                ))
            }
            Self::GrblSerialSimple => {
                let port = string(config, "port").unwrap_or_default();
                let baudrate =
                    uint(config, "baudrate").unwrap_or(115200) as u32;
                let mut simple = GrblSimpleConfig::new();
                if let Some(v) = boolean(config, "poll_status_while_running") {
                    simple.poll_status_while_running = v;
                }
                simple.command_timeout =
                    float(config, "command_timeout").unwrap_or(30.0);
                simple.homing_timeout =
                    float(config, "homing_timeout").unwrap_or(0.0);
                Ok(crate::driver::grbl::simple::GrblSerialSimpleDriver::new(
                    TransportKind::Serial { port, baudrate },
                    simple,
                    dialect,
                    events,
                ))
            }
            Self::MarlinSerial => {
                let port = string(config, "port").unwrap_or_default();
                let baudrate =
                    uint(config, "baudrate").unwrap_or(115200) as u32;
                let mut marlin =
                    MarlinConfig::new(TransportKind::Serial { port, baudrate });
                if let Some(v) = boolean(config, "poll_status_while_running") {
                    marlin.poll_status_while_running = v;
                }
                if let Some(v) = float(config, "command_timeout") {
                    marlin.command_timeout = v;
                }
                if let Some(v) = float(config, "connect_settle") {
                    marlin.connect_settle = v;
                }
                Ok(MarlinSerialDriver::new(marlin, dialect, events))
            }
            Self::Smoothie => {
                let transport = TransportKind::Telnet {
                    host: string(config, "host").unwrap_or_default(),
                    port: uint(config, "tcp_port").unwrap_or(23) as u16,
                };
                Ok(SmoothieDriver::new(
                    SmoothieConfig::new(transport),
                    dialect,
                    events,
                ))
            }
            #[cfg(feature = "m2")]
            Self::XtoolM2 => {
                let mut m2 = crate::driver::xtool_m2::driver::M2Config::new(
                    &string(config, "host").unwrap_or_default(),
                );
                m2.port = uint(config, "port").unwrap_or(28900) as u16;
                if let Some(v) = boolean(config, "tls_allow_invalid_certs") {
                    m2.tls_allow_invalid_certs = v;
                }
                Ok(crate::driver::xtool_m2::driver::XtoolM2Driver::new(
                    m2, events,
                ))
            }
            Self::XtoolNetwork => {
                let mut xtool = XtoolConfig::new(
                    &string(config, "host").unwrap_or_default(),
                );
                xtool.http_port =
                    uint(config, "http_port").unwrap_or(8080) as u16;
                xtool.ws_port = uint(config, "ws_port").unwrap_or(8081) as u16;
                Ok(XtoolNetworkDriver::new(xtool, dialect, events))
            }
            Self::OctoPrint => {
                let mut octoprint = OctoPrintConfig::new(
                    &string(config, "host").unwrap_or_default(),
                );
                octoprint.port = uint(config, "port").unwrap_or(80) as u16;
                if let Some(v) = string(config, "path") {
                    octoprint.path_prefix =
                        crate::driver::octoprint::driver::normalize_path_prefix(
                            &v,
                        );
                }
                if let Some(v) = string(config, "api_key") {
                    octoprint.api_key = v;
                }
                Ok(OctoPrintDriver::new(octoprint, events))
            }
            Self::Ruida => {
                let magic = match string(config, "magic_number") {
                    Some(raw) => parse_magic(&raw)?,
                    None => crate::driver::ruida::driver::DEFAULT_MAGIC,
                };
                let ruida = RuidaConfig {
                    host: string(config, "host")
                        .filter(|value| !value.is_empty()),
                    usb_device: string(config, "usb_device")
                        .filter(|value| !value.is_empty()),
                    port: uint(config, "port").unwrap_or(50200) as u16,
                    protocol: match string(config, "protocol").as_deref() {
                        Some("tcp") => RuidaTransportKind::Tcp,
                        _ => RuidaTransportKind::Udp,
                    },
                    magic,
                    protect: boolean(config, "protect").unwrap_or(true),
                    timeout: float(config, "timeout")
                        .map(|value| {
                            std::time::Duration::from_millis(
                                (value * 1000.0) as u64,
                            )
                        })
                        .unwrap_or_default(),
                    query_interval: float(config, "query_interval")
                        .map(|value| {
                            std::time::Duration::from_secs_f64(value.max(0.05))
                        })
                        .unwrap_or_default(),
                    ping_retries: uint(config, "ping_retries").unwrap_or(10)
                        as u32,
                    ping_delay: float(config, "ping_delay")
                        .map(|value| {
                            std::time::Duration::from_secs_f64(value.max(0.0))
                        })
                        .unwrap_or_default(),
                    ..RuidaConfig::default()
                };
                Ok(RuidaDriver::new(ruida, events))
            }
            Self::NoDevice => Ok(NoDeviceDriver::new(
                NoDeviceConfig::from_value(config),
                events,
            )),
            Self::Epilog => {
                let mut epilog = EpilogConfig::new(
                    &string(config, "host").unwrap_or_default(),
                );
                epilog.port = uint(config, "port").unwrap_or(515) as u16;
                if let Some(v) = string(config, "queue") {
                    epilog.queue = v;
                }
                if let Some(v) = float(config, "timeout") {
                    epilog.timeout = v;
                }
                Ok(Arc::new(EpilogDriver::new(epilog, events)))
            }
        }
    }
}

fn require(config: &Value, key: &str) -> Result<(), String> {
    match config.get(key).and_then(Value::as_str) {
        Some(value) if !value.is_empty() => Ok(()),
        _ => Err(format!("{key} must be configured")),
    }
}

fn string(config: &Value, key: &str) -> Option<String> {
    config.get(key).and_then(Value::as_str).map(str::to_string)
}

fn int(config: &Value, key: &str) -> Option<i64> {
    config.get(key).and_then(|value| {
        value.as_i64().or_else(|| value.as_f64().map(|v| v as i64))
    })
}

fn uint(config: &Value, key: &str) -> Option<u64> {
    int(config, key).map(|value| value.max(0) as u64)
}

fn float(config: &Value, key: &str) -> Option<f64> {
    config.get(key).and_then(Value::as_f64)
}

fn boolean(config: &Value, key: &str) -> Option<bool> {
    config.get(key).and_then(Value::as_bool)
}

fn grbl_stream_config(config: &Value) -> GrblStreamConfig {
    let mut tuning = GrblStreamConfig::new();
    if let Some(v) = boolean(config, "poll_status_while_running") {
        tuning.poll_status_while_running = v;
    }
    if let Some(v) = boolean(config, "deadlock_detection") {
        tuning.deadlock_detection = v;
    }
    if let Some(v) = int(config, "rx_buffer_size_override") {
        tuning.rx_buffer_size_override = v;
    }
    if let Some(v) = int(config, "cached_rx_buffer_size") {
        tuning.cached_rx_buffer_size = (v > 0).then_some(v);
    }
    if let Some(v) = float(config, "stall_timeout_min") {
        tuning.stall_timeout_min = v;
    }
    if let Some(v) = float(config, "stall_timeout_max") {
        tuning.stall_timeout_max = v;
    }
    if let Some(v) = float(config, "stall_timeout_safety_factor") {
        tuning.stall_timeout_safety_factor = v;
    }
    if let Some(v) = float(config, "stall_timeout_default") {
        tuning.stall_timeout_default = v;
    }
    if let Some(v) = float(config, "safety_shutdown_delay") {
        tuning.safety_shutdown_delay = v;
    }
    if let Some(v) = float(config, "handshake_timeout") {
        tuning.handshake_timeout = v;
    }
    if let Some(v) = float(config, "handshake_poll_interval") {
        tuning.handshake_poll_interval = v;
    }
    if let Some(v) = float(config, "status_poll_interval") {
        tuning.status_poll_interval = v;
    }
    if let Some(v) = float(config, "reconnect_delay") {
        tuning.reconnect_delay = v;
    }
    if let Some(v) = float(config, "command_timeout") {
        tuning.command_timeout = v;
    }
    if let Some(v) = float(config, "homing_timeout") {
        tuning.homing_timeout = v;
    }
    tuning
}

fn parse_magic(raw: &str) -> Result<u8, DriverError> {
    let raw = raw.trim();
    let value = if let Some(hex) =
        raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X"))
    {
        u8::from_str_radix(hex, 16).ok()
    } else {
        raw.parse::<u8>().ok()
    };
    value.ok_or_else(|| {
        DriverError::Device(format!(
            "magic number must be between 0x00 and 0xFF: {raw:?}"
        ))
    })
}
