//! `GrblSerialDriver`: GRBL over a serial port.

use std::sync::Arc;

use crate::driver::core::dialect::Dialect;
use crate::driver::core::driver::{
    delegate_driver,
    info::{caps, DEFAULT_SUPPORTED_WCS},
    ConnectionKind, DriverEvents, DriverInfo, DriverMaturity, ProbeFingerprint,
};

use crate::driver::core::driver::{SetupField, SetupKind};
use crate::driver::core::transport::TransportKind;

use super::stream::{GrblStream, GrblStreamConfig};

/// Typed identity and capabilities of the serial GRBL driver.
pub static INFO: DriverInfo = DriverInfo {
    name: "GrblSerialDriver",
    connection: ConnectionKind::Serial,
    maturity: DriverMaturity::Stable,
    caps: caps(
        true, true, true, true, true, true, true, true, false, true, true,
    ),
    uses_gcode: true,
    machine_space_wcs: "G53",
    supported_wcs: DEFAULT_SUPPORTED_WCS,
    config_keys: &[
        "port",
        "baudrate",
        "poll_status_while_running",
        "deadlock_detection",
        "rx_buffer_size_override",
    ],
    setup: SETUP,
    fingerprint: Some(ProbeFingerprint {
        command: "$I",
        signature: "VER:",
    }),
};

/// A GRBL-compatible device on a serial port.
pub struct GrblSerialDriver {
    stream: Arc<GrblStream>,
}

impl GrblSerialDriver {
    /// Build a driver for *port* at *baudrate*.
    pub fn new(
        port: impl Into<String>,
        baudrate: u32,
        config: GrblStreamConfig,
        dialect: Dialect,
        events: Arc<dyn DriverEvents>,
    ) -> Arc<Self> {
        Self::with_transport(
            TransportKind::Serial {
                port: port.into(),
                baudrate,
            },
            config,
            dialect,
            events,
        )
    }

    /// Build a driver around an arbitrary transport (tests, wasm).
    pub fn with_transport(
        transport: TransportKind,
        config: GrblStreamConfig,
        dialect: Dialect,
        events: Arc<dyn DriverEvents>,
    ) -> Arc<Self> {
        Arc::new(Self {
            stream: GrblStream::new(transport, config, &INFO, dialect, events),
        })
    }
}

delegate_driver!(GrblSerialDriver, stream);

/// Connection-setup form for this driver.
pub static SETUP: &[SetupField] = &[
    SetupField::new("port", SetupKind::SerialPort).required(),
    SetupField::new("baudrate", SetupKind::Baudrate).with_default("115200"),
    SetupField::new("poll_status_while_running", SetupKind::Bool)
        .with_default("false"),
    SetupField::new("deadlock_detection", SetupKind::Bool)
        .with_default("false"),
    SetupField::new("rx_buffer_size_override", SetupKind::Int)
        .with_default("0")
        .with_range(0.0, 1024.0),
];
