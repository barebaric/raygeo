//! `NoDeviceDriver`: a simulated machine for UI previews and tests.
//!
//! It speaks the full [`Driver`] contract without a transport: a job
//! "runs" with a configurable per-op delay, moves and settings are
//! accepted, and a probe cycle returns a fixed point.  This is the
//! Rust counterpart of the original `NoDeviceDriver` in Rayforge.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;

use crate::driver::core::dialect::Dialect;
use crate::driver::core::driver::{
    info::{caps, DEFAULT_SUPPORTED_WCS},
    CancelMode, ConnectionKind, Driver, DriverError, DriverEvents, DriverInfo,
    DriverMaturity,
};

use crate::driver::core::driver::{SetupField, SetupKind};
use crate::driver::core::types::{
    DeviceState, DeviceStatus, TransportStatus, UnitSystem,
};

/// Typed identity and capabilities of the simulator.
pub static INFO: DriverInfo = DriverInfo {
    name: "NoDeviceDriver",
    connection: ConnectionKind::None,
    maturity: DriverMaturity::Stable,
    caps: caps(
        false, false, false, true, true, true, true, true, false, true, false,
    ),
    uses_gcode: true,
    machine_space_wcs: "MACHINE",
    supported_wcs: DEFAULT_SUPPORTED_WCS,
    config_keys: &["step_delay", "active_wcs"],
    setup: SETUP,
    fingerprint: None,
};

/// Simulator configuration.
#[derive(Debug, Clone)]
pub struct NoDeviceConfig {
    /// Delay per simulated command, in seconds.
    pub step_delay: f64,
    /// The initially active work-coordinate system.
    pub active_wcs: String,
    /// Initial WCS offsets, keyed by slot (G54 ...).
    pub coordinate_systems: HashMap<String, (f64, f64, f64)>,
}

impl Default for NoDeviceConfig {
    fn default() -> Self {
        Self {
            step_delay: 0.01,
            active_wcs: "G54".to_string(),
            coordinate_systems: HashMap::new(),
        }
    }
}

impl NoDeviceConfig {
    /// Build the config from a JSON-style dictionary.
    pub fn from_value(value: &Value) -> Self {
        let mut config = Self::default();
        if let Some(delay) = value.get("step_delay").and_then(Value::as_f64) {
            config.step_delay = delay.max(0.0);
        }
        if let Some(wcs) = value.get("active_wcs").and_then(Value::as_str) {
            config.active_wcs = wcs.to_string();
        }
        if let Some(systems) =
            value.get("coordinate_systems").and_then(Value::as_object)
        {
            for (slot, offsets) in systems {
                let values = offsets.as_array();
                let get = |index: usize| {
                    values
                        .and_then(|values| values.get(index))
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0)
                };
                config
                    .coordinate_systems
                    .insert(slot.clone(), (get(0), get(1), get(2)));
            }
        }
        config
    }
}

/// A simulated machine with no physical device behind it.
pub struct NoDeviceDriver {
    config: NoDeviceConfig,
    events: Arc<dyn DriverEvents>,
    state: Mutex<DeviceState>,
    offsets: Mutex<HashMap<String, (f64, f64, f64)>>,
    job_running: AtomicBool,
    cancelled: AtomicBool,
}

impl NoDeviceDriver {
    pub fn new(
        config: NoDeviceConfig,
        events: Arc<dyn DriverEvents>,
    ) -> Arc<Self> {
        let mut offsets = HashMap::new();
        for slot in ["G54", "G55", "G56", "G57", "G58", "G59"] {
            let value = config
                .coordinate_systems
                .get(slot)
                .copied()
                .unwrap_or((0.0, 0.0, 0.0));
            offsets.insert(slot.to_string(), value);
        }
        Arc::new(Self {
            config,
            events,
            state: Mutex::new(DeviceState::default()),
            offsets: Mutex::new(offsets),
            job_running: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
        })
    }

    async fn delay(&self) {
        if self.config.step_delay > 0.0 {
            tokio::time::sleep(Duration::from_secs_f64(self.config.step_delay))
                .await;
        }
    }

    fn set_status(&self, status: DeviceStatus) {
        let snapshot = {
            let mut state = self.state.lock().unwrap();
            if state.status != status {
                state.status = status;
                Some(state.clone())
            } else {
                None
            }
        };
        if let Some(snapshot) = snapshot {
            self.events.state_changed(&snapshot);
        }
    }

    fn clear_device_error(&self) {
        let snapshot = {
            let mut state = self.state.lock().unwrap();
            if state.error.is_some() {
                state.error = None;
                Some(state.clone())
            } else {
                None
            }
        };
        if let Some(snapshot) = snapshot {
            self.events.state_changed(&snapshot);
        }
    }
}

#[async_trait]
impl Driver for NoDeviceDriver {
    fn info(&self) -> &'static DriverInfo {
        &INFO
    }

    fn state(&self) -> DeviceState {
        self.state.lock().unwrap().clone()
    }

    fn is_job_running(&self) -> bool {
        self.job_running.load(Ordering::SeqCst)
    }

    fn resource_uri(&self) -> Option<String> {
        None
    }

    fn update_dialect(&self, _dialect: Dialect) {}

    fn abort_background_tasks_now(&self) {}

    async fn connect(self: Arc<Self>) -> Result<(), DriverError> {
        self.events
            .connection_status(TransportStatus::Connecting, None);
        tokio::time::sleep(Duration::from_secs_f64(0.1)).await;
        self.set_status(DeviceStatus::Idle);
        self.events
            .connection_status(TransportStatus::Connected, None);
        self.events
            .wcs_updated(&self.offsets.lock().unwrap().clone());
        Ok(())
    }

    async fn cleanup(&self) {
        self.job_running.store(false, Ordering::SeqCst);
        self.events
            .connection_status(TransportStatus::Disconnected, None);
    }

    async fn run(
        self: Arc<Self>,
        gcode: &str,
        op_map: &HashMap<usize, i64>,
        _op_estimates: &[f64],
    ) -> Result<(), DriverError> {
        let _ = gcode;
        self.job_running.store(true, Ordering::SeqCst);
        self.cancelled.store(false, Ordering::SeqCst);
        self.set_status(DeviceStatus::Run);
        let mut ops: Vec<i64> = op_map.values().copied().collect();
        ops.sort_unstable();
        ops.dedup();
        for op in ops {
            if self.cancelled.load(Ordering::SeqCst) {
                break;
            }
            self.delay().await;
            self.events.command_done(op);
        }
        self.job_running.store(false, Ordering::SeqCst);
        self.set_status(DeviceStatus::Idle);
        self.events.job_finished();
        Ok(())
    }

    async fn run_raw(
        self: Arc<Self>,
        machine_code: &str,
    ) -> Result<(), DriverError> {
        self.job_running.store(true, Ordering::SeqCst);
        self.set_status(DeviceStatus::Run);
        for line in machine_code.lines() {
            log_crate::info!("{line}");
            self.delay().await;
        }
        self.job_running.store(false, Ordering::SeqCst);
        self.set_status(DeviceStatus::Idle);
        self.events.job_finished();
        Ok(())
    }

    async fn cancel(&self, _mode: CancelMode) -> Result<(), DriverError> {
        self.cancelled.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn set_hold(&self, _hold: bool) -> Result<(), DriverError> {
        Ok(())
    }

    async fn home(
        &self,
        _axes: Option<Vec<String>>,
        _active_wcs: &str,
    ) -> Result<(), DriverError> {
        Ok(())
    }

    async fn move_to(
        &self,
        _speed: f64,
        _x: f64,
        _y: f64,
        _z: Option<f64>,
    ) -> Result<(), DriverError> {
        Ok(())
    }

    async fn jog(
        &self,
        _speed: f64,
        _deltas: &[(String, f64)],
    ) -> Result<(), DriverError> {
        Ok(())
    }

    async fn select_tool(&self, _tool_number: i64) -> Result<(), DriverError> {
        Ok(())
    }

    async fn set_power(&self, power: Option<f64>) -> Result<(), DriverError> {
        log_crate::info!("Simulated power: {power:?}");
        Ok(())
    }

    async fn set_focus_power(
        &self,
        power: Option<f64>,
    ) -> Result<(), DriverError> {
        log_crate::info!("Simulated focus power: {power:?}");
        Ok(())
    }

    async fn clear_alarm(&self) -> Result<(), DriverError> {
        self.clear_device_error();
        Ok(())
    }

    async fn read_settings(
        &self,
    ) -> Result<Vec<(String, String)>, DriverError> {
        Ok(Vec::new())
    }

    async fn write_setting(
        &self,
        _key: &str,
        _value: &str,
    ) -> Result<(), DriverError> {
        Ok(())
    }

    async fn set_wcs_offset(
        &self,
        wcs_slot: &str,
        x: f64,
        y: f64,
        z: Option<f64>,
    ) -> Result<(), DriverError> {
        let mut offsets = self.offsets.lock().unwrap();
        offsets.insert(wcs_slot.to_string(), (x, y, z.unwrap_or(0.0)));
        let snapshot = offsets.clone();
        drop(offsets);
        self.events.wcs_updated(&snapshot);
        Ok(())
    }

    async fn read_wcs_offsets(
        &self,
    ) -> Result<HashMap<String, (f64, f64, f64)>, DriverError> {
        Ok(self.offsets.lock().unwrap().clone())
    }

    async fn read_parser_state(&self) -> Result<Option<String>, DriverError> {
        Ok(Some(self.config.active_wcs.clone()))
    }

    async fn detect_unit_system(
        &self,
    ) -> Result<Option<UnitSystem>, DriverError> {
        Ok(None)
    }

    async fn run_probe_cycle(
        &self,
        axis_letter: &str,
        _max_travel: f64,
        _feed_rate: f64,
    ) -> Result<Option<(f64, f64, f64)>, DriverError> {
        self.events
            .probe_status(&format!("Probing {axis_letter}..."));
        tokio::time::sleep(Duration::from_secs_f64(0.5)).await;
        self.events.probe_status("Probe complete");
        Ok(Some((10.0, 15.0, -1.0)))
    }
}

/// Connection-setup form for this driver.
pub static SETUP: &[SetupField] = &[
    SetupField::new("step_delay", SetupKind::Float)
        .with_default("0.01")
        .with_range(0.0, 10.0),
    SetupField::new("active_wcs", SetupKind::Text).with_default("G54"),
];
