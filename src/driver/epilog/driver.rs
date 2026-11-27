//! The Epilog LPD session: probe reachability and upload print jobs.
//!
//! See the module docs for the wire exchange. The session opens one
//! connection per operation and never retries a partial job.

use crate::driver::core::driver::{
    info::caps, ConnectionKind, DriverInfo, DriverMaturity,
};

use crate::driver::core::driver::{SetupField, SetupKind};

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::task::JoinHandle;

use crate::driver::core::connector::{dial, DynStream};
use crate::driver::core::driver::{
    unsupported, CancelMode, Driver, DriverError, DriverEvents,
};
use crate::driver::core::types::{DeviceState, DeviceStatus, TransportStatus};

/// Default LPD port of a printer.
pub const DEFAULT_PORT: u16 = 515;

/// Default print queue name.
///
/// Empty by default: the reference driver submits the receive-job
/// command without a queue operand (`\x02\n`), which the Epilog print
/// servers accept. Set a name only for a print server that requires
/// one.
pub const DEFAULT_QUEUE: &str = "";

/// Name of the machine submitting the job (RFC 1179's `H` line).
///
/// The machine lists it with queued jobs. Raydriver carries no OS
/// hostname dependency (the crate also builds for wasm), so embedders
/// supply it; the default is generic.
pub const DEFAULT_SUBMIT_HOST: &str = "rayforge";

/// Default submitting user recorded in the control file.
pub const DEFAULT_USER: &str = "rayforge";

/// Default deadline for connecting, writing and reading one ack.
const DEFAULT_TIMEOUT: f64 = 30.0;

/// Write chunk size; the control transfer must not stall the machine.
const WRITE_CHUNK: usize = 64 * 1024;

/// Connection settings for an Epilog machine.
#[derive(Clone)]
pub struct EpilogConfig {
    pub host: String,
    pub port: u16,
    /// Print queue the job is submitted to (RFC 1179 receive-job
    /// operand).
    pub queue: String,
    /// Host name recorded in the control file's `H` line (RFC 1179).
    pub submit_host: String,
    /// User id recorded in the control file's `P` line (RFC 1179).
    pub user: String,
    /// Per-stage connect/write/ack deadline in seconds.
    pub timeout: f64,
}

impl EpilogConfig {
    pub fn new(host: &str) -> Self {
        Self {
            host: host.to_string(),
            port: DEFAULT_PORT,
            queue: DEFAULT_QUEUE.to_string(),
            submit_host: DEFAULT_SUBMIT_HOST.to_string(),
            user: DEFAULT_USER.to_string(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    fn deadline(&self) -> Duration {
        Duration::from_secs_f64(self.timeout.max(0.001))
    }

    /// The queue operand, reduced to something a daemon accepts.
    ///
    /// The name travels as a command operand terminated by LF, so
    /// only printable characters without whitespace survive. An empty
    /// configured queue stays empty: the receive-job command is then
    /// `\\x02\\n`, which is what the reference driver sends.
    fn queue_operand(&self) -> String {
        self.queue
            .chars()
            .filter(|c| c.is_ascii_graphic())
            .take(31)
            .collect()
    }
}

/// A submitted job the caller can still cancel while it uploads.
struct ActiveUpload {
    task: JoinHandle<Result<(), DriverError>>,
}

/// The Epilog LPD session core.
pub struct EpilogDriver {
    config: EpilogConfig,
    events: Arc<dyn DriverEvents>,
    state: Mutex<DeviceState>,
    connected: AtomicBool,
    uploading: AtomicBool,
    /// Monotonic id so a late upload result cannot clear a newer one.
    upload_seq: AtomicU64,
    upload: Mutex<Option<ActiveUpload>>,
}

/// Typed identity and capabilities of this driver.
pub static INFO: DriverInfo = DriverInfo {
    name: "EpilogDriver",
    connection: ConnectionKind::Network,
    maturity: DriverMaturity::Experimental,
    caps: caps(
        false, false, false, false, false, false, false, false, false, false,
        false,
    ),
    uses_gcode: false,
    machine_space_wcs: "MACHINE",
    supported_wcs: &[],
    config_keys: &["host", "port", "queue", "timeout"],
    setup: SETUP,
    fingerprint: None,
};

impl EpilogDriver {
    pub fn new(config: EpilogConfig, events: Arc<dyn DriverEvents>) -> Self {
        Self {
            config,
            events,
            state: Mutex::new(DeviceState::default()),
            connected: AtomicBool::new(false),
            uploading: AtomicBool::new(false),
            upload_seq: AtomicU64::new(0),
            upload: Mutex::new(None),
        }
    }

    fn set_status(&self, status: DeviceStatus) {
        let mut state = self.state.lock().unwrap();
        state.status = status;
        let snapshot = state.clone();
        drop(state);
        self.events.state_changed(&snapshot);
    }

    fn set_error(&self, message: Option<String>) {
        let mut state = self.state.lock().unwrap();
        state.error = message.as_ref().map(|message| {
            crate::driver::core::types::DeviceError::new(1, "EPILOG", message)
        });
        let snapshot = state.clone();
        drop(state);
        self.events.state_changed(&snapshot);
    }

    fn connection(&self, status: TransportStatus, message: Option<&str>) {
        self.events.connection_status(status, message);
    }

    /// Open a TCP connection, send nothing, close it again.
    ///
    /// The LPD port accepts a connection only when the printer is
    /// reachable, so this is the whole reachability check; the
    /// machine exposes no status query.
    pub async fn probe(&self) -> Result<(), DriverError> {
        let deadline = self.config.deadline();
        let mut stream = tokio::time::timeout(
            deadline,
            dial(&self.config.host, self.config.port),
        )
        .await
        .map_err(|_| {
            DriverError::Timeout(format!(
                "connecting to {}:{} timed out",
                self.config.host, self.config.port
            ))
        })?
        .map_err(|e| DriverError::Connection(e.to_string()))?;
        let _ = stream.shutdown().await;
        Ok(())
    }

    /// Upload a complete print job and wait until the machine has
    /// accepted it.
    ///
    /// *payload* is the encoder's byte stream; *title* names the job
    /// in the machine's job list.  The transfer runs in a tracked
    /// task so `cancel` (or the asyncio future's cancellation) can
    /// abort it mid-flight.
    pub async fn upload(
        self: Arc<Self>,
        payload: Vec<u8>,
        title: String,
    ) -> Result<(), DriverError> {
        if payload.is_empty() {
            return Err(DriverError::Device(
                "the Epilog job is empty".to_string(),
            ));
        }
        if self.uploading.swap(true, Ordering::SeqCst) {
            return Err(DriverError::Device(
                "an Epilog upload is already in progress".to_string(),
            ));
        }
        let seq = self.upload_seq.fetch_add(1, Ordering::SeqCst) + 1;
        self.set_status(DeviceStatus::Queue);
        self.events
            .command_status(TransportStatus::Connecting, None);

        let session = self.clone();
        let task =
            tokio::spawn(async move {
                let result = session.transfer(&payload, &title).await.map_err(
                    |e| match e {
                        TransferError::Connection(message) => {
                            DriverError::Connection(message)
                        }
                        TransferError::Timeout(message) => {
                            DriverError::Timeout(message)
                        }
                        TransferError::Rejected(message) => {
                            DriverError::Device(message)
                        }
                    },
                );
                session.finish_upload(seq, &result);
                result
            });
        self.track_upload(task, seq);

        let Some(task) = self.upload_task() else {
            return Err(DriverError::Device(
                "Epilog upload was cancelled".to_string(),
            ));
        };
        match task.await {
            Ok(result) => result,
            Err(join) if join.is_cancelled() => Err(DriverError::Device(
                "Epilog upload was cancelled".to_string(),
            )),
            Err(join) => Err(DriverError::Other(join.to_string())),
        }
    }

    /// Publishes the terminal state of an upload unless it has been
    /// superseded by a newer one.
    fn finish_upload(&self, seq: u64, result: &Result<(), DriverError>) {
        if seq != self.upload_seq.load(Ordering::SeqCst) {
            return;
        }
        self.uploading.store(false, Ordering::SeqCst);
        self.upload.lock().unwrap().take();
        match result {
            Ok(()) => {
                self.set_error(None);
                self.set_status(DeviceStatus::Idle);
                self.events.command_status(TransportStatus::Idle, None);
                self.events.job_finished();
            }
            Err(error) => {
                let message = error.to_string();
                self.set_error(Some(message.clone()));
                self.set_status(DeviceStatus::Unknown);
                self.events
                    .command_status(TransportStatus::Error, Some(&message));
                self.events.job_error(&message);
            }
        }
    }

    /// Perform the LPD exchange on one fresh connection.
    async fn transfer(
        &self,
        payload: &[u8],
        title: &str,
    ) -> Result<(), TransferError> {
        let deadline = self.config.deadline();
        let mut stream = tokio::time::timeout(
            deadline,
            dial(&self.config.host, self.config.port),
        )
        .await
        .map_err(|_| {
            TransferError::Timeout("connecting to the Epilog timed out".into())
        })?
        .map_err(|e| TransferError::Connection(e.to_string()))?;

        let exchange =
            self.exchange(&mut stream, payload, title, deadline).await;
        let _ = stream.shutdown().await;
        exchange
    }

    async fn exchange(
        &self,
        stream: &mut DynStream,
        payload: &[u8],
        title: &str,
        deadline: Duration,
    ) -> Result<(), TransferError> {
        let token = upload_token();
        let name = format!("dfA{token}rayforge");
        let control_name = format!("cfA{token}rayforge");
        let control = control_file(
            &self.config.submit_host,
            &self.config.user,
            title,
            &name,
        );

        let receive = format!("\x02{}\n", self.config.queue_operand());
        self.stage(stream, receive.as_bytes(), "receive a job", deadline)
            .await?;

        // From here on the daemon owns a partially created job, so a
        // failure aborts it instead of leaving a half job behind
        // (RFC 1179 §6.1).
        let result = self
            .transfer_files(
                stream,
                &control,
                &control_name,
                &name,
                payload,
                deadline,
            )
            .await;
        if result.is_err() {
            self.send_abort(stream, deadline).await;
        }
        result
    }

    async fn transfer_files(
        &self,
        stream: &mut DynStream,
        control: &[u8],
        control_name: &str,
        name: &str,
        payload: &[u8],
        deadline: Duration,
    ) -> Result<(), TransferError> {
        let header = format!("\x02{} {control_name}\n", control.len());
        self.stage(stream, header.as_bytes(), "the control header", deadline)
            .await?;

        let mut control_block = control.to_vec();
        control_block.push(0);
        self.stage(stream, &control_block, "the control file", deadline)
            .await?;

        let data_header = format!("\x03{} {name}\n", payload.len());
        self.stage(stream, data_header.as_bytes(), "the data header", deadline)
            .await?;

        self.write_all(stream, payload, deadline).await?;
        self.read_ack(stream, "the job data", deadline).await?;
        Ok(())
    }

    /// Tell the daemon to discard the partially received job.
    ///
    /// Best effort: the connection is about to be closed either way,
    /// and the RFC replies to the abort with an ack nobody needs.
    async fn send_abort(&self, stream: &mut DynStream, deadline: Duration) {
        let _ =
            tokio::time::timeout(deadline, stream.write_all(b"\x01\n")).await;
        let _ = tokio::time::timeout(deadline, stream.flush()).await;
        let mut ack = [0u8; 1];
        let _ =
            tokio::time::timeout(deadline, stream.read_exact(&mut ack)).await;
    }

    async fn stage(
        &self,
        stream: &mut DynStream,
        bytes: &[u8],
        label: &str,
        deadline: Duration,
    ) -> Result<(), TransferError> {
        self.write_all(stream, bytes, deadline).await?;
        self.read_ack(stream, label, deadline).await
    }

    async fn write_all(
        &self,
        stream: &mut DynStream,
        bytes: &[u8],
        deadline: Duration,
    ) -> Result<(), TransferError> {
        for chunk in bytes.chunks(WRITE_CHUNK) {
            tokio::time::timeout(deadline, stream.write_all(chunk))
                .await
                .map_err(|_| {
                    TransferError::Timeout(
                        "writing the Epilog job timed out".into(),
                    )
                })?
                .map_err(|e| TransferError::Connection(e.to_string()))?;
        }
        tokio::time::timeout(deadline, stream.flush())
            .await
            .map_err(|_| {
                TransferError::Timeout(
                    "flushing the Epilog job timed out".into(),
                )
            })?
            .map_err(|e| TransferError::Connection(e.to_string()))?;
        Ok(())
    }

    async fn read_ack(
        &self,
        stream: &mut DynStream,
        label: &str,
        deadline: Duration,
    ) -> Result<(), TransferError> {
        let mut ack = [0u8; 1];
        tokio::time::timeout(deadline, stream.read_exact(&mut ack))
            .await
            .map_err(|_| {
                TransferError::Timeout(format!(
                    "Epilog timed out acknowledging {label}"
                ))
            })?
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    TransferError::Connection(format!(
                        "Epilog disconnected during {label}"
                    ))
                } else {
                    TransferError::Connection(e.to_string())
                }
            })?;
        match ack[0] {
            0 => Ok(()),
            code => Err(TransferError::Rejected(format!(
                "Epilog rejected {label} (code {code})"
            ))),
        }
    }

    /// Stop the in-flight upload, if any.
    async fn cancel_upload(&self) -> bool {
        let active = self.upload.lock().unwrap().take();
        let Some(active) = active else {
            return false;
        };
        self.upload_seq.fetch_add(1, Ordering::SeqCst);
        active.task.abort();
        let _ = active.task.await;
        self.uploading.store(false, Ordering::SeqCst);
        self.events.job_error(
            "Epilog upload interrupted; inspect the job list at the machine",
        );
        true
    }
}

/// Failure categories of the LPD exchange.
enum TransferError {
    Connection(String),
    Timeout(String),
    Rejected(String),
}

impl std::fmt::Display for TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(m) | Self::Timeout(m) | Self::Rejected(m) => {
                write!(f, "{m}")
            }
        }
    }
}

/// The control file describing one job.
///
/// Following RFC 1179: `H` names the submitting host, `P` the user,
/// `J` the job name the machine lists, `N` the source file name, and
/// `l`/`U` print and then remove the data file.
fn control_file(
    host: &str,
    user: &str,
    title: &str,
    data_name: &str,
) -> Vec<u8> {
    let job = sanitize_title(title);
    format!(
        "H{}\nP{}\nJ{job}\nN{job}\nl{data_name}\nU{data_name}\n",
        operand(host, 31),
        operand(user, 31),
    )
    .into_bytes()
}

/// Reduce a control-file operand to RFC 1179's printable characters
/// and length bound.
fn operand(value: &str, max_len: usize) -> String {
    let cleaned: String = value
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(max_len)
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Where the last job number handed out reached.
static LAST_JOB_NUMBER: AtomicU64 = AtomicU64::new(0);

/// A three-digit suffix that keeps queued jobs distinguishable.
///
/// Cycled through a process-wide counter rather than derived from the
/// clock: two uploads in the same millisecond must not collide.
/// The reserve/acknowledge protocol numbers jobs 0..=999, so only the
/// low three digits travel.
fn upload_token() -> String {
    let issued = LAST_JOB_NUMBER.fetch_add(1, Ordering::SeqCst);
    format!("{:03}", issued % 1000)
}

/// Keep the job name printable and free of LPD control characters.
fn sanitize_title(title: &str) -> String {
    let mut out = String::with_capacity(title.len().min(64));
    for ch in title.chars() {
        if out.len() >= 64 {
            break;
        }
        if ch.is_ascii_alphanumeric() || matches!(ch, ' ' | '.' | '_' | '-') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim();
    if trimmed.is_empty() {
        "Rayforge".to_string()
    } else {
        trimmed.to_string()
    }
}

#[async_trait]
impl Driver for EpilogDriver {
    fn info(&self) -> &'static crate::driver::core::driver::DriverInfo {
        &INFO
    }

    async fn connect(self: Arc<Self>) -> Result<(), DriverError> {
        self.connection(TransportStatus::Connecting, None);
        match self.probe().await {
            Ok(()) => {
                self.connected.store(true, Ordering::SeqCst);
                self.set_status(DeviceStatus::Idle);
                self.connection(
                    TransportStatus::Connected,
                    Some("Epilog reachable; machine status is unavailable"),
                );
                Ok(())
            }
            Err(error) => {
                self.connected.store(false, Ordering::SeqCst);
                self.set_status(DeviceStatus::Unknown);
                self.connection(
                    TransportStatus::Error,
                    Some(&error.to_string()),
                );
                Err(error)
            }
        }
    }

    async fn cleanup(&self) {
        self.connected.store(false, Ordering::SeqCst);
        self.cancel_upload().await;
        self.connection(TransportStatus::Disconnected, None);
    }

    fn state(&self) -> DeviceState {
        self.state.lock().unwrap().clone()
    }

    fn is_job_running(&self) -> bool {
        self.uploading.load(Ordering::SeqCst)
    }

    fn resource_uri(&self) -> Option<String> {
        Some(format!("tcp://{}:{}", self.config.host, self.config.port))
    }

    fn abort_background_tasks_now(&self) {
        if let Some(active) = self.upload.lock().unwrap().take() {
            active.task.abort();
        }
        self.uploading.store(false, Ordering::SeqCst);
    }

    async fn cancel(&self, _mode: CancelMode) -> Result<(), DriverError> {
        self.cancel_upload().await;
        Ok(())
    }

    async fn run(
        self: Arc<Self>,
        _gcode: &str,
        _op_map: &std::collections::HashMap<usize, i64>,
        _op_estimates: &[f64],
    ) -> Result<(), DriverError> {
        Err(unsupported("run"))
    }

    async fn read_wcs_offsets(
        &self,
    ) -> Result<std::collections::HashMap<String, (f64, f64, f64)>, DriverError>
    {
        Ok([("MACHINE".to_string(), (0.0, 0.0, 0.0))]
            .into_iter()
            .collect())
    }

    async fn read_parser_state(&self) -> Result<Option<String>, DriverError> {
        Ok(Some("MACHINE".to_string()))
    }

    async fn select_tool(&self, _tool_number: i64) -> Result<(), DriverError> {
        Err(unsupported("select_tool"))
    }
}

impl EpilogDriver {
    /// Store the upload task so `cancel` can abort it.
    fn track_upload(
        &self,
        task: JoinHandle<Result<(), DriverError>>,
        _seq: u64,
    ) {
        *self.upload.lock().unwrap() = Some(ActiveUpload { task });
    }

    /// Take the tracked upload task out so the caller can await it.
    fn upload_task(&self) -> Option<JoinHandle<Result<(), DriverError>>> {
        self.upload.lock().unwrap().take().map(|active| active.task)
    }
}

/// Connection-setup form for this driver.
pub static SETUP: &[SetupField] = &[
    SetupField::new("host", SetupKind::Hostname).required(),
    SetupField::new("port", SetupKind::Port)
        .with_default("515")
        .with_range(1.0, 65535.0),
    SetupField::new("queue", SetupKind::Text).with_default("Laser"),
    SetupField::new("timeout", SetupKind::Float).with_default("30"),
];
