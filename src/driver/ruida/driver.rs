//! The Ruida direct session: USB, UDP and TCP.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Mutex as AsyncMutex;

use crate::driver::core::driver::{
    CancelMode, Capabilities, ConnectionKind, Driver, DriverError,
    DriverEvents, DriverInfo, DriverMaturity, SetupField, SetupKind,
};
use crate::driver::core::types::{
    DeviceState, DeviceStatus, TransportStatus, UnitSystem,
};
use crate::driver::ruida::codec::{self};
use crate::driver::ruida::status::MachineStatus;
use crate::driver::ruida::transport::{
    open_link, Link, RuidaTransport, TappedLink, TransportKind,
};

/// The controller's Ruida network port.
pub const DEFAULT_PORT: u16 = 50200;
/// The usual controller swizzle key.
pub const DEFAULT_MAGIC: u8 = 0x88;
/// The memory address of the machine status word.
const ADDRESS_MACHINE_STATUS: u16 = 0x0400;
/// The memory address of the controller card ID.
const ADDRESS_CARD_ID: u16 = 0x057E;

/// Connection parameters for a Ruida controller.
#[derive(Debug, Clone)]
pub struct RuidaConfig {
    pub host: Option<String>,
    pub usb_device: Option<String>,
    pub magic: u8,
    pub protocol: TransportKind,
    /// The controller's network port.
    pub port: u16,
    /// Blocks ``SET_SETTING`` unless explicitly disabled.
    pub protect: bool,
    pub timeout: Duration,
    pub query_interval: Duration,
    pub reconnect_delay: Duration,
    pub ping_retries: u32,
    pub ping_delay: Duration,
}

impl Default for RuidaConfig {
    fn default() -> Self {
        Self {
            host: None,
            usb_device: None,
            magic: DEFAULT_MAGIC,
            protocol: TransportKind::Udp,
            port: DEFAULT_PORT,
            protect: true,
            timeout: Duration::from_millis(500),
            query_interval: Duration::from_secs(1),
            reconnect_delay: Duration::from_secs(1),
            ping_retries: 10,
            ping_delay: Duration::from_secs(1),
        }
    }
}

impl RuidaConfig {
    /// The link this configuration connects over. USB wins when a
    /// device is configured.
    pub fn transport(&self) -> TransportKind {
        if self.usb_device.is_some() {
            TransportKind::Usb
        } else {
            self.protocol
        }
    }
}

/// The Ruida setup form.
pub static SETUP: &[SetupField] = &[
    SetupField::new("host", SetupKind::Hostname)
        .with_default("")
        .required(),
    SetupField::new("usb_device", SetupKind::SerialPort),
    SetupField::new("protocol", SetupKind::Choice)
        .with_default("udp")
        .with_choices(&["udp", "tcp"]),
    SetupField::new("magic_number", SetupKind::Text).with_default("0x88"),
    SetupField::new("protect", SetupKind::Bool).with_default("true"),
];

/// Typed identity and capabilities of this driver.
pub static INFO: DriverInfo = DriverInfo {
    name: "RuidaDriver",
    connection: ConnectionKind::Network,
    maturity: DriverMaturity::Experimental,
    caps: caps(),
    uses_gcode: false,
    machine_space_wcs: "MACHINE",
    supported_wcs: &["MACHINE", "ANCHOR", "CURRENT", "SET_POINT"],
    config_keys: &["host", "usb_device", "protocol", "magic_number", "protect"],
    setup: SETUP,
    fingerprint: None,
};

const fn caps() -> Capabilities {
    crate::driver::core::driver::info::caps(
        false, false, true, true, true, false, true, true, true, false, false,
    )
}

/// A direct connection to a Ruida laser controller.
pub struct RuidaDriver {
    config: RuidaConfig,
    events: Arc<dyn DriverEvents>,
    state: Mutex<DeviceState>,
    status: Mutex<MachineStatus>,
    transport: Arc<AsyncMutex<Option<RuidaTransport>>>,
    connection_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    keep_running: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
    protect: AtomicBool,
    jog_speeds: Mutex<JogSpeeds>,
}

impl RuidaDriver {
    /// Creates a session with an event sink.
    pub fn new(
        config: RuidaConfig,
        events: Arc<dyn DriverEvents>,
    ) -> Arc<Self> {
        let protect = AtomicBool::new(config.protect);
        Arc::new(Self {
            config,
            events,
            state: Mutex::new(DeviceState::default()),
            status: Mutex::new(MachineStatus::default()),
            transport: Arc::new(AsyncMutex::new(None)),
            connection_task: Mutex::new(None),
            keep_running: Arc::new(AtomicBool::new(false)),
            connected: Arc::new(AtomicBool::new(false)),
            protect,
            jog_speeds: Mutex::new(JogSpeeds {
                xy: 100.0,
                z: 100.0,
                u: 100.0,
            }),
        })
    }

    /// The configuration this session was created with.
    pub fn config(&self) -> &RuidaConfig {
        &self.config
    }

    /// Whether the controller answered the last status query.
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Enables or disables protect mode, which blocks
    /// ``SET_SETTING`` commands.
    pub fn set_protect(&self, enabled: bool) {
        self.protect.store(enabled, Ordering::SeqCst);
    }

    /// Whether protect mode blocks ``SET_SETTING``.
    pub fn protect_enabled(&self) -> bool {
        self.protect.load(Ordering::SeqCst)
    }

    /// The last decoded machine status.
    pub fn machine_status(&self) -> MachineStatus {
        self.status.lock().unwrap().clone()
    }

    fn set_connection(&self, status: TransportStatus, message: Option<&str>) {
        self.events.connection_status(status, message);
    }

    fn publish(&self) {
        let status = self.status.lock().unwrap().clone();
        let mut state = self.state.lock().unwrap();
        let next = status.device_status();
        let position =
            [status.position[0], status.position[1], status.position[2]];
        let changed =
            state.status != next || state.machine_pos.as_slice() != position;
        state.status = next;
        state.machine_pos = position.to_vec();
        if changed {
            let snapshot = state.clone();
            drop(state);
            self.events.state_changed(&snapshot);
        }
    }

    fn apply_replies(&self, replies: Vec<codec::Reply>) {
        let mut status = self.status.lock().unwrap();
        let mut changed = false;
        for reply in &replies {
            changed |= status.apply(reply);
        }
        drop(status);
        if changed {
            self.publish();
        }
    }

    async fn send_raw(&self, raw: &[u8]) -> Result<(), DriverError> {
        let mut guard = self.transport.lock().await;
        let transport = guard.as_mut().ok_or_else(|| {
            DriverError::Connection("controller is not connected".to_string())
        })?;
        transport.send(raw).await?;
        let replies = transport.take_replies();
        drop(guard);
        self.apply_replies(replies);
        Ok(())
    }

    async fn send_lines(&self, lines: &[String]) -> Result<(), DriverError> {
        let commands = codec::parse_script(lines).map_err(codec_error)?;
        let commands: Vec<_> = commands
            .into_iter()
            .filter(|command| {
                !(self.protect_enabled() && command.mnemonic == "SET_SETTING")
            })
            .collect();
        let raw = codec::encode_commands(&commands).map_err(codec_error)?;
        self.send_raw(&raw).await
    }

    async fn send_script(&self, script: &str) -> Result<(), DriverError> {
        let lines: Vec<String> = script
            .lines()
            .map(str::to_string)
            .filter(|line| !line.trim().is_empty())
            .collect();
        if !lines.is_empty() {
            self.send_lines(&lines).await?;
        }
        self.events.job_finished();
        Ok(())
    }

    /// Sends one rpascript command.
    pub async fn execute(&self, line: &str) -> Result<(), DriverError> {
        self.send_lines(&[line.to_string()]).await
    }

    /// Runs a full rpascript job.
    pub async fn run_script(&self, script: &str) -> Result<(), DriverError> {
        self.send_script(script).await
    }

    /// Moves the XY axes to the current origin.
    pub async fn home_xy(&self) -> Result<(), DriverError> {
        self.jog_xy_to(0.0, 0.0).await
    }

    /// Homes the Z axis through the controller's homing routine.
    pub async fn home_z(&self) -> Result<(), DriverError> {
        self.execute("HOME_Z").await
    }

    /// Homes the U axis.
    pub async fn home_u(&self) -> Result<(), DriverError> {
        self.execute("HOME_U").await
    }

    /// Runs the controller's Z auto-focus routine.
    pub async fn focus_z(&self) -> Result<(), DriverError> {
        self.execute("FOCUS_Z").await
    }

    /// Stops the job and homes XY, like the panel's reset.
    pub async fn reset(&self) -> Result<(), DriverError> {
        self.execute("STOP_JOB").await?;
        self.execute("HOME_XY").await
    }

    /// Sets the jog speeds in mm/s.
    pub fn set_jog_speeds(&self, xy: f64, z: f64, u: f64) {
        self.jog_speeds.lock().unwrap().update(xy, z, u);
    }

    /// Jogs to an absolute XY position in mm.
    pub async fn jog_xy_to(&self, x: f64, y: f64) -> Result<(), DriverError> {
        let speed = self.jog_speeds.lock().unwrap().xy;
        self.send_lines(&[
            format!("SPEED_LASER_1 {speed}"),
            format!("JOG_XY Rel:MACHINE X={x}mm Y={y}mm"),
        ])
        .await
    }

    /// Jogs to an absolute X position in mm.
    pub async fn jog_x_to(&self, x: f64) -> Result<(), DriverError> {
        let speed = self.jog_speeds.lock().unwrap().xy;
        self.send_lines(&[
            format!("SPEED_LASER_1 {speed}"),
            format!("JOG_X Rel:MACHINE X={x}mm"),
        ])
        .await
    }

    /// Jogs to an absolute Y position in mm.
    pub async fn jog_y_to(&self, y: f64) -> Result<(), DriverError> {
        let speed = self.jog_speeds.lock().unwrap().xy;
        self.send_lines(&[
            format!("SPEED_LASER_1 {speed}"),
            format!("JOG_Y Rel:MACHINE Y={y}mm"),
        ])
        .await
    }

    /// Jogs to an absolute Z position in mm.
    pub async fn jog_z_to(&self, z: f64) -> Result<(), DriverError> {
        let speed = self.jog_speeds.lock().unwrap().z;
        self.send_lines(&[
            format!("SPEED_AXIS {speed}"),
            format!("JOG_Z Rel:MACHINE Z={z}mm"),
        ])
        .await
    }

    /// Jogs to an absolute U position in mm.
    pub async fn jog_u_to(&self, u: f64) -> Result<(), DriverError> {
        let speed = self.jog_speeds.lock().unwrap().u;
        self.send_lines(&[
            format!("SPEED_AXIS {speed}"),
            format!("JOG_U Rel:MACHINE U={u}mm"),
        ])
        .await
    }

    /// Jogs XY relatively in mm.
    pub async fn jog_xy_rel(
        &self,
        x: Option<f64>,
        y: Option<f64>,
    ) -> Result<(), DriverError> {
        let speed = self.jog_speeds.lock().unwrap().xy;
        self.send_lines(&[
            format!("SPEED_LASER_1 {speed}"),
            format!(
                "JOG_XY Rel:CURRENT X={}mm Y={}mm",
                x.unwrap_or(0.0),
                y.unwrap_or(0.0)
            ),
        ])
        .await
    }

    /// Jogs one axis relatively in mm.
    pub async fn jog_axis_rel(
        &self,
        axis: char,
        delta: f64,
    ) -> Result<(), DriverError> {
        let axis = axis.to_ascii_uppercase();
        let speed = {
            let speeds = self.jog_speeds.lock().unwrap();
            match axis {
                'Z' => speeds.z,
                'U' => speeds.u,
                _ => speeds.xy,
            }
        };
        self.send_lines(&[
            format!("SPEED_AXIS {speed}"),
            format!("JOG_{axis} Rel:CURRENT {axis}={delta}mm"),
        ])
        .await
    }

    /// Pauses the running job.
    pub async fn pause(&self) -> Result<(), DriverError> {
        self.execute("PAUSE_JOB").await?;
        self.set_paused(true);
        Ok(())
    }

    /// Resumes the paused job.
    pub async fn resume(&self) -> Result<(), DriverError> {
        self.execute("RESUME_JOB").await?;
        self.set_paused(false);
        Ok(())
    }

    /// Stops the running job.
    pub async fn stop_job(&self) -> Result<(), DriverError> {
        self.execute("STOP_JOB").await
    }

    /// Reads one controller memory address.
    pub async fn get_setting(
        &self,
        address: u16,
    ) -> Result<Vec<codec::Reply>, DriverError> {
        let mut guard = self.transport.lock().await;
        let transport = guard.as_mut().ok_or_else(|| {
            DriverError::Connection("controller is not connected".to_string())
        })?;
        transport.send(&codec::get_setting(address)).await?;
        let replies = transport.take_replies();
        drop(guard);
        self.apply_replies(replies.clone());
        Ok(replies)
    }

    /// Writes one controller memory address. Blocked while protect
    /// mode is on.
    pub async fn set_setting(
        &self,
        address: u16,
        a: u64,
        b: u64,
    ) -> Result<(), DriverError> {
        if self.protect_enabled() {
            return Err(DriverError::Unsupported(
                "SET_SETTING is blocked by protect mode".to_string(),
            ));
        }
        self.send_raw(&codec::set_setting(address, a, b)).await
    }

    fn set_paused(&self, paused: bool) {
        let mut status = self.status.lock().unwrap();
        status.paused = paused;
        drop(status);
        self.publish();
    }

    async fn connection_loop(self: Arc<Self>) {
        while self.keep_running.load(Ordering::SeqCst) {
            self.set_connection(TransportStatus::Connecting, None);
            if let Err(error) = self.clone().open_and_monitor().await {
                self.set_connection(
                    TransportStatus::Error,
                    Some(&error.to_string()),
                );
            }
            self.connected.store(false, Ordering::SeqCst);
            if !self.keep_running.load(Ordering::SeqCst) {
                break;
            }
            self.set_connection(TransportStatus::Sleeping, None);
            tokio::time::sleep(self.config.reconnect_delay).await;
        }
        self.connected.store(false, Ordering::SeqCst);
        self.set_connection(TransportStatus::Disconnected, None);
    }

    async fn open_and_monitor(self: Arc<Self>) -> Result<(), DriverError> {
        let link = open_link(
            self.config.transport(),
            self.config.host.as_deref(),
            self.config.usb_device.as_deref(),
            self.config.port,
        )
        .await?;
        let link: Box<dyn Link> =
            Box::new(TappedLink::new(link, self.events.clone()));
        let mut transport = RuidaTransport::new(
            link,
            self.config.transport(),
            self.config.magic,
        );
        transport.timeout = self.config.timeout;
        self.handshake(&mut transport).await?;
        *self.transport.lock().await = Some(transport);
        if let Err(error) = self.query_identity().await {
            log_crate::debug!("Ruida identity query failed: {error}");
        }
        self.connected.store(true, Ordering::SeqCst);
        {
            let mut state = self.state.lock().unwrap();
            state.status = DeviceStatus::Idle;
            state.error = None;
        }
        self.set_connection(TransportStatus::Connected, None);
        self.publish();
        self.query_loop().await
    }

    async fn handshake(
        &self,
        transport: &mut RuidaTransport,
    ) -> Result<(), DriverError> {
        for _ in 0..self.config.ping_retries.max(1) {
            if !self.keep_running.load(Ordering::SeqCst) {
                return Err(DriverError::Connection(
                    "connection closed".to_string(),
                ));
            }
            let ping = codec::get_setting(ADDRESS_CARD_ID);
            if transport.send_command(&ping).await.is_ok() {
                let replies = transport.take_replies();
                if !replies.is_empty() {
                    let mut status = self.status.lock().unwrap();
                    for reply in &replies {
                        status.apply(reply);
                    }
                    return Ok(());
                }
            }
            tokio::time::sleep(self.config.ping_delay).await;
        }
        Err(DriverError::Timeout(
            "controller did not answer the status ping".to_string(),
        ))
    }

    async fn query_identity(&self) -> Result<(), DriverError> {
        let mut raw = Vec::new();
        for address in [0x0026u16, 0x0036, 0x030F, 0x057F] {
            raw.extend(codec::get_setting(address));
        }
        self.send_raw(&raw).await
    }

    async fn query_loop(self: Arc<Self>) -> Result<(), DriverError> {
        loop {
            if !self.keep_running.load(Ordering::SeqCst) {
                return Ok(());
            }
            tokio::time::sleep(self.config.query_interval).await;
            let mut guard = self.transport.lock().await;
            let Some(transport) = guard.as_mut() else {
                return Err(DriverError::Connection(
                    "controller is not connected".to_string(),
                ));
            };
            let mut raw = codec::get_setting(ADDRESS_MACHINE_STATUS);
            for address in [0x0421u16, 0x0431, 0x0441, 0x0451] {
                raw.extend(codec::get_setting(address));
            }
            let result = transport.send(&raw).await;
            let replies = transport.take_replies();
            drop(guard);
            result?;
            self.apply_replies(replies);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct JogSpeeds {
    xy: f64,
    z: f64,
    u: f64,
}

impl JogSpeeds {
    fn update(&mut self, xy: f64, z: f64, u: f64) {
        self.xy = xy;
        self.z = z;
        self.u = u;
    }
}

fn codec_error(error: codec::CodecError) -> DriverError {
    DriverError::Device(error.to_string())
}

#[async_trait::async_trait]
impl Driver for RuidaDriver {
    fn info(&self) -> &'static DriverInfo {
        &INFO
    }

    fn state(&self) -> DeviceState {
        self.state.lock().unwrap().clone()
    }

    fn is_job_running(&self) -> bool {
        self.status.lock().unwrap().running
    }

    fn resource_uri(&self) -> Option<String> {
        if let Some(device) = &self.config.usb_device {
            return Some(format!("serial://{device}"));
        }
        let host = self.config.host.as_deref()?;
        match self.config.protocol {
            TransportKind::Udp => {
                Some(format!("udp://{host}:{}", self.config.port))
            }
            TransportKind::Tcp => {
                Some(format!("tcp://{host}:{}", self.config.port))
            }
            TransportKind::Usb => Some(format!("serial://{host}")),
        }
    }

    async fn connect(self: Arc<Self>) -> Result<(), DriverError> {
        let mut task = self.connection_task.lock().unwrap();
        if let Some(task) = task.take() {
            task.abort();
        }
        self.keep_running.store(true, Ordering::SeqCst);
        *task = Some(tokio::spawn(self.clone().connection_loop()));
        Ok(())
    }

    async fn cleanup(&self) {
        self.keep_running.store(false, Ordering::SeqCst);
        if let Some(task) = self.connection_task.lock().unwrap().take() {
            task.abort();
        }
        if let Some(mut transport) = self.transport.lock().await.take() {
            transport.close().await;
        }
        self.connected.store(false, Ordering::SeqCst);
        self.set_connection(TransportStatus::Disconnected, None);
    }

    async fn run(
        self: Arc<Self>,
        gcode: &str,
        _op_map: &HashMap<usize, i64>,
        _op_estimates: &[f64],
    ) -> Result<(), DriverError> {
        self.run_script(gcode).await
    }

    async fn run_raw(
        self: Arc<Self>,
        machine_code: &str,
    ) -> Result<(), DriverError> {
        self.run_script(machine_code).await
    }

    async fn cancel(&self, _mode: CancelMode) -> Result<(), DriverError> {
        self.stop_job().await
    }

    async fn execute_interactive_command(
        &self,
        command: &str,
    ) -> Result<Vec<String>, DriverError> {
        self.execute(command).await?;
        Ok(Vec::new())
    }

    async fn home(
        &self,
        axes: Option<Vec<String>>,
        _active_wcs: &str,
    ) -> Result<(), DriverError> {
        let Some(axes) = axes else {
            return self.home_xy().await;
        };
        for axis in axes {
            match axis.to_ascii_lowercase().as_str() {
                "x" => self.jog_x_to(0.0).await?,
                "y" => self.jog_y_to(0.0).await?,
                "z" => self.home_z().await?,
                "u" => self.home_u().await?,
                _ => {}
            }
        }
        Ok(())
    }

    async fn move_to(
        &self,
        speed: f64,
        x: f64,
        y: f64,
        _z: Option<f64>,
    ) -> Result<(), DriverError> {
        let speed_mm_s = if speed > 0.0 { speed / 60.0 } else { 100.0 };
        self.jog_speeds.lock().unwrap().xy = speed_mm_s;
        self.jog_xy_to(x, y).await
    }

    async fn jog(
        &self,
        speed: f64,
        deltas: &[(String, f64)],
    ) -> Result<(), DriverError> {
        let speed_mm_s = if speed > 0.0 { speed / 60.0 } else { 100.0 };
        self.jog_speeds.lock().unwrap().xy = speed_mm_s;
        let x = deltas
            .iter()
            .find(|(axis, _)| axis.eq_ignore_ascii_case("x"))
            .map(|(_, value)| *value);
        let y = deltas
            .iter()
            .find(|(axis, _)| axis.eq_ignore_ascii_case("y"))
            .map(|(_, value)| *value);
        self.jog_xy_rel(x, y).await?;
        for (axis, value) in deltas {
            match axis.to_ascii_lowercase().as_str() {
                "z" => self.jog_axis_rel('z', *value).await?,
                "u" => self.jog_axis_rel('u', *value).await?,
                _ => {}
            }
        }
        Ok(())
    }

    async fn set_hold(&self, hold: bool) -> Result<(), DriverError> {
        if hold {
            self.pause().await
        } else {
            self.resume().await
        }
    }

    async fn clear_alarm(&self) -> Result<(), DriverError> {
        self.stop_job().await
    }

    async fn select_tool(&self, _tool_number: i64) -> Result<(), DriverError> {
        Ok(())
    }

    async fn read_settings(
        &self,
    ) -> Result<Vec<(String, String)>, DriverError> {
        Ok(Vec::new())
    }

    async fn read_wcs_offsets(
        &self,
    ) -> Result<HashMap<String, (f64, f64, f64)>, DriverError> {
        Ok(HashMap::from([
            ("MACHINE".to_string(), (0.0, 0.0, 0.0)),
            ("ANCHOR".to_string(), (0.0, 0.0, 0.0)),
            ("CURRENT".to_string(), (0.0, 0.0, 0.0)),
            ("SET_POINT".to_string(), (0.0, 0.0, 0.0)),
        ]))
    }

    async fn read_parser_state(&self) -> Result<Option<String>, DriverError> {
        Ok(None)
    }

    async fn detect_unit_system(
        &self,
    ) -> Result<Option<UnitSystem>, DriverError> {
        Ok(Some(UnitSystem::Metric))
    }

    fn abort_background_tasks_now(&self) {
        self.keep_running.store(false, Ordering::SeqCst);
        if let Some(task) = self.connection_task.lock().unwrap().take() {
            task.abort();
        }
    }
}
