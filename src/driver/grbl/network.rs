//! `GrblNetworkDriver`: GRBL behind the ESP3D / FluidNC HTTP API.

use std::sync::Arc;

use crate::driver::core::dialect::Dialect;
use crate::driver::core::driver::{
    delegate_driver,
    info::{caps, DEFAULT_SUPPORTED_WCS},
    ConnectionKind, DriverEvents, DriverInfo, DriverMaturity,
};

use crate::driver::core::driver::{SetupField, SetupKind};
use crate::driver::core::transport::TransportKind;
use crate::driver::esp3d::transport::{Esp3dConfig, Esp3dTransport};

use super::stream::{GrblStream, GrblStreamConfig};

/// A GRBL-compatible device on the FluidNC / ESP3D HTTP + WebSocket
/// interface.  Commands travel over HTTP; jobs are uploaded whole and
/// started, and status is reported over the WebSocket.
/// Typed identity and capabilities of the network GRBL driver.
pub static INFO: DriverInfo = DriverInfo {
    name: "GrblNetworkDriver",
    connection: ConnectionKind::Network,
    maturity: DriverMaturity::Stable,
    caps: caps(
        true, true, true, true, true, false, true, true, false, true, false,
    ),
    uses_gcode: true,
    machine_space_wcs: "G53",
    supported_wcs: DEFAULT_SUPPORTED_WCS,
    config_keys: &["host", "http_port", "ws_port", "protocol"],
    setup: SETUP,
    fingerprint: None,
};

/// A GRBL-compatible device on the FluidNC / ESP3D HTTP + WebSocket
/// interface.  Commands travel over HTTP; jobs are uploaded whole and
/// started, and status is reported over the WebSocket.
pub struct GrblNetworkDriver {
    stream: Arc<GrblStream>,
}

impl GrblNetworkDriver {
    /// Build a driver from the ESP3D connection parameters.
    pub fn new(
        esp3d: Esp3dConfig,
        config: GrblStreamConfig,
        dialect: Dialect,
        events: Arc<dyn DriverEvents>,
    ) -> Arc<Self> {
        Self::with_transport(
            TransportKind::Custom(Esp3dTransport::new(esp3d)),
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

delegate_driver!(GrblNetworkDriver, stream);

/// Connection-setup form for this driver.
pub static SETUP: &[SetupField] = &[
    SetupField::new("host", SetupKind::Hostname).required(),
    SetupField::new("http_port", SetupKind::Port)
        .with_default("80")
        .with_range(1.0, 65535.0),
    SetupField::new("ws_port", SetupKind::Port)
        .with_default("81")
        .with_range(1.0, 65535.0),
    SetupField::new("protocol", SetupKind::Choice)
        .with_default("esp3d")
        .with_choices(&["esp3d", "longer", "fluidnc"]),
];
