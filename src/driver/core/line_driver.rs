//! Shared skeleton for the strict ping-pong line sessions (the
//! minimal GRBL session, Smoothie and Marlin): send one line, wait
//! for its `ok`/`error:` acknowledgement, then send the next.
//!
//! This module owns the mechanics such sessions used to duplicate —
//! the pending-acknowledgement bookkeeping, RX line reassembly, the
//! connection loop skeleton, job streaming, cancel and the safety
//! shutdown — so the quirks (late-ack suppression, wire-ordered FIFO
//! for Marlin, event-outside-the-lock emission) exist exactly once.
//!
//! A session is a struct that embeds a [`LineDriverBase`] and
//! implements [`PingPongDriver`]: the trait's *hooks* carry
//! everything protocol-specific (line dispatch, handshake style,
//! poll loop, cancel bytes), while its *operations* — command
//! exchange, job streaming, the dialect-formatted device operations
//! and the lifecycle — are provided once, here.  Implementing the
//! trait is the supported way to add a new strict ping-pong protocol;
//! callers use the session's own API (the trait is in scope in this
//! crate's bindings) or the protocol-agnostic
//! [`crate::driver::core::driver::Driver`] surface.

use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::driver::core::dialect::{format_template, Arg, Dialect};
use crate::driver::core::driver::{AsyncEvent, DriverError, DriverEvents};
use crate::driver::core::transport::Transport;
use crate::driver::core::types::{
    DeviceError, DeviceState, DeviceStatus, TransportStatus,
};

/// One sent command awaiting its acknowledgement.
struct PendingCmd {
    lines: Mutex<Vec<String>>,
    finished: AsyncEvent,
}

impl PendingCmd {
    fn new() -> Self {
        Self {
            lines: Mutex::new(Vec::new()),
            finished: AsyncEvent::new(),
        }
    }

    fn add_line(&self, line: &str) {
        self.lines.lock().unwrap().push(line.to_string());
    }

    fn finish(&self) {
        self.finished.set();
    }

    fn lines(&self) -> Vec<String> {
        self.lines.lock().unwrap().clone()
    }

    fn is_finished(&self) -> bool {
        self.finished.is_set()
    }
}

/// Where outstanding pending acknowledgements live.
///
/// The slot model allows a single in-flight command (GRBL simple,
/// Smoothie); the FIFO queue matches the device's wire-ordered
/// responses when a status poll may be in flight while a user
/// command waits (Marlin).
enum PendingInner {
    Slot(Mutex<Option<Arc<PendingCmd>>>),
    Queue(Mutex<VecDeque<Arc<PendingCmd>>>),
}

/// The pending-acknowledgement bookkeeping of a line session.
/// Construct with [`PendingStore::slot`] or [`PendingStore::queue`].
pub struct PendingStore {
    inner: PendingInner,
}

impl PendingStore {
    /// One command in flight at a time.
    pub fn slot() -> Self {
        Self {
            inner: PendingInner::Slot(Mutex::new(None)),
        }
    }

    /// Wire-ordered FIFO of in-flight commands.
    pub fn queue() -> Self {
        Self {
            inner: PendingInner::Queue(Mutex::new(VecDeque::new())),
        }
    }

    /// Register a newly sent command (used by [`LineDriverBase::await_ack`]).
    fn enqueue(&self, pending: Arc<PendingCmd>) {
        match &self.inner {
            PendingInner::Slot(slot) => {
                *slot.lock().unwrap() = Some(pending);
            }
            PendingInner::Queue(queue) => {
                queue.lock().unwrap().push_back(pending);
            }
        }
    }

    /// Feed an unsolicited response line to the oldest outstanding
    /// command (unless it already finished).
    pub fn collect(&self, line: &str) {
        let pending = match &self.inner {
            PendingInner::Slot(slot) => slot.lock().unwrap().clone(),
            PendingInner::Queue(queue) => {
                queue.lock().unwrap().front().cloned()
            }
        };
        if let Some(pending) = pending {
            if !pending.is_finished() {
                pending.add_line(line);
            }
        }
    }

    /// Finish the oldest outstanding command, optionally collecting
    /// one final response line first (the `error:`/`ALARM:` text).
    /// The entry stays owned by the waiting ping-pong.
    pub fn finish_next(&self, last_line: Option<&str>) {
        let pending = match &self.inner {
            PendingInner::Slot(slot) => slot.lock().unwrap().clone(),
            PendingInner::Queue(queue) => {
                queue.lock().unwrap().front().cloned()
            }
        };
        if let Some(pending) = pending {
            if !pending.is_finished() {
                if let Some(line) = last_line {
                    pending.add_line(line);
                }
                pending.finish();
            }
        }
    }

    /// Drop one entry on every path: a late acknowledgement for a
    /// timed out command must not resolve a later one.
    fn remove(&self, pending: &Arc<PendingCmd>) {
        match &self.inner {
            PendingInner::Slot(slot) => {
                if let Some(pending) = slot.lock().unwrap().take() {
                    pending.finish();
                }
            }
            PendingInner::Queue(queue) => {
                queue.lock().unwrap().retain(|p| !Arc::ptr_eq(p, pending));
            }
        }
    }

    /// Finish and drop every outstanding entry (cancel, cleanup).
    pub fn resolve(&self) {
        match &self.inner {
            PendingInner::Slot(slot) => {
                if let Some(pending) = slot.lock().unwrap().take() {
                    pending.finish();
                }
            }
            PendingInner::Queue(queue) => {
                for pending in queue.lock().unwrap().drain(..) {
                    pending.finish();
                }
            }
        }
    }
}

/// The state bundle shared by every ping-pong session.  A session
/// struct embeds one and hands a reference to it back from
/// [`PingPongDriver::base`]; the bundle's methods carry everything
/// the hooks and provided operations have in common.
pub struct LineDriverBase {
    /// The command templates (owned data; see [`Dialect`]).
    pub dialect: RwLock<Dialect>,
    /// The event sink (implemented by the Python bindings or by the
    /// embedding application).
    pub events: Arc<dyn DriverEvents>,
    /// The byte transport to the device.
    pub transport: Arc<dyn Transport>,
    /// The last reported device state.
    pub state: Mutex<DeviceState>,
    /// Drives the connection loop; true while the session wants to
    /// stay (re)connected.
    pub keep_running: AtomicBool,
    /// True while a job streams.
    pub job_running: AtomicBool,
    /// Set by [`cancel`]; observed by the job loop.
    pub is_cancelled: AtomicBool,
    /// Set by hold requests; observed by the status handlers.
    pub is_holding: AtomicBool,
    /// Set by the handshake hooks when the device announces itself.
    pub handshake_received: AsyncEvent,
    /// Partial bytes between complete `\n`-terminated lines.
    pub line_buffer: Mutex<Vec<u8>>,
    /// The pending-acknowledgement bookkeeping.
    pub pending: PendingStore,
    /// The error that aborted the current job, if any.
    pub job_exception: Mutex<Option<DriverError>>,
    /// Serializes user commands for slot-style sessions.
    pub cmd_lock: tokio::sync::Mutex<()>,
    /// The last op index reported via `command_done`.
    last_reported_op_index: Mutex<i64>,
    connection_task: Mutex<Option<JoinHandle<()>>>,
    background_tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl LineDriverBase {
    /// Build the bundle for a transport already chosen by the
    /// session's constructor.
    pub fn new(
        dialect: Dialect,
        events: Arc<dyn DriverEvents>,
        transport: Arc<dyn Transport>,
        pending: PendingStore,
    ) -> Self {
        Self {
            dialect: RwLock::new(dialect),
            events,
            transport,
            state: Mutex::new(DeviceState::default()),
            keep_running: AtomicBool::new(false),
            job_running: AtomicBool::new(false),
            is_cancelled: AtomicBool::new(false),
            is_holding: AtomicBool::new(false),
            handshake_received: AsyncEvent::new(),
            line_buffer: Mutex::new(Vec::new()),
            pending,
            job_exception: Mutex::new(None),
            last_reported_op_index: Mutex::new(-1),
            cmd_lock: tokio::sync::Mutex::new(()),
            connection_task: Mutex::new(None),
            background_tasks: Mutex::new(Vec::new()),
        }
    }

    /// The last reported device state.
    pub fn state(&self) -> DeviceState {
        self.state.lock().unwrap().clone()
    }

    /// True while a job streams.
    pub fn is_job_running(&self) -> bool {
        self.job_running.load(Ordering::SeqCst)
    }

    /// True after [`cancel`] was called (observed by the job loop).
    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::SeqCst)
    }

    /// The transport's resource identifier (e.g. `serial:///dev/ttyUSB0`).
    pub fn resource_uri(&self) -> Option<String> {
        self.transport.resource_uri()
    }

    /// The current command templates.
    pub fn dialect(&self) -> Dialect {
        self.dialect.read().unwrap().clone()
    }

    /// Replace the command templates.
    pub fn update_dialect(&self, dialect: Dialect) {
        *self.dialect.write().unwrap() = dialect;
    }

    /// Clear any reported device error and emit the state (used
    /// after successful homing or alarm clearing).
    pub fn clear_device_error(&self) {
        let state = {
            let mut state = self.state.lock().unwrap();
            state.error = None;
            state.clone()
        };
        self.events.state_changed(&state);
    }

    /// Store *state* and emit it, unless it is identical to the
    /// currently reported state.  The callback runs outside the lock
    /// (the callback attaches to Python).
    pub fn emit_state_if_changed(&self, state: DeviceState) {
        let snapshot = {
            let mut current = self.state.lock().unwrap();
            if *current == state {
                None
            } else {
                *current = state.clone();
                Some(state)
            }
        };
        if let Some(snapshot) = snapshot {
            self.events.state_changed(&snapshot);
        }
    }

    /// Record a device error if none is reported yet, emitting the
    /// state when it changed.
    pub fn set_device_error(&self, error: DeviceError) {
        let snapshot = {
            let mut state = self.state.lock().unwrap();
            if state.error.is_none() {
                state.error = Some(error);
                Some(state.clone())
            } else {
                None
            }
        };
        if let Some(snapshot) = snapshot {
            self.events.state_changed(&snapshot);
        }
    }

    /// Emit a connection-status event.
    pub fn connection_status(
        &self,
        status: TransportStatus,
        message: Option<&str>,
    ) {
        // A lost or failed connection means the last reported status
        // is stale: the protocol drivers reset it to Unknown, as the
        // original shells did.
        if matches!(
            status,
            TransportStatus::Error | TransportStatus::Disconnected
        ) {
            let snapshot = {
                let mut state = self.state.lock().unwrap();
                if state.status != DeviceStatus::Unknown {
                    state.status = DeviceStatus::Unknown;
                    Some(state.clone())
                } else {
                    None
                }
            };
            if let Some(snapshot) = snapshot {
                self.events.state_changed(&snapshot);
            }
        }
        self.events.connection_status(status, message);
    }

    /// The shared tail of every ping-pong: enqueue the pending
    /// acknowledgement, write the command, await its acknowledgement
    /// under *timeout*, then drop the entry.  A session's
    /// [`PingPongDriver::ping_pong`] wraps this with its protocol
    /// specifics (serialization, handshake gates).
    pub async fn await_ack(
        &self,
        command: &str,
        timeout: Duration,
    ) -> Result<Vec<String>, DriverError> {
        let pending = Arc::new(PendingCmd::new());
        self.pending.enqueue(pending.clone());
        self.events.raw_tx(command);
        let result = async {
            self.transport
                .write(format!("{command}\n").as_bytes())
                .await?;
            match tokio::time::timeout(timeout, pending.finished.wait()).await {
                Ok(()) => Ok(()),
                Err(_) => {
                    Err(DriverError::Timeout("operation timed out".to_string()))
                }
            }
        }
        .await;
        self.pending.remove(&pending);
        match result {
            Ok(()) => Ok(pending.lines()),
            Err(err) => Err(err),
        }
    }

    /// Stop the session: shut the keep-running flag off, resolve the
    /// pending acknowledgements, abort the background tasks, close
    /// the transport and report `Disconnected`.
    pub async fn cleanup(&self) {
        self.keep_running.store(false, Ordering::SeqCst);
        self.job_running.store(false, Ordering::SeqCst);
        self.pending.resolve();
        self.abort_background_tasks().await;
        self.transport.close().await;
        self.connection_status(TransportStatus::Disconnected, None);
    }

    /// Abort every background task without awaiting, for use from
    /// `Drop` where blocking is not allowed.  A session dropped
    /// without disconnecting must not leave its connection, polling
    /// and reader tasks running on the shared runtime.
    pub fn abort_background_tasks_now(&self) {
        self.keep_running.store(false, Ordering::SeqCst);
        if let Some(task) = self.connection_task.lock().unwrap().take() {
            task.abort();
        }
        for task in self.background_tasks.lock().unwrap().drain(..) {
            task.abort();
        }
    }

    async fn abort_background_tasks(&self) {
        let connection = self.connection_task.lock().unwrap().take();
        if let Some(task) = connection {
            task.abort();
            let _ = task.await;
        }
        let tasks: Vec<_> =
            self.background_tasks.lock().unwrap().drain(..).collect();
        for task in tasks {
            task.abort();
            let _ = task.await;
        }
    }

    async fn abort_readers(&self) {
        let tasks: Vec<_> =
            self.background_tasks.lock().unwrap().drain(..).collect();
        for task in tasks {
            task.abort();
            let _ = task.await;
        }
    }

    /// Shared job prologue: reset cancellation and exception state,
    /// mark the job running and report `Run`.
    fn start_job(&self) {
        self.is_cancelled.store(false, Ordering::SeqCst);
        self.job_running.store(true, Ordering::SeqCst);
        self.is_holding.store(false, Ordering::SeqCst);
        *self.last_reported_op_index.lock().unwrap() = -1;
        *self.job_exception.lock().unwrap() = None;
        let state = {
            let mut state = self.state.lock().unwrap();
            state.status = DeviceStatus::Run;
            state.clone()
        };
        self.events.state_changed(&state);
    }

    /// Fire [`DriverEvents::command_done`] when the op index
    /// changes, before the op's first line is sent.
    fn fire_op_progress(&self, op_map: &HashMap<usize, i64>, line_idx: usize) {
        if let Some(op_index) = op_map.get(&line_idx).copied() {
            let mut last = self.last_reported_op_index.lock().unwrap();
            if op_index != *last {
                *last = op_index;
                drop(last);
                log_crate::debug!("Firing command_done for op {op_index}");
                self.events.command_done(op_index);
            }
        }
    }
}

/// Open the connection for *session*: clean up any previous run,
/// spawn the connection loop until
/// [`cleanup`](LineDriverBase::cleanup).
pub async fn connect<S: PingPongDriver>(session: &Arc<S>) {
    session.base().cleanup().await;
    session.base().keep_running.store(true, Ordering::SeqCst);
    let core = session.clone();
    let task = tokio::spawn(async move { connection_loop(core).await });
    *session.base().connection_task.lock().unwrap() = Some(task);
}

/// The connection loop skeleton: open the transport, spawn the RX
/// reader, run the protocol handshake, run the poll loop while the
/// connection holds, then tear down and sleep until reconnect.
async fn connection_loop<S: PingPongDriver>(session: Arc<S>) {
    let base = session.base();
    while base.keep_running.load(Ordering::SeqCst) {
        base.handshake_received.clear();
        let (tx, mut rx) = mpsc::unbounded_channel();
        if let Err(err) = base.transport.open(tx).await {
            log_crate::error!("Connection error: {err}");
            base.connection_status(
                TransportStatus::Error,
                Some(&err.to_string()),
            );
        } else {
            let reader_session = session.clone();
            let reader = tokio::spawn(async move {
                while let Some(data) = rx.recv().await {
                    reader_session.handle_rx(&data);
                }
            });
            base.background_tasks.lock().unwrap().push(reader);

            session.on_opened().await;
            if session.handshake().await {
                session.on_established().await;
                session.poll_loop().await;
            } else {
                session.on_handshake_failed();
            }

            base.abort_readers().await;
            base.transport.close().await;
            base.line_buffer.lock().unwrap().clear();
        }

        if !base.keep_running.load(Ordering::SeqCst) {
            break;
        }
        base.connection_status(TransportStatus::Sleeping, None);
        tokio::time::sleep(session.reconnect_delay()).await;
    }
}

/// The protocol-specific cancel payload: the job-abort bytes (sent
/// raw, without a `raw_tx` event, exactly like the sessions always
/// did) and the log line describing them.
pub struct StopCommand {
    /// The bytes to write.
    pub bytes: &'static [u8],
    /// The `log_crate::info` message describing the stop.
    pub log: &'static str,
}

/// The ping-pong session hooks.
///
/// Implement this trait to plug a strict ping-pong line protocol
/// into the shared skeleton: the struct embeds a [`LineDriverBase`],
/// hands it back from [`PingPongDriver::base`], and fills in the
/// protocol hooks.  The skeleton's operations — job streaming, the
/// command exchange, cancel, the dialect-formatted device operations
/// and the connection loop — are the free functions of this module;
/// a session's [`Driver`](crate::driver::core::driver::Driver) impl calls
/// them with its own signatures.
#[async_trait]
pub trait PingPongDriver: Send + Sync + 'static {
    /// The session's shared state bundle.
    fn base(&self) -> &LineDriverBase;

    /// Dispatch one complete RX line (status reports, acks, alarms,
    /// protocol-specific lines).
    fn handle_line(&self, line: &str);

    /// Wait for the protocol handshake after transport open.
    /// `false` marks a phantom port.
    async fn handshake(&self) -> bool;

    /// Called right after the transport opened, before the
    /// handshake.  Only sessions that report "Connected" before the
    /// handshake completes need this.
    async fn on_opened(&self) {}

    /// Called after a successful handshake, before the poll loop.
    /// The default reports "Connected".
    async fn on_established(&self) {
        self.base()
            .connection_status(TransportStatus::Connected, None);
    }

    /// Log and report a failed handshake (phantom port).
    fn on_handshake_failed(&self);

    /// The post-handshake poll loop (status reports or position
    /// polling); runs until the connection drops.
    async fn poll_loop(&self);

    /// Send one command and wait for its `ok`/`error:`
    /// acknowledgement.  Returns the collected response lines; an
    /// `error:` response is the last line.
    async fn ping_pong(
        &self,
        command: &str,
    ) -> Result<Vec<String>, DriverError>;

    /// Send one command and wait up to *timeout* for its
    /// acknowledgement.  Sessions that can honour a per-command
    /// timeout override this; the default ignores it.
    async fn ping_pong_with_timeout(
        &self,
        command: &str,
        _timeout: Duration,
    ) -> Result<Vec<String>, DriverError> {
        self.ping_pong(command).await
    }

    /// Acknowledgement budget for one streamed line, derived from the
    /// line's estimated execution time when available.
    fn line_timeout(&self, estimate: Option<f64>) -> Duration;

    /// Firmware label used in the job streaming log lines.
    fn job_log_label(&self) -> &'static str;

    /// Whether an `Alarm` device status stops the job stream.
    fn stop_job_on_alarm(&self) -> bool {
        true
    }

    /// Prepare one raw G-code line for sending.
    fn prep_job_line<'a>(&'a self, raw: &'a str) -> Cow<'a, str> {
        Cow::Borrowed(raw.trim())
    }

    /// Delay before the safety shutdown commands are sent; `None`
    /// sends them immediately.
    fn safety_shutdown_delay(&self) -> Option<Duration>;

    /// How long to sleep between reconnect attempts.
    fn reconnect_delay(&self) -> Duration;

    /// The job-abort bytes and their log line, written raw by
    /// [`cancel`] (no `raw_tx` event, exactly like the sessions
    /// always did).
    fn stop_command(&self) -> StopCommand;

    /// The connection-error message [`cancel`] returns when it finds
    /// the transport down.
    fn not_connected_message(&self) -> &'static str;

    /// Reassemble received bytes into lines and dispatch each one.
    fn handle_rx(&self, data: &[u8]) {
        let base = self.base();
        let mut new_lines = Vec::new();
        {
            let mut buffer = base.line_buffer.lock().unwrap();
            buffer.extend_from_slice(data);
            while let Some(nl) = buffer.iter().position(|b| *b == b'\n') {
                let line_bytes: Vec<u8> = buffer.drain(..=nl).collect();
                if let Ok(text) = String::from_utf8(line_bytes) {
                    for line in text.trim().lines() {
                        if !line.is_empty() {
                            new_lines.push(line.to_string());
                        }
                    }
                }
            }
        }
        for line in new_lines {
            self.handle_line(&line);
        }
    }
}

/// The shared job streaming loop, strict ping-pong: prologue
/// (`start_job` semantics), per-line preparation, op progress,
/// alarm/cancel stop rules, exception handling and the tail
/// (safety shutdown, `job_finished`).  Errors are reported through
/// the event sink, mirroring the sessions' historical behavior.
pub async fn stream_job<S: PingPongDriver>(
    session: &S,
    gcode: &str,
    op_map: &HashMap<usize, i64>,
    op_estimates: &[f64],
) {
    let base = session.base();
    base.start_job();
    let lines: Vec<String> = gcode.lines().map(|l| l.to_string()).collect();
    let total = lines.len();
    let mut sent = 0usize;
    let mut interrupted = false;
    log_crate::debug!(
        "Starting {} job with {total} lines.",
        session.job_log_label()
    );

    for (line_idx, raw_line) in lines.iter().enumerate() {
        if base.is_cancelled()
            || (session.stop_job_on_alarm()
                && base.state.lock().unwrap().status == DeviceStatus::Alarm)
        {
            if session.stop_job_on_alarm() {
                log_crate::info!("Job cancelled or alarm. Stopping.");
            } else {
                log_crate::info!("Job cancelled. Stopping.");
            }
            break;
        }

        let line = session.prep_job_line(raw_line);
        if line.is_empty() {
            continue;
        }

        base.fire_op_progress(op_map, line_idx);

        let estimate = op_map
            .get(&line_idx)
            .and_then(|op| op_estimates.get(*op as usize).copied());
        let timeout = session.line_timeout(estimate);
        match session.ping_pong_with_timeout(&line, timeout).await {
            Ok(_) => {
                if base.job_exception.lock().unwrap().is_some() {
                    break;
                }
                sent += 1;
            }
            Err(err) => {
                log_crate::warn!("Job interrupted: {err:?}");
                interrupted = true;
                break;
            }
        }
        tokio::task::yield_now().await;
    }

    let had_exception = base.job_exception.lock().unwrap().take().is_some();
    let was_cancelled = base.is_cancelled();
    base.job_running.store(false, Ordering::SeqCst);
    base.is_holding.store(false, Ordering::SeqCst);

    if had_exception || interrupted {
        safety_shutdown(session, true).await;
    }
    if !was_cancelled {
        base.events.job_finished();
    }
    log_crate::debug!(
        "{} streaming finished ({sent}/{total} lines).",
        session.job_log_label()
    );
}

/// Stream pre-rendered machine code: trim, drop empties, stream.
pub async fn run_raw<S: PingPongDriver>(session: &S, machine_code: &str) {
    let lines: Vec<String> = machine_code
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        return;
    }
    stream_job(session, &lines.join("\n"), &HashMap::new(), &[]).await;
}

/// The shared command path: clear a finished job's cancellation
/// flag, send the line, await its acknowledgement.
pub async fn execute_command<S: PingPongDriver>(
    session: &S,
    command: &str,
) -> Result<Vec<String>, DriverError> {
    if !session.base().job_running.load(Ordering::SeqCst) {
        session.base().is_cancelled.store(false, Ordering::SeqCst);
    }
    session.ping_pong(command).await
}

/// Like [`execute_command`], but with an explicit acknowledgement
/// budget for operations such as homing that outlast the ordinary
/// ping-pong timeout.
pub async fn execute_command_with_timeout<S: PingPongDriver>(
    session: &S,
    command: &str,
    timeout: Duration,
) -> Result<Vec<String>, DriverError> {
    if !session.base().job_running.load(Ordering::SeqCst) {
        session.base().is_cancelled.store(false, Ordering::SeqCst);
    }
    session.ping_pong_with_timeout(command, timeout).await
}

/// The shared cancel sequence: mark the job cancelled, resolve the
/// pending acknowledgements, write the protocol's stop bytes, run
/// the safety shutdown and report the job finished if one was
/// running.
pub async fn cancel<S: PingPongDriver>(
    session: &S,
    emergency: bool,
) -> Result<(), DriverError> {
    let base = session.base();
    let stop = session.stop_command();
    log_crate::debug!("Cancel command initiated.");
    let job_was_running = base.job_running.load(Ordering::SeqCst);
    base.is_cancelled.store(true, Ordering::SeqCst);
    base.job_running.store(false, Ordering::SeqCst);
    base.is_holding.store(false, Ordering::SeqCst);

    // Unblock a sender parked on a missing acknowledgement; the
    // stream loop observes the cancellation and stops.
    base.pending.resolve();
    base.line_buffer.lock().unwrap().clear();

    if !base.transport.is_open() {
        return Err(DriverError::Connection(
            session.not_connected_message().to_string(),
        ));
    }
    log_crate::info!("{}", stop.log);
    base.transport.write(stop.bytes).await?;

    safety_shutdown(session, emergency).await;

    if job_was_running {
        base.events.job_finished();
    }
    Ok(())
}

/// Best-effort transmission of the dialect's tool-off commands so a
/// cancelled or aborted job cannot leave persistent PWM outputs
/// energized.  Fire-and-forget: responses are ignored.
async fn safety_shutdown<S: PingPongDriver>(session: &S, emergency: bool) {
    let base = session.base();
    let dialect = base.dialect();
    let mut commands = dialect.safety_off_commands.clone();
    if emergency {
        if let Some(cmd) = dialect.emergency_stop.clone() {
            commands.push(cmd);
        }
    }
    if commands.is_empty() || !base.transport.is_open() {
        return;
    }
    if let Some(delay) = session.safety_shutdown_delay() {
        tokio::time::sleep(delay).await;
    }
    for command in commands {
        if !base.transport.is_open() {
            break;
        }
        if let Err(err) = base
            .transport
            .write(format!("{command}\n").as_bytes())
            .await
        {
            log_crate::warn!("Safety command '{command}' failed: {err}");
        }
    }
}

/// Home all axes or *axes*; *active_wcs* is irrelevant for the
/// generic path (protocols with a re-assertion step implement their
/// own homing).
pub async fn home<S: PingPongDriver>(
    session: &S,
    axes: Option<Vec<String>>,
) -> Result<(), DriverError> {
    let dialect = session.base().dialect();
    match axes {
        None => {
            execute_command(session, &dialect.home_all).await?;
        }
        Some(axes) => {
            for axis in axes {
                let cmd = format_template(
                    &dialect.home_axis,
                    &[("axis_letter", Arg::Str(axis))],
                )
                .map_err(DriverError::Other)?;
                execute_command(session, &cmd).await?;
            }
        }
    }
    session.base().clear_device_error();
    Ok(())
}

/// Absolute move in the machine's current coordinate frame.  *z* is
/// ignored: the GRBL-family `move_to` templates carry no Z word.
pub async fn move_to<S: PingPongDriver>(
    session: &S,
    speed: f64,
    pos_x: f64,
    pos_y: f64,
) -> Result<(), DriverError> {
    let dialect = session.base().dialect();
    let cmd = format_template(
        &dialect.move_to,
        &[
            ("speed", Arg::Speed(speed)),
            ("x", Arg::Length(pos_x)),
            ("y", Arg::Length(pos_y)),
        ],
    )
    .map_err(DriverError::Other)?;
    execute_command(session, &cmd).await?;
    Ok(())
}

/// Relative jog by per-axis distances.
pub async fn jog<S: PingPongDriver>(
    session: &S,
    speed: f64,
    deltas: &[(String, f64)],
) -> Result<(), DriverError> {
    let dialect = session.base().dialect();
    let head = format_template(&dialect.jog, &[("speed", Arg::Speed(speed))])
        .map_err(DriverError::Other)?;
    let mut cmd_parts = vec![head];
    for (axis_name, distance) in deltas {
        cmd_parts.push(format!(
            "{}{}",
            axis_name.to_uppercase(),
            crate::driver::core::dialect::python_float_repr(*distance)
        ));
    }
    if cmd_parts.len() == 1 {
        return Ok(());
    }
    let cmd = cmd_parts.join(" ");
    execute_command(session, &cmd).await?;
    Ok(())
}

/// Switch to *tool_number*.
pub async fn select_tool<S: PingPongDriver>(
    session: &S,
    tool_number: i64,
) -> Result<(), DriverError> {
    let dialect = session.base().dialect();
    let cmd = format_template(
        &dialect.tool_change,
        &[("tool_number", Arg::Int(tool_number))],
    )
    .map_err(DriverError::Other)?;
    execute_command(session, &cmd).await?;
    Ok(())
}

/// Laser/spindle power; `None` disables the tool.
pub async fn set_power<S: PingPongDriver>(
    session: &S,
    power: Option<f64>,
) -> Result<(), DriverError> {
    let dialect = session.base().dialect();
    let cmd = match power {
        Some(p) if p > 0.0 => {
            format_template(&dialect.laser_on, &[("power", Arg::Length(p))])
                .map_err(DriverError::Other)?
        }
        _ => dialect.laser_off.clone(),
    };
    execute_command(session, &cmd).await?;
    Ok(())
}

/// Laser power used while focusing; `None` disables it.
pub async fn set_focus_power<S: PingPongDriver>(
    session: &S,
    power: Option<f64>,
) -> Result<(), DriverError> {
    let dialect = session.base().dialect();
    let cmd = match power {
        Some(p) if p > 0.0 => format_template(
            &dialect.focus_laser_on,
            &[("power", Arg::Length(p))],
        )
        .map_err(DriverError::Other)?,
        _ => dialect.laser_off.clone(),
    };
    execute_command(session, &cmd).await?;
    Ok(())
}
