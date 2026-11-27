//! Device and camera listing from raydriver daemons.
//!
//! A daemon is an endpoint, not a firmware: the probe speaks the
//! control protocol's welcome and lists the daemon's devices and
//! cameras. The wire format is a 4-byte big-endian payload length
//! followed by one UTF-8 JSON object, so the probe needs no firmware
//! knowledge.
//!
//! Discovery probes run unauthenticated and therefore only enumerate
//! daemons that expose an open listener. [`query_cameras`] additionally
//! speaks the token handshake and TLS so a client can list the cameras
//! of a protected daemon after the user supplies credentials.

use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::RootCertStore;
use serde_json::{json, Value};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

use crate::driver::core::remote::protocol;

/// Why a daemon could not be enumerated, for the daemon-level row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaemonNote {
    Unreachable,
    TokenRequired,
    NoDevices,
}

/// One entry of a daemon probe: a hosted device, or a daemon-level
/// row when the endpoint cannot be enumerated.
#[derive(Clone, Debug)]
pub enum DaemonEntry {
    Device {
        /// Device resource reference on the daemon.
        device: String,
        /// Firmware protocol the device speaks (info only).
        protocol: String,
        /// Display label announced by the daemon, when any.
        label: Option<String>,
    },
    Row {
        note: DaemonNote,
    },
}

/// Everything one daemon probe yielded.
#[derive(Clone, Debug)]
pub struct DaemonReport {
    pub host: String,
    pub port: u16,
    /// mDNS instance name of the daemon, for the identity banner.
    pub name: String,
    pub entries: Vec<DaemonEntry>,
}

/// A camera endpoint advertised by a daemon.
///
/// Video never crosses the control protocol: *uri* is opened by the
/// client's own network-camera source, and *source_type* names the
/// source kind (e.g. ``http_stream``).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaemonCamera {
    pub name: String,
    pub uri: String,
    pub source_type: String,
}

/// Connects to a daemon, reads its welcome and lists its devices.
/// Never raises: an unreachable or protected endpoint yields a
/// daemon row (or nothing when the peer does not speak the protocol).
pub async fn query_daemon(
    host: &str,
    port: u16,
    name: &str,
    timeout: Duration,
) -> DaemonReport {
    let entries = match query_inner(host, port, timeout).await {
        Ok(entries) => entries,
        Err(_) => vec![DaemonEntry::Row {
            note: DaemonNote::Unreachable,
        }],
    };
    DaemonReport {
        host: host.to_string(),
        port,
        name: name.to_string(),
        entries,
    }
}

/// Lists the cameras a daemon advertises.
///
/// Unlike discovery, this can authenticate against a protected
/// daemon: a token is sent only over TLS, except to loopback where
/// the token never leaves the machine. TLS is enabled when *tls_ca*
/// names a PEM certificate; *tls_hostname* overrides the name
/// verified against that certificate when it differs from *host*.
pub async fn query_cameras(
    host: &str,
    port: u16,
    token: Option<&str>,
    tls_ca: Option<&str>,
    tls_hostname: Option<&str>,
    timeout: Duration,
) -> Result<Vec<DaemonCamera>, std::io::Error> {
    if token.is_some() && tls_ca.is_none() && !is_loopback(host) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "refusing to send a token without TLS",
        ));
    }
    let mut stream =
        connect_stream(host, port, tls_ca, tls_hostname, timeout).await?;
    let welcome = read_message(&mut stream, timeout).await?;
    if welcome.get("type").and_then(Value::as_str) != Some("welcome") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "daemon did not send welcome",
        ));
    }
    let payload = welcome.get("payload").unwrap_or(&Value::Null);
    if payload.get("protocol").and_then(Value::as_i64)
        != Some(protocol::PROTOCOL_VERSION)
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unsupported daemon protocol version",
        ));
    }
    let auth_required = payload
        .get("auth_required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if auth_required {
        let Some(token) = token else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "daemon requires authentication",
            ));
        };
        let auth = json!({
            "id": 0,
            "type": "auth",
            "payload": {"token": token},
        });
        write_frame(&mut stream, &auth).await?;
        let reply = read_message(&mut stream, timeout).await?;
        if !reply.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "daemon authentication failed",
            ));
        }
    }

    let request = json!({"id": 1, "type": "device/list", "payload": {}});
    write_frame(&mut stream, &request).await?;
    let reply = read_message(&mut stream, timeout).await?;
    if !reply.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        let message = reply
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("device/list failed");
        return Err(std::io::Error::other(message.to_string()));
    }
    Ok(parse_cameras(
        reply
            .get("payload")
            .and_then(|payload| payload.get("cameras")),
    ))
}

fn parse_cameras(value: Option<&Value>) -> Vec<DaemonCamera> {
    let Some(entries) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut cameras = Vec::new();
    for entry in entries {
        let Some(uri) = entry.get("uri").and_then(Value::as_str) else {
            continue;
        };
        let Some(source_type) = entry.get("type").and_then(Value::as_str)
        else {
            continue;
        };
        let uri = uri.trim();
        let source_type = source_type.trim();
        if uri.is_empty() || source_type.is_empty() {
            continue;
        }
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(uri);
        cameras.push(DaemonCamera {
            name: name.to_string(),
            uri: uri.to_string(),
            source_type: source_type.to_string(),
        });
    }
    cameras
}

async fn query_inner(
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<Vec<DaemonEntry>, ()> {
    let mut stream =
        tokio::time::timeout(timeout, TcpStream::connect((host, port)))
            .await
            .map_err(|_| ())?
            .map_err(|_| ())?;

    let welcome = read_message(&mut stream, timeout).await.map_err(|_| ())?;
    if welcome.get("type").and_then(Value::as_str) != Some("welcome") {
        return Ok(Vec::new());
    }
    if welcome
        .get("payload")
        .and_then(|p| p.get("auth_required"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(vec![DaemonEntry::Row {
            note: DaemonNote::TokenRequired,
        }]);
    }

    let request = json!({"id": 1, "type": "device/list", "payload": {}});
    write_frame(&mut stream, &request).await.map_err(|_| ())?;
    let reply = read_message(&mut stream, timeout).await.map_err(|_| ())?;
    if !reply.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return Ok(vec![DaemonEntry::Row {
            note: DaemonNote::TokenRequired,
        }]);
    }

    let mut entries = Vec::new();
    if let Some(devices) = reply
        .get("payload")
        .and_then(|p| p.get("devices"))
        .and_then(Value::as_array)
    {
        for device in devices {
            let Some(reference) = device.get("device").and_then(Value::as_str)
            else {
                continue;
            };
            if reference.is_empty() {
                continue;
            }
            entries.push(DaemonEntry::Device {
                device: reference.to_string(),
                protocol: device
                    .get("protocol")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                label: device
                    .get("label")
                    .and_then(Value::as_str)
                    .filter(|l| !l.is_empty())
                    .map(str::to_string),
            });
        }
    }
    if entries.is_empty() {
        entries.push(DaemonEntry::Row {
            note: DaemonNote::NoDevices,
        });
    }
    Ok(entries)
}

/// A control-protocol stream, plain TCP or TLS.
pub type BoxedStream = Box<dyn crate::driver::core::connector::ByteStream>;

pub async fn connect_stream(
    host: &str,
    port: u16,
    tls_ca: Option<&str>,
    tls_hostname: Option<&str>,
    timeout: Duration,
) -> Result<BoxedStream, std::io::Error> {
    let tcp = tokio::time::timeout(timeout, TcpStream::connect((host, port)))
        .await
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("connecting to {host}:{port} timed out"),
            )
        })?
        .map_err(|err| {
            std::io::Error::new(
                err.kind(),
                format!("cannot connect to {host}:{port}: {err}"),
            )
        })?;
    let Some(ca_path) = tls_ca else {
        return Ok(Box::new(tcp));
    };

    let roots = load_roots(ca_path)?;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("TLS configuration failed: {err}"),
            )
        })?
        .with_root_certificates(roots)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));
    let name = tls_hostname.unwrap_or(host);
    let server_name = ServerName::try_from(name.to_string()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid TLS server name {name:?}"),
        )
    })?;
    let stream =
        tokio::time::timeout(timeout, connector.connect(server_name, tcp))
            .await
            .map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "TLS handshake timed out",
                )
            })?
            .map_err(|err| {
                std::io::Error::new(
                    err.kind(),
                    format!("TLS handshake failed: {err}"),
                )
            })?;
    Ok(Box::new(stream))
}

fn load_roots(path: &str) -> Result<RootCertStore, std::io::Error> {
    let certificates = CertificateDer::pem_file_iter(path).map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("cannot read TLS CA {path}: {err}"),
        )
    })?;
    let mut roots = RootCertStore::empty();
    let mut added = 0;
    for certificate in certificates {
        let certificate = certificate.map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid TLS CA {path}: {err}"),
            )
        })?;
        roots.add(certificate).map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid TLS CA {path}: {err}"),
            )
        })?;
        added += 1;
    }
    if added == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("no certificates in TLS CA {path}"),
        ));
    }
    Ok(roots)
}

pub fn is_loopback(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    host.parse::<std::net::IpAddr>()
        .map(|address| address.is_loopback())
        .unwrap_or(false)
}

/// Reads one frame, skipping keepalive pings.
pub async fn read_message<S: AsyncRead + Unpin>(
    stream: &mut S,
    timeout: Duration,
) -> Result<Value, std::io::Error> {
    loop {
        let msg = read_frame(stream, timeout).await?;
        if msg.get("type").and_then(Value::as_str) == Some("ping") {
            continue;
        }
        return Ok(msg);
    }
}

pub async fn read_frame<S: AsyncRead + Unpin>(
    stream: &mut S,
    timeout: Duration,
) -> Result<Value, std::io::Error> {
    tokio::time::timeout(timeout, protocol::read_frame(stream))
        .await
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "timed out reading frame",
            )
        })?
        .map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                err.to_string(),
            )
        })
}

pub async fn write_frame<S: AsyncWrite + Unpin>(
    stream: &mut S,
    message: &Value,
) -> Result<(), std::io::Error> {
    protocol::write_frame(stream, message)
        .await
        .map_err(|err| std::io::Error::other(err.to_string()))
}
