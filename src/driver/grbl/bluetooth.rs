//! `GrblBluetoothDriver`: GRBL over Bluetooth SPP.

use std::sync::Arc;

use crate::driver::core::dialect::Dialect;
use crate::driver::core::driver::{
    delegate_driver,
    info::{caps, DEFAULT_SUPPORTED_WCS},
    ConnectionKind, DriverEvents, DriverInfo, DriverMaturity,
};

use crate::driver::core::driver::{SetupField, SetupKind};
use crate::driver::core::transport::TransportKind;

use super::stream::{GrblStream, GrblStreamConfig};

/// Typed identity and capabilities of the Bluetooth GRBL driver.
pub static INFO: DriverInfo = DriverInfo {
    name: "GrblBluetoothDriver",
    connection: ConnectionKind::Bluetooth,
    maturity: DriverMaturity::Experimental,
    caps: caps(
        true, true, true, true, true, true, true, true, false, true, true,
    ),
    uses_gcode: true,
    machine_space_wcs: "G53",
    supported_wcs: DEFAULT_SUPPORTED_WCS,
    config_keys: &[
        "address",
        "channel",
        "poll_status_while_running",
        "deadlock_detection",
        "rx_buffer_size_override",
    ],
    setup: SETUP,
    fingerprint: None,
};

/// A GRBL-compatible device on a Bluetooth serial channel.
pub struct GrblBluetoothDriver {
    stream: Arc<GrblStream>,
}

impl GrblBluetoothDriver {
    /// Build a driver for *address* on *channel*.
    pub fn new(
        address: impl Into<String>,
        channel: u8,
        config: GrblStreamConfig,
        dialect: Dialect,
        events: Arc<dyn DriverEvents>,
    ) -> Arc<Self> {
        Self::with_transport(
            TransportKind::Bluetooth {
                address: address.into(),
                channel,
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

delegate_driver!(GrblBluetoothDriver, stream);

/// Connection-setup form for this driver.
pub static SETUP: &[SetupField] = &[
    SetupField::new("address", SetupKind::Text).required(),
    SetupField::new("channel", SetupKind::Int)
        .with_default("1")
        .with_range(1.0, 30.0),
    SetupField::new("poll_status_while_running", SetupKind::Bool)
        .with_default("false"),
    SetupField::new("deadlock_detection", SetupKind::Bool)
        .with_default("false"),
    SetupField::new("rx_buffer_size_override", SetupKind::Int)
        .with_default("0")
        .with_range(0.0, 1024.0),
];
