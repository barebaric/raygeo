//! Device-host daemon for remote sessions.
//!
//! Hosts sessions next to the hardware and serves the control
//! protocol over TCP, optionally TLS. Jobs run daemon-side: a
//! submitted job survives client disconnects, and reattaching clients
//! resync from the event ring or a snapshot.

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};
use tokio::io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Notify};
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;

use crate::driver::core::discovery::daemon::is_loopback;
use crate::driver::core::driver::DriverError;
use crate::driver::core::remote::protocol::{
    self, ProtocolError, WireError, WireEvent,
};
use crate::driver::core::remote::registry;
use crate::driver::core::remote::session::{
    HostedSession, RemoteEventSink, SessionFactory,
};
use crate::driver::core::types::DeviceState;

pub const DEFAULT_PORT: u16 = 8765;
pub const DEFAULT_RING_SIZE: usize = 1000;
pub const DEFAULT_LEASE_GRACE: f64 = 5.0;
pub const SEND_QUEUE_LIMIT: usize = 1000;
pub const AUTH_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub enum DaemonError {
    Insecure(String),
    Io(String),
    Tls(String),
    Config(String),
    Protocol(ProtocolError),
}

impl std::fmt::Display for DaemonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Insecure(message) => f.write_str(message),
            Self::Io(message) => f.write_str(message),
            Self::Tls(message) => f.write_str(message),
            Self::Config(message) => f.write_str(message),
            Self::Protocol(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for DaemonError {}

impl From<ProtocolError> for DaemonError {
    fn from(err: ProtocolError) -> Self {
        Self::Protocol(err)
    }
}

impl From<std::io::Error> for DaemonError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

/// Listener and session-host settings (the daemon.json shape).
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub host: String,
    pub port: u16,
    pub token: Option<String>,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
    pub keepalive_interval: f64,
    pub keepalive_misses: u32,
    pub lease_grace: f64,
    pub ring_size: usize,
    pub cameras: Value,
    pub sessions: Value,
    pub mdns: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: DEFAULT_PORT,
            token: None,
            tls_cert: None,
            tls_key: None,
            keepalive_interval: protocol::DEFAULT_KEEPALIVE_INTERVAL,
            keepalive_misses: protocol::DEFAULT_KEEPALIVE_MISSES,
            lease_grace: DEFAULT_LEASE_GRACE,
            ring_size: DEFAULT_RING_SIZE,
            cameras: json!([]),
            sessions: json!([]),
            mdns: true,
        }
    }
}

pub fn validate_bind_policy(
    host: &str,
    token: Option<&str>,
    tls_cert: Option<&str>,
) -> Result<(), DaemonError> {
    if is_loopback(host) {
        return Ok(());
    }
    if token.is_none() {
        return Err(DaemonError::Insecure(
            "exposed listeners require a token".to_string(),
        ));
    }
    if tls_cert.is_none() {
        return Err(DaemonError::Insecure(
            "exposed listeners require TLS".to_string(),
        ));
    }
    Ok(())
}

/// Bounded per-client send buffer; drops raw events first.
pub struct ClientQueue {
    limit: usize,
    frames: Mutex<VecDeque<Value>>,
    ready: Notify,
}

impl ClientQueue {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            frames: Mutex::new(VecDeque::new()),
            ready: Notify::new(),
        }
    }

    pub fn try_push(&self, frame: Value) -> bool {
        if !self.push(frame) {
            return false;
        }
        self.ready.notify_one();
        true
    }

    /// Push a frame, dropping raw events first when full.
    pub fn push(&self, frame: Value) -> bool {
        let mut frames = self.frames.lock().unwrap();
        if frames.len() >= self.limit {
            frames.retain(|frame| {
                !matches!(
                    frame
                        .get("payload")
                        .and_then(|payload| payload.get("event"))
                        .and_then(|event| event.get("kind"))
                        .and_then(Value::as_str),
                    Some("raw_rx") | Some("raw_tx")
                )
            });
        }
        if frames.len() >= self.limit {
            return false;
        }
        frames.push_back(frame);
        true
    }

    pub async fn get(&self) -> Value {
        loop {
            let notified = self.ready.notified();
            if let Some(frame) = self.frames.lock().unwrap().pop_front() {
                return frame;
            }
            notified.await;
        }
    }
}

struct Client {
    id: String,
    address: String,
    connected_at: f64,
    queue: Arc<ClientQueue>,
    authenticated: AtomicBool,
    closed: AtomicBool,
    last_rx: Mutex<Instant>,
    roles: Mutex<HashMap<String, String>>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
    shutdown: Notify,
}

impl Client {
    fn send(&self, frame: Value) -> bool {
        self.queue.try_push(frame)
    }
}

struct LeaseState {
    holder: Option<String>,
    in_grace: bool,
    generation: u64,
}

struct Job {
    data: Value,
}

struct Host {
    id: String,
    protocol: String,
    config: Value,
    dialect: Value,
    session: Arc<dyn HostedSession>,
    ring: Mutex<VecDeque<(i64, Value)>>,
    next_seq: AtomicI64,
    subscribers: Mutex<HashMap<String, Arc<Client>>>,
    lease: Mutex<LeaseState>,
    job: Mutex<Option<Job>>,
    job_task: Mutex<Option<JoinHandle<()>>>,
    connection_status: Mutex<String>,
}

impl Host {
    fn snapshot(&self) -> Value {
        let job = self
            .job
            .lock()
            .unwrap()
            .as_ref()
            .map(|job| job.data.clone());
        json!({
            "protocol": self.protocol,
            "config": self.config,
            "dialect": self.dialect,
            "state": protocol::encode_state(&self.session.state()),
            "connection_status": self.connection_status.lock().unwrap().clone(),
            "job": job,
            "next_seq": self.next_seq.load(Ordering::SeqCst),
        })
    }
}

struct ListenerHandle {
    task: JoinHandle<()>,
    stop: Arc<Notify>,
    #[allow(dead_code)]
    addr: SocketAddr,
}

/// A daemon listening on a socket.
pub struct StartedServer {
    pub addr: SocketAddr,
}

/// Driver host serving the remote control protocol.
pub struct Daemon {
    config: Arc<Mutex<DaemonConfig>>,
    factory: Arc<dyn SessionFactory>,
    /// The embedder-supplied protocol table (core carries no
    /// protocol knowledge of its own).
    registry: registry::Registry,
    hosts: Mutex<HashMap<String, Arc<Host>>>,
    clients: Mutex<HashMap<String, Arc<Client>>>,
    listener: Mutex<Option<ListenerHandle>>,
}

impl Daemon {
    pub fn new(
        config: Arc<Mutex<DaemonConfig>>,
        factory: Arc<dyn SessionFactory>,
        registry: registry::Registry,
    ) -> Self {
        Self {
            config,
            factory,
            registry,
            hosts: Mutex::new(HashMap::new()),
            clients: Mutex::new(HashMap::new()),
            listener: Mutex::new(None),
        }
    }

    /// The live daemon configuration.  The embedding application
    /// shares this handle and may adjust settings (cameras, sessions,
    /// keepalive) between calls; the daemon re-reads it per request.
    pub fn config(&self) -> Arc<Mutex<DaemonConfig>> {
        self.config.clone()
    }

    /// Open the listener and auto-open configured sessions.
    pub async fn start(self: &Arc<Self>) -> Result<StartedServer, DaemonError> {
        let (host, port, token, tls_cert, tls_key) = {
            let cfg = self.config.lock().unwrap();
            (
                cfg.host.clone(),
                cfg.port,
                cfg.token.clone(),
                cfg.tls_cert.clone(),
                cfg.tls_key.clone(),
            )
        };
        validate_bind_policy(&host, token.as_deref(), tls_cert.as_deref())?;
        let acceptor = match &tls_cert {
            Some(cert) => {
                let key = tls_key.as_deref().ok_or_else(|| {
                    DaemonError::Insecure(
                        "tls_cert also requires tls_key".to_string(),
                    )
                })?;
                Some(tls_acceptor(cert, key)?)
            }
            None => None,
        };

        let listener = TcpListener::bind((host.as_str(), port)).await?;
        let addr = listener.local_addr()?;
        let stop = Arc::new(Notify::new());
        let daemon = self.clone();
        let accept_stop = stop.clone();
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = accept_stop.notified() => break,
                    accepted = listener.accept() => {
                        let Ok((stream, peer)) = accepted else {
                            break;
                        };
                        let daemon = daemon.clone();
                        let acceptor = acceptor.clone();
                        tokio::spawn(async move {
                            daemon.handle_connection(stream, peer, acceptor).await;
                        });
                    }
                }
            }
        });
        *self.listener.lock().unwrap() =
            Some(ListenerHandle { task, stop, addr });

        let sessions = self
            .config
            .lock()
            .unwrap()
            .sessions
            .as_array()
            .cloned()
            .unwrap_or_default();
        for entry in sessions {
            let protocol_name = entry
                .get("protocol")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let config =
                entry.get("config").cloned().unwrap_or_else(|| json!({}));
            let dialect =
                entry.get("dialect").cloned().unwrap_or_else(|| json!({}));
            self.open_host(&protocol_name, config, dialect).await?;
        }
        Ok(StartedServer { addr })
    }

    /// Close every session and connection, then the listener.
    pub async fn stop(self: &Arc<Self>) {
        let listener = self.listener.lock().unwrap().take();
        if let Some(listener) = listener {
            listener.stop.notify_one();
            listener.task.abort();
        }
        let hosts: Vec<Arc<Host>> =
            self.hosts.lock().unwrap().values().cloned().collect();
        for host in hosts {
            self.close_host(&host).await;
        }
        let clients: Vec<Arc<Client>> =
            self.clients.lock().unwrap().values().cloned().collect();
        for client in clients {
            self.disconnect(client).await;
        }
    }

    // -- connection lifecycle ----------------------------------------

    async fn handle_connection(
        self: Arc<Self>,
        stream: TcpStream,
        peer: SocketAddr,
        acceptor: Option<TlsAcceptor>,
    ) {
        match acceptor {
            Some(acceptor) => match acceptor.accept(stream).await {
                Ok(stream) => {
                    self.serve_connection(stream, peer).await;
                }
                Err(err) => log_crate::debug!("TLS handshake failed: {err}"),
            },
            None => self.serve_connection(stream, peer).await,
        }
    }

    async fn serve_connection<S>(self: Arc<Self>, stream: S, peer: SocketAddr)
    where
        S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        let (mut reader, mut writer) = tokio::io::split(stream);
        let client = Arc::new(Client {
            id: uuid::Uuid::new_v4().simple().to_string()[..12].to_string(),
            address: peer.to_string(),
            connected_at: unix_time(),
            queue: Arc::new(ClientQueue::new(SEND_QUEUE_LIMIT)),
            authenticated: AtomicBool::new(false),
            closed: AtomicBool::new(false),
            last_rx: Mutex::new(Instant::now()),
            roles: Mutex::new(HashMap::new()),
            tasks: Mutex::new(Vec::new()),
            shutdown: Notify::new(),
        });
        self.clients
            .lock()
            .unwrap()
            .insert(client.id.clone(), client.clone());

        let welcome = json!({
            "type": protocol::WELCOME,
            "payload": protocol::welcome_payload(
                &crate::driver::core::remote::version(),
                self.config.lock().unwrap().token.is_some(),
            ),
        });
        if let Err(err) = protocol::write_frame(&mut writer, &welcome).await {
            log_crate::debug!("welcome write failed: {err}");
            self.disconnect(client).await;
            return;
        }
        if !self.authenticate(&client, &mut reader, &mut writer).await {
            self.disconnect(client).await;
            return;
        }

        let writer_client = client.clone();
        let writer_daemon = self.clone();
        let writer_task = tokio::spawn(async move {
            writer_loop(writer_daemon, writer_client, writer).await;
        });
        client.tasks.lock().unwrap().push(writer_task);

        let keepalive_client = client.clone();
        let keepalive_daemon = self.clone();
        let keepalive_task = tokio::spawn(async move {
            keepalive_loop(keepalive_daemon, keepalive_client).await;
        });
        client.tasks.lock().unwrap().push(keepalive_task);

        loop {
            tokio::select! {
                _ = client.shutdown.notified() => break,
                frame = protocol::read_frame(&mut reader) => match frame {
                    Ok(message) => {
                        *client.last_rx.lock().unwrap() = Instant::now();
                        if let Some(reply) =
                            self.dispatch(&client, message).await
                        {
                            if !client.send(reply) {
                                break;
                            }
                        }
                    }
                    Err(WireError::Connection(message)) => {
                        log_crate::debug!(
                            "client {} dropped: {message}",
                            client.id
                        );
                        break;
                    }
                    Err(WireError::Protocol(err)) => {
                        log_crate::warn!("client {} dropped: {err}", client.id);
                        break;
                    }
                },
            }
        }
        self.disconnect(client).await;
    }

    async fn authenticate<R, W>(
        &self,
        client: &Arc<Client>,
        reader: &mut ReadHalf<R>,
        writer: &mut WriteHalf<W>,
    ) -> bool
    where
        R: AsyncRead + Send + Unpin,
        W: AsyncWrite + Send + Unpin,
    {
        let token = self.config.lock().unwrap().token.clone();
        let Some(token) = token else {
            client.authenticated.store(true, Ordering::SeqCst);
            return true;
        };
        let message = match tokio::time::timeout(
            AUTH_TIMEOUT,
            protocol::read_frame(reader),
        )
        .await
        {
            Ok(Ok(message)) => message,
            Ok(Err(_)) | Err(_) => return false,
        };
        *client.last_rx.lock().unwrap() = Instant::now();
        let id = message.get("id").and_then(Value::as_i64).unwrap_or(0);
        let supplied = message
            .get("payload")
            .and_then(|payload| payload.get("token"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let is_auth =
            message.get("type").and_then(Value::as_str) == Some(protocol::AUTH);
        if !is_auth || !tokens_equal(supplied, &token) {
            let reply = protocol::error_response(
                id,
                protocol::AUTH_FAILED,
                "invalid token",
            );
            let _ = protocol::write_frame(writer, &reply).await;
            return false;
        }
        client.authenticated.store(true, Ordering::SeqCst);
        let _ =
            protocol::write_frame(writer, &protocol::response(id, json!({})))
                .await;
        true
    }

    async fn disconnect(self: &Arc<Self>, client: Arc<Client>) {
        if client.closed.swap(true, Ordering::SeqCst) {
            return;
        }
        let hosts: Vec<Arc<Host>> =
            self.hosts.lock().unwrap().values().cloned().collect();
        for host in hosts {
            if host.subscribers.lock().unwrap().contains_key(&client.id) {
                self.unsubscribe(&host, &client).await;
            }
        }
        client.shutdown.notify_waiters();
        let tasks = std::mem::take(&mut *client.tasks.lock().unwrap());
        for task in tasks {
            task.abort();
        }
        self.clients.lock().unwrap().remove(&client.id);
    }

    // -- dispatch ------------------------------------------------------

    async fn dispatch(
        self: &Arc<Self>,
        client: &Arc<Client>,
        message: Value,
    ) -> Option<Value> {
        let id = message.get("id").and_then(Value::as_i64).unwrap_or(0);
        let type_ = message
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let payload =
            message.get("payload").cloned().unwrap_or_else(|| json!({}));
        if type_ == protocol::PING {
            return Some(json!({"type": protocol::PONG}));
        }
        if type_ == protocol::PONG {
            return None;
        }
        if type_ == protocol::AUTH {
            return Some(protocol::response(id, json!({})));
        }
        let result = match type_ {
            protocol::DEVICE_LIST => self.device_list().await,
            protocol::DRIVER_LIST => Ok(self.registry.drivers_payload()),
            protocol::SESSION_LIST => self.session_list().await,
            protocol::SESSION_OPEN => self.session_open(client, payload).await,
            protocol::SESSION_ATTACH => {
                self.session_attach(client, payload).await
            }
            protocol::SESSION_DETACH => {
                self.session_detach(client, payload).await
            }
            protocol::SESSION_CLOSE => {
                self.session_close(client, payload).await
            }
            protocol::SESSION_COMMAND => {
                self.session_command(client, payload).await
            }
            protocol::SESSION_PROPERTY => self.session_property(payload).await,
            protocol::CLIENT_LIST => self.client_list().await,
            _ => Err(ProtocolError::new(
                protocol::UNKNOWN_TYPE,
                format!("unknown type {type_:?}"),
            )),
        };
        Some(match result {
            Ok(payload) => protocol::response(id, payload),
            Err(err) => protocol::error_response_with_kind(
                id,
                &err.code,
                &err.message,
                err.kind,
            ),
        })
    }

    async fn device_list(&self) -> Result<Value, ProtocolError> {
        let cfg = self.config.lock().unwrap().clone();
        Ok(json!({
            "ports": serial_ports(),
            "cameras": cfg.cameras,
            "devices": self.devices(&cfg),
        }))
    }

    fn devices(&self, cfg: &DaemonConfig) -> Value {
        let mut devices: Map<String, Value> = Map::new();
        if let Some(entries) = cfg.sessions.as_array() {
            for entry in entries {
                let config =
                    entry.get("config").cloned().unwrap_or_else(|| json!({}));
                let reference = device_ref(&config);
                let protocol_name = entry
                    .get("protocol")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if reference.is_empty()
                    || !self.registry.is_known(protocol_name)
                {
                    continue;
                }
                let label = self
                    .registry
                    .driver_info(protocol_name)
                    .map(|info| info.name)
                    .unwrap_or_default();
                devices.insert(
                    reference.clone(),
                    json!({
                        "device": reference,
                        "protocol": protocol_name,
                        "label": label,
                        "config": config,
                        "session_id": null,
                        "connection_status": null,
                    }),
                );
            }
        }
        let hosts: Vec<Arc<Host>> =
            self.hosts.lock().unwrap().values().cloned().collect();
        for host in hosts {
            let reference = device_ref(&host.config);
            if reference.is_empty() {
                continue;
            }
            let label = self
                .registry
                .driver_info(&host.protocol)
                .map(|info| info.name)
                .unwrap_or_default();
            devices.insert(
                reference.clone(),
                json!({
                    "device": reference,
                    "protocol": host.protocol,
                    "label": label,
                    "config": host.config,
                    "session_id": host.id,
                    "connection_status":
                        host.connection_status.lock().unwrap().clone(),
                }),
            );
        }
        Value::Array(devices.into_values().collect())
    }

    async fn session_list(&self) -> Result<Value, ProtocolError> {
        let hosts: Vec<Arc<Host>> =
            self.hosts.lock().unwrap().values().cloned().collect();
        let sessions = hosts
            .into_iter()
            .map(|host| {
                json!({
                    "session_id": host.id,
                    "protocol": host.protocol,
                    "config": host.config,
                    "connection_status":
                        host.connection_status.lock().unwrap().clone(),
                    "job": host.job.lock().unwrap().as_ref()
                        .map(|job| job.data.clone()),
                    "lease_holder":
                        host.lease.lock().unwrap().holder.clone(),
                    "watchers": host.subscribers.lock().unwrap().len(),
                })
            })
            .collect::<Vec<_>>();
        Ok(json!({"sessions": sessions}))
    }

    async fn client_list(&self) -> Result<Value, ProtocolError> {
        let clients: Vec<Arc<Client>> =
            self.clients.lock().unwrap().values().cloned().collect();
        let clients = clients
            .into_iter()
            .map(|client| {
                json!({
                    "id": client.id,
                    "address": client.address,
                    "authenticated":
                        client.authenticated.load(Ordering::SeqCst),
                    "connected_at": client.connected_at,
                    "roles": client.roles.lock().unwrap().clone(),
                })
            })
            .collect::<Vec<_>>();
        Ok(json!({"clients": clients}))
    }

    async fn session_open(
        self: &Arc<Self>,
        client: &Arc<Client>,
        payload: Value,
    ) -> Result<Value, ProtocolError> {
        let device = payload.get("device").and_then(Value::as_str);
        let host = if let Some(device) = device {
            match self.resolve_device(device) {
                Some(host) => host,
                None => match self.probe_device(device).await {
                    Some((protocol_name, config, dialect)) => {
                        self.open_host(&protocol_name, config, dialect).await?
                    }
                    None => {
                        return Err(ProtocolError::new(
                            protocol::UNKNOWN_DEVICE,
                            format!(
                                "device {device:?} is not configured on \
                                 this daemon; add it with 'raydriver daemon \
                                 add'"
                            ),
                        ))
                    }
                },
            }
        } else {
            self.open_host(
                payload
                    .get("protocol")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                payload.get("config").cloned().unwrap_or_else(|| json!({})),
                payload.get("dialect").cloned().unwrap_or_else(|| json!({})),
            )
            .await?
        };
        self.subscribe(&host, client, &payload).await?;
        let role = client
            .roles
            .lock()
            .unwrap()
            .get(&host.id)
            .cloned()
            .unwrap_or_else(|| "watch".to_string());
        let metadata = self
            .registry
            .driver_info(&host.protocol)
            .map(|info| info.to_json())
            .unwrap_or(Value::Null);
        Ok(json!({
            "session_id": host.id,
            "protocol": host.protocol,
            "role": role,
            "lease_holder": host.lease.lock().unwrap().holder,
            "metadata": metadata,
        }))
    }

    async fn session_attach(
        self: &Arc<Self>,
        client: &Arc<Client>,
        payload: Value,
    ) -> Result<Value, ProtocolError> {
        let host = self.host(payload.get("session_id"))?;
        self.subscribe(&host, client, &payload).await?;
        let after_seq = payload.get("after_seq").and_then(Value::as_i64);
        if let Some(after_seq) = after_seq {
            let ring = host.ring.lock().unwrap();
            if !ring.is_empty() && ring.front().unwrap().0 <= after_seq + 1 {
                let mut replayed = 0;
                for (seq, event) in ring.iter() {
                    if *seq > after_seq {
                        client.send(protocol::event_frame(
                            &host.id,
                            *seq,
                            event.clone(),
                        ));
                        replayed += 1;
                    }
                }
                return Ok(json!({
                    "replayed": replayed,
                    "next_seq": host.next_seq.load(Ordering::SeqCst),
                }));
            }
        }
        let mut snapshot = host.snapshot();
        snapshot["metadata"] = self
            .registry
            .driver_info(&host.protocol)
            .map(|info| info.to_json())
            .unwrap_or(Value::Null);
        Ok(json!({"snapshot": snapshot}))
    }

    async fn session_detach(
        self: &Arc<Self>,
        client: &Arc<Client>,
        payload: Value,
    ) -> Result<Value, ProtocolError> {
        let host = self.host(payload.get("session_id"))?;
        self.unsubscribe(&host, client).await;
        Ok(json!({}))
    }

    async fn session_close(
        self: &Arc<Self>,
        client: &Arc<Client>,
        payload: Value,
    ) -> Result<Value, ProtocolError> {
        let host = self.host(payload.get("session_id"))?;
        require_control(&host, client)?;
        self.close_host(&host).await;
        Ok(json!({}))
    }

    async fn session_command(
        self: &Arc<Self>,
        client: &Arc<Client>,
        payload: Value,
    ) -> Result<Value, ProtocolError> {
        let host = self.host(payload.get("session_id"))?;
        require_control(&host, client)?;
        let method = payload
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let args = payload.get("args").cloned().unwrap_or_else(|| json!({}));
        if method == "run" {
            return self.start_job(&host, args).await;
        }
        if method == "connect" {
            host.session.connect().await.map_err(internal)?;
            return Ok(json!({"result": null}));
        }
        if method == "disconnect" {
            host.session.disconnect().await.map_err(internal)?;
            return Ok(json!({"result": null}));
        }
        let result =
            host.session.execute(method, args).await.map_err(internal)?;
        Ok(json!({"result": result}))
    }

    async fn session_property(
        &self,
        payload: Value,
    ) -> Result<Value, ProtocolError> {
        let host = self.host(payload.get("session_id"))?;
        let name = payload
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let value = match name {
            "state" => protocol::encode_state(&host.session.state()),
            "job_running" => json!(host.session.job_running()),
            "resource_uri" => json!(host.session.resource_uri()),
            "buffer_count" => json!(host.session.buffer_count()),
            "rx_buffer_size" => json!(host.session.rx_buffer_size()),
            "pending_commands" => json!(host.session.pending_commands()),
            _ => {
                return Err(ProtocolError::new(
                    protocol::UNKNOWN_PROPERTY,
                    format!("unknown property {name:?}"),
                ))
            }
        };
        Ok(json!({"value": value}))
    }

    // -- sessions ------------------------------------------------------

    async fn open_host(
        self: &Arc<Self>,
        protocol_name: &str,
        config: Value,
        dialect: Value,
    ) -> Result<Arc<Host>, ProtocolError> {
        if !self.registry.is_known(protocol_name) {
            return Err(ProtocolError::new(
                protocol::BAD_REQUEST,
                format!("unknown protocol {protocol_name:?}"),
            ));
        }
        let (tx, rx) = mpsc::unbounded_channel();
        let sink = Arc::new(ChannelSink { tx });
        let session = self
            .factory
            .create(protocol_name, config.clone(), dialect.clone(), sink)
            .map_err(internal)?;
        let host = Arc::new(Host {
            id: uuid::Uuid::new_v4().simple().to_string()[..12].to_string(),
            protocol: protocol_name.to_string(),
            config,
            dialect,
            session,
            ring: Mutex::new(VecDeque::new()),
            next_seq: AtomicI64::new(1),
            subscribers: Mutex::new(HashMap::new()),
            lease: Mutex::new(LeaseState {
                holder: None,
                in_grace: false,
                generation: 0,
            }),
            job: Mutex::new(None),
            job_task: Mutex::new(None),
            connection_status: Mutex::new("DISCONNECTED".to_string()),
        });
        self.hosts
            .lock()
            .unwrap()
            .insert(host.id.clone(), host.clone());
        let event_host = host.clone();
        let event_daemon = self.clone();
        tokio::spawn(async move {
            let mut rx = rx;
            while let Some(event) = rx.recv().await {
                event_daemon.on_event(&event_host, event);
            }
        });
        Ok(host)
    }

    async fn close_host(self: &Arc<Self>, host: &Arc<Host>) {
        host.lease.lock().unwrap().generation += 1;
        if let Some(task) = host.job_task.lock().unwrap().take() {
            task.abort();
        }
        if let Err(err) = host.session.disconnect().await {
            log_crate::warn!("session disconnect failed: {err}");
        }
        self.fan_out(
            host,
            WireEvent::Lease {
                holder: None,
                reason: "closed".to_string(),
            },
        );
        let subscribers: Vec<Arc<Client>> =
            host.subscribers.lock().unwrap().values().cloned().collect();
        for client in subscribers {
            client.roles.lock().unwrap().remove(&host.id);
        }
        host.subscribers.lock().unwrap().clear();
        self.hosts.lock().unwrap().remove(&host.id);
    }

    fn host(
        &self,
        session_id: Option<&Value>,
    ) -> Result<Arc<Host>, ProtocolError> {
        let id = session_id.and_then(Value::as_str).unwrap_or_default();
        self.hosts.lock().unwrap().get(id).cloned().ok_or_else(|| {
            ProtocolError::new(
                protocol::UNKNOWN_SESSION,
                format!("unknown session {id:?}"),
            )
        })
    }

    fn resolve_device(&self, reference: &str) -> Option<Arc<Host>> {
        let hosts: Vec<Arc<Host>> =
            self.hosts.lock().unwrap().values().cloned().collect();
        hosts
            .into_iter()
            .find(|host| device_ref(&host.config) == reference)
    }

    async fn probe_device(
        &self,
        reference: &str,
    ) -> Option<(String, Value, Value)> {
        let candidates: Vec<(
            &str,
            &crate::driver::core::driver::ProbeFingerprint,
        )> = self
            .registry
            .drivers()
            .iter()
            .filter_map(|info| {
                info.fingerprint
                    .as_ref()
                    .map(|fingerprint| (info.name, fingerprint))
            })
            .collect();
        for (name, fingerprint) in candidates {
            let config = json!({
                "port": reference,
                "baudrate": 115200,
                "handshake_timeout": 1.0,
                "command_timeout": 1.0,
            });
            let (_tx, _rx) = mpsc::unbounded_channel();
            let sink = Arc::new(ChannelSink { tx: _tx });
            let Ok(session) =
                self.factory.create(name, config.clone(), json!({}), sink)
            else {
                continue;
            };
            if tokio::time::timeout(Duration::from_secs(3), session.connect())
                .await
                .ok()
                .and_then(Result::ok)
                .is_none()
            {
                let _ = session.disconnect().await;
                continue;
            }
            let reply = tokio::time::timeout(
                Duration::from_secs(3),
                session.execute(
                    "execute_interactive_command",
                    json!({"command": fingerprint.command}),
                ),
            )
            .await;
            if let Ok(Ok(value)) = reply {
                let text = value.to_string();
                if text.contains(fingerprint.signature) {
                    let _ = session.disconnect().await;
                    return Some((name.to_string(), config, json!({})));
                }
            }
            let _ = session.disconnect().await;
        }
        None
    }

    // -- subscriptions and lease ---------------------------------------

    async fn subscribe(
        self: &Arc<Self>,
        host: &Arc<Host>,
        client: &Arc<Client>,
        payload: &Value,
    ) -> Result<(), ProtocolError> {
        let role = payload
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("watch");
        if role == "control" {
            let force = payload
                .get("force")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            self.request_lease(host, client, force)?;
        } else {
            client
                .roles
                .lock()
                .unwrap()
                .insert(host.id.clone(), "watch".to_string());
        }
        host.subscribers
            .lock()
            .unwrap()
            .insert(client.id.clone(), client.clone());
        Ok(())
    }

    async fn unsubscribe(
        self: &Arc<Self>,
        host: &Arc<Host>,
        client: &Arc<Client>,
    ) {
        host.subscribers.lock().unwrap().remove(&client.id);
        client.roles.lock().unwrap().remove(&host.id);
        let holder = host.lease.lock().unwrap().holder.clone();
        if holder.as_deref() == Some(client.id.as_str()) {
            self.start_grace(host, &client.id);
        }
    }

    fn request_lease(
        self: &Arc<Self>,
        host: &Arc<Host>,
        client: &Arc<Client>,
        force: bool,
    ) -> Result<(), ProtocolError> {
        let (holder, in_grace, generation) = {
            let lease = host.lease.lock().unwrap();
            (lease.holder.clone(), lease.in_grace, lease.generation)
        };
        if holder.is_none() || holder.as_deref() == Some(&client.id) {
            self.grant_lease(host, client, "granted", generation + 1);
            return Ok(());
        }
        if in_grace {
            self.grant_lease(host, client, "granted", generation + 1);
            return Ok(());
        }
        if force {
            if let Some(old_id) = &holder {
                let old = self.clients.lock().unwrap().get(old_id).cloned();
                if let Some(old) = old {
                    old.roles
                        .lock()
                        .unwrap()
                        .insert(host.id.clone(), "watch".to_string());
                }
            }
            self.grant_lease(host, client, "stolen", generation + 1);
            return Ok(());
        }
        Err(ProtocolError::new(
            protocol::LEASE_TAKEN,
            format!("control lease held by {holder:?}"),
        ))
    }

    fn grant_lease(
        self: &Arc<Self>,
        host: &Arc<Host>,
        client: &Arc<Client>,
        reason: &str,
        generation: u64,
    ) {
        {
            let mut lease = host.lease.lock().unwrap();
            lease.holder = Some(client.id.clone());
            lease.in_grace = false;
            lease.generation = generation;
        }
        client
            .roles
            .lock()
            .unwrap()
            .insert(host.id.clone(), "control".to_string());
        self.fan_out(
            host,
            WireEvent::Lease {
                holder: Some(client.id.clone()),
                reason: reason.to_string(),
            },
        );
    }

    fn start_grace(self: &Arc<Self>, host: &Arc<Host>, client_id: &str) {
        let generation = {
            let mut lease = host.lease.lock().unwrap();
            if lease.holder.as_deref() != Some(client_id) || lease.in_grace {
                return;
            }
            lease.in_grace = true;
            lease.generation += 1;
            lease.generation
        };
        let host = host.clone();
        let daemon = self.clone();
        let client_id = client_id.to_string();
        let grace = self.config.lock().unwrap().lease_grace;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs_f64(grace)).await;
            let release = {
                let mut lease = host.lease.lock().unwrap();
                if lease.holder.as_deref() == Some(&client_id)
                    && lease.in_grace
                    && lease.generation == generation
                {
                    lease.holder = None;
                    lease.in_grace = false;
                    true
                } else {
                    false
                }
            };
            if release {
                daemon.fan_out(
                    &host,
                    WireEvent::Lease {
                        holder: None,
                        reason: "released".to_string(),
                    },
                );
            }
        });
    }

    // -- events and jobs -------------------------------------------------

    fn on_event(self: &Arc<Self>, host: &Arc<Host>, event: WireEvent) {
        match &event {
            WireEvent::ConnectionStatus { status, .. } => {
                *host.connection_status.lock().unwrap() = status.clone();
            }
            WireEvent::JobFinished => {
                if let Some(job) = host.job.lock().unwrap().as_mut() {
                    job.data["running"] = json!(false);
                }
            }
            WireEvent::Progress { op } => {
                if let Some(job) = host.job.lock().unwrap().as_mut() {
                    job.data["current_op"] = json!(op);
                }
            }
            _ => {}
        }
        self.fan_out(host, event);
    }

    fn fan_out(self: &Arc<Self>, host: &Arc<Host>, event: WireEvent) {
        let seq = host.next_seq.fetch_add(1, Ordering::SeqCst);
        let json = event.to_json();
        {
            let mut ring = host.ring.lock().unwrap();
            let limit = self.config.lock().unwrap().ring_size;
            ring.push_back((seq, json.clone()));
            while ring.len() > limit {
                ring.pop_front();
            }
        }
        let frame = protocol::event_frame(&host.id, seq, json);
        let subscribers: Vec<Arc<Client>> =
            host.subscribers.lock().unwrap().values().cloned().collect();
        for client in subscribers {
            if !client.send(frame.clone()) {
                host.subscribers.lock().unwrap().remove(&client.id);
                let daemon = self.clone();
                let client = client.clone();
                tokio::spawn(async move {
                    daemon.disconnect(client).await;
                });
            }
        }
    }

    async fn start_job(
        self: &Arc<Self>,
        host: &Arc<Host>,
        args: Value,
    ) -> Result<Value, ProtocolError> {
        {
            let job = host.job.lock().unwrap();
            if job.as_ref().is_some_and(|job| {
                job.data
                    .get("running")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            }) {
                return Err(ProtocolError::new(
                    protocol::JOB_RUNNING,
                    "a job is already running",
                ));
            }
        }
        let op_map = args
            .get("op_line_map")
            .and_then(Value::as_array)
            .map(|pairs| {
                pairs
                    .iter()
                    .filter_map(|pair| {
                        let pair = pair.as_array()?;
                        Some((
                            pair.first()?.as_i64()? as usize,
                            pair.get(1)?.as_i64()?,
                        ))
                    })
                    .collect::<HashMap<usize, i64>>()
            })
            .unwrap_or_default();
        let handle =
            uuid::Uuid::new_v4().simple().to_string()[..12].to_string();
        let total_ops = if op_map.is_empty() {
            Value::Null
        } else {
            json!(op_map.values().max().copied().unwrap_or(0) + 1)
        };
        let job = Job {
            data: json!({
                "handle": handle,
                "total_ops": total_ops,
                "current_op": 0,
                "running": true,
                "submitted_at": unix_time(),
            }),
        };
        *host.job.lock().unwrap() = Some(job);
        let gcode = args
            .get("gcode")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let estimates =
            args.get("op_estimates")
                .and_then(Value::as_array)
                .map(|items| {
                    items.iter().filter_map(Value::as_f64).collect::<Vec<_>>()
                });
        let session = host.session.clone();
        let job_host = host.clone();
        let daemon = self.clone();
        let task = tokio::spawn(async move {
            let result = session
                .run(
                    gcode,
                    if op_map.is_empty() {
                        None
                    } else {
                        Some(op_map)
                    },
                    estimates,
                )
                .await;
            if let Some(job) = job_host.job.lock().unwrap().as_mut() {
                job.data["running"] = json!(false);
            }
            if let Err(err) = result {
                daemon.fan_out(
                    &job_host,
                    WireEvent::JobError {
                        message: err.to_string(),
                    },
                );
            }
        });
        *host.job_task.lock().unwrap() = Some(task);
        Ok(json!({"job": handle}))
    }
}

/// A sink that forwards events into the daemon's per-host channel.
struct ChannelSink {
    tx: mpsc::UnboundedSender<WireEvent>,
}

impl RemoteEventSink for ChannelSink {
    fn emit(&self, event: WireEvent) {
        let _ = self.tx.send(event);
    }
}

async fn writer_loop<S>(
    daemon: Arc<Daemon>,
    client: Arc<Client>,
    mut writer: WriteHalf<S>,
) where
    S: AsyncRead + AsyncWrite + Send + Unpin,
{
    loop {
        tokio::select! {
            _ = client.shutdown.notified() => {
                let _ = tokio::io::AsyncWriteExt::shutdown(&mut writer).await;
                return;
            }
            frame = client.queue.get() => {
                if let Err(err) =
                    protocol::write_frame(&mut writer, &frame).await
                {
                    log_crate::debug!("client writer stopped: {err}");
                    daemon.disconnect(client).await;
                    return;
                }
            }
        }
    }
}

async fn keepalive_loop(daemon: Arc<Daemon>, client: Arc<Client>) {
    let (interval, misses) = {
        let cfg = daemon.config.lock().unwrap();
        (cfg.keepalive_interval, cfg.keepalive_misses)
    };
    if interval <= 0.0 {
        return;
    }
    let deadline = Duration::from_secs_f64(interval * f64::from(misses));
    loop {
        tokio::time::sleep(Duration::from_secs_f64(interval)).await;
        if client.last_rx.lock().unwrap().elapsed() > deadline {
            daemon.disconnect(client).await;
            return;
        }
        if !client.send(json!({"type": protocol::PING})) {
            daemon.disconnect(client).await;
            return;
        }
    }
}

fn require_control(
    host: &Arc<Host>,
    client: &Arc<Client>,
) -> Result<(), ProtocolError> {
    let roles = client.roles.lock().unwrap();
    if roles.get(&host.id).map(String::as_str) != Some("control") {
        return Err(ProtocolError::new(
            protocol::LEASE_HELD,
            "command requires the control lease",
        ));
    }
    Ok(())
}

/// Constant-time token comparison. Hashing first gives fixed-length
/// inputs, so neither contents nor length influence the comparison.
fn tokens_equal(supplied: &str, expected: &str) -> bool {
    use sha1::{Digest, Sha1};
    let supplied = Sha1::digest(supplied.as_bytes());
    let expected = Sha1::digest(expected.as_bytes());
    let mut difference = 0u8;
    for (a, b) in supplied.iter().zip(expected.iter()) {
        difference |= a ^ b;
    }
    difference == 0
}

fn internal(err: DriverError) -> ProtocolError {
    let kind = match err {
        DriverError::Timeout(_) => "timeout",
        DriverError::Connection(_) => "connection",
        DriverError::Device(_) | DriverError::BufferStall(_) => "device",
        DriverError::ResourceBusy { .. } => "connection",
        DriverError::Unsupported(_) => "unsupported",
        DriverError::Other(_) => "internal",
    };
    ProtocolError::new(protocol::INTERNAL, err.to_string()).with_kind(kind)
}

fn device_ref(config: &Value) -> String {
    config
        .get("port")
        .or_else(|| config.get("host"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn unix_time() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

fn serial_ports() -> Value {
    match serialport::available_ports() {
        Ok(ports) => Value::Array(
            ports
                .into_iter()
                .map(|info| {
                    let usb = match &info.port_type {
                        serialport::SerialPortType::UsbPort(usb) => Some(usb),
                        _ => None,
                    };
                    json!({
                        "port": info.port_name,
                        "vid": usb.map(|usb| usb.vid),
                        "pid": usb.map(|usb| usb.pid),
                        "serial_number": usb.and_then(|usb| usb.serial_number.clone()),
                        "manufacturer": usb.and_then(|usb| usb.manufacturer.clone()),
                        "product": usb.and_then(|usb| usb.product.clone()),
                    })
                })
                .collect(),
        ),
        Err(_) => json!([]),
    }
}

fn tls_acceptor(cert: &str, key: &str) -> Result<TlsAcceptor, DaemonError> {
    use rustls::pki_types::pem::PemObject;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};

    let certs = CertificateDer::pem_file_iter(cert)
        .map_err(|err| DaemonError::Tls(err.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| DaemonError::Tls(err.to_string()))?;
    let key = PrivateKeyDer::from_pem_file(key)
        .map_err(|err| DaemonError::Tls(err.to_string()))?;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|err| DaemonError::Tls(err.to_string()))?
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|err| DaemonError::Tls(err.to_string()))?;
    Ok(TlsAcceptor::from(Arc::new(config)))
}

/// The state of a hosted session, exposed for tests.
pub fn encode_state(state: &DeviceState) -> Value {
    protocol::encode_state(state)
}
