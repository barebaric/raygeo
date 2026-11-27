//! ESP3D / FluidNC transport.
//!
//! Commands are HTTP GET requests against the firmware's command
//! endpoint; the response body is ignored because every reply (status
//! reports, `ok`/`error:` acks, `$` replies) arrives on the WebSocket
//! channel.  Jobs are uploaded as one file and started with the
//! `[ESP220]` execute command instead of being streamed line by line.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::driver::core::driver::DriverError;
use crate::driver::core::http;
use crate::driver::core::transport::{
    NetworkJobRunner, Transport, TransportError,
};
use crate::driver::core::ws::WsClient;

/// Firmware flavour behind the same web API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolVariant {
    Esp3d,
    Longer,
    FluidNc,
}

impl ProtocolVariant {
    pub fn from_name(name: &str) -> Self {
        match name.to_ascii_lowercase().as_str() {
            "fluidnc" => Self::FluidNc,
            "longer" => Self::Longer,
            _ => Self::Esp3d,
        }
    }

    fn command_path(&self, encoded: &str) -> String {
        match self {
            Self::FluidNc => format!("/?commandText={encoded}"),
            _ => format!("/command?commandText={encoded}&PAGEID="),
        }
    }
}

/// ESP3D connection settings.
#[derive(Clone)]
pub struct Esp3dConfig {
    pub host: String,
    pub http_port: u16,
    pub ws_port: u16,
    pub protocol: ProtocolVariant,
    pub request_timeout: f64,
    pub connect_timeout: f64,
}

impl Esp3dConfig {
    pub fn new(host: &str) -> Self {
        Self {
            host: host.to_string(),
            http_port: 80,
            ws_port: 81,
            protocol: ProtocolVariant::Esp3d,
            request_timeout: 10.0,
            connect_timeout: 6.0,
        }
    }

    fn secs(value: f64) -> Duration {
        Duration::from_secs_f64(value.max(0.0))
    }

    fn base(&self) -> String {
        format!("http://{}:{}", self.host, self.http_port)
    }
}

impl Default for Esp3dConfig {
    fn default() -> Self {
        Self::new("127.0.0.1")
    }
}

/// The ESP3D transport: HTTP writes plus a WebSocket receive task.
pub struct Esp3dTransport {
    config: Esp3dConfig,
    is_open: AtomicBool,
    ws_task: Mutex<Option<JoinHandle<()>>>,
}

impl Esp3dTransport {
    pub fn new(config: Esp3dConfig) -> Arc<Self> {
        Arc::new(Self {
            config,
            is_open: AtomicBool::new(false),
            ws_task: Mutex::new(None),
        })
    }

    async fn get(&self, path: &str) -> Result<http::HttpResponse, DriverError> {
        http::get(
            &self.config.host,
            self.config.http_port,
            path,
            Esp3dConfig::secs(self.config.request_timeout),
        )
        .await
    }

    async fn require_ok(&self, path: &str) -> Result<(), DriverError> {
        let response = self.get(path).await?;
        if !(200..300).contains(&response.status) {
            return Err(DriverError::Connection(format!(
                "unexpected HTTP status {} for {path}",
                response.status
            )));
        }
        Ok(())
    }

    async fn handshake(&self) -> Result<(), DriverError> {
        match self.config.protocol {
            ProtocolVariant::FluidNc => {
                self.require_ok("/api/status").await?;
            }
            _ => {
                self.require_ok("/command?plain=%5BESP420%5D&PAGEID=")
                    .await?;
                self.require_ok("/command?plain=%5BESP800%5D&PAGEID=")
                    .await?;
                self.require_ok("/command?plain=%5BESP400%5D&PAGEID=")
                    .await?;
            }
        }
        Ok(())
    }

    fn command_path(&self, command: &str) -> String {
        if command == "?" && self.config.protocol != ProtocolVariant::FluidNc {
            return "/command?commandText=?&PAGEID=".to_string();
        }
        self.config
            .protocol
            .command_path(&url_encode(command.as_bytes()))
    }

    fn upload_body(&self, gcode: &str) -> (String, Vec<u8>) {
        let boundary = crate::driver::core::http::multipart_boundary();
        let mut body = Vec::with_capacity(gcode.len() + 512);
        if self.config.protocol != ProtocolVariant::Esp3d {
            push_field(&mut body, &boundary, "path", b"/", false);
            push_field(
                &mut body,
                &boundary,
                "size",
                gcode.len().to_string().as_bytes(),
                false,
            );
            push_field(&mut body, &boundary, "file", gcode.as_bytes(), true);
        } else {
            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            body.extend_from_slice(
                b"Content-Disposition: form-data; name=\"file\"; \
                  filename=\"rayforge.gcode\"\r\n\
                  Content-Type: text/plain\r\n\r\n",
            );
            body.extend_from_slice(gcode.as_bytes());
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        (format!("multipart/form-data; boundary={boundary}"), body)
    }

    async fn upload(&self, gcode: &str) -> Result<(), DriverError> {
        let (content_type, body) = self.upload_body(gcode);
        let response = http::post(
            &self.config.host,
            self.config.http_port,
            "/upload?path=/",
            &content_type,
            &body,
            Esp3dConfig::secs(self.config.request_timeout),
        )
        .await?;
        if !(200..300).contains(&response.status) {
            return Err(DriverError::Connection(format!(
                "unexpected HTTP status {} for upload",
                response.status
            )));
        }
        Ok(())
    }
}

#[async_trait]
impl Transport for Esp3dTransport {
    async fn open(
        &self,
        tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Result<(), TransportError> {
        self.close().await;
        self.handshake()
            .await
            .map_err(|err| TransportError::Connection(err.to_string()))?;

        let origin = self.config.base();
        let ws = WsClient::connect(
            &self.config.host,
            self.config.ws_port,
            Some(&origin),
            Esp3dConfig::secs(self.config.connect_timeout),
        )
        .await
        .map_err(|err| TransportError::Connection(err.to_string()))?;

        let task = tokio::spawn(async move {
            let mut ws = ws;
            loop {
                match ws.receive().await {
                    Ok(message) => {
                        let mut data = message.into_bytes();
                        data.push(b'\n');
                        if tx.send(data).is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        log_crate::warn!(
                            "ESP3D WebSocket channel failed: {err}"
                        );
                        break;
                    }
                }
            }
        });
        *self.ws_task.lock().unwrap() = Some(task);
        self.is_open.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn close(&self) {
        self.is_open.store(false, Ordering::SeqCst);
        let task = self.ws_task.lock().unwrap().take();
        if let Some(task) = task {
            // Dropping the aborted task closes the WebSocket halves;
            // waiting for it can block on Windows if the peer has not
            // acknowledged the close yet.
            task.abort();
        }
    }

    async fn write(&self, data: &[u8]) -> Result<(), TransportError> {
        if !self.is_open.load(Ordering::SeqCst) {
            return Err(TransportError::NotConnected);
        }
        let Ok(text) = std::str::from_utf8(data) else {
            return Err(TransportError::Connection(
                "ESP3D commands must be UTF-8".to_string(),
            ));
        };
        let command = text.trim_end_matches(['\r', '\n']);
        if command.is_empty() {
            return Ok(());
        }
        let path = self.command_path(command);
        self.require_ok(&path)
            .await
            .map_err(|err| TransportError::Connection(err.to_string()))
    }

    fn is_open(&self) -> bool {
        self.is_open.load(Ordering::SeqCst)
    }

    fn resource_uri(&self) -> Option<String> {
        Some(format!(
            "tcp://{}:{}",
            self.config.host, self.config.http_port
        ))
    }

    fn network_job_runner(&self) -> Option<&dyn NetworkJobRunner> {
        Some(self)
    }
}

#[async_trait]
impl NetworkJobRunner for Esp3dTransport {
    async fn run_network_job(&self, gcode: &str) -> Result<(), TransportError> {
        self.upload(gcode)
            .await
            .map_err(|err| TransportError::Connection(err.to_string()))?;
        self.require_ok("/command?commandText=%5BESP220%5D/rayforge.gcode")
            .await
            .map_err(|err| TransportError::Connection(err.to_string()))
    }
}

fn push_field(
    body: &mut Vec<u8>,
    boundary: &str,
    name: &str,
    value: &[u8],
    is_file: bool,
) {
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    if is_file {
        body.extend_from_slice(
            b"Content-Disposition: form-data; name=\"file\"; \
              filename=\"rayforge.gcode\"\r\n\
              Content-Type: application/octet-stream\r\n\r\n",
        );
    } else {
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n")
                .as_bytes(),
        );
    }
    body.extend_from_slice(value);
    body.extend_from_slice(b"\r\n");
}

fn url_encode(data: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(data.len() * 3);
    for byte in data {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'.'
            | b'_'
            | b'~' => out.push(char::from(*byte)),
            _ => {
                out.push('%');
                out.push(HEX[(byte >> 4) as usize] as char);
                out.push(HEX[(byte & 0x0F) as usize] as char);
            }
        }
    }
    out
}
