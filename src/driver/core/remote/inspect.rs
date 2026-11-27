//! Read-only inspection of a running daemon.
//!
//! Speaks the control protocol directly: welcome, optional auth,
//! then a batch of requests. Used by the debug CLI; never mutates
//! daemon state.

use std::time::Duration;

use serde_json::{json, Value};

use crate::driver::core::discovery::daemon::{
    connect_stream, is_loopback, read_message, write_frame,
};
use crate::driver::core::remote::protocol;

const RPC_TIMEOUT: Duration = Duration::from_secs(5);

/// Connect, authenticate and run *requests*.
///
/// Returns `(welcome_payload, replies)`. Errors when the endpoint is
/// not a reachable, authenticating daemon.
pub async fn daemon_rpc(
    host: &str,
    port: u16,
    token: Option<&str>,
    tls_ca: Option<&str>,
    tls_hostname: Option<&str>,
    requests: &[(&str, Value)],
) -> Result<(Value, Vec<Value>), std::io::Error> {
    if token.is_some() && tls_ca.is_none() && !is_loopback(host) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "refusing to send a token without TLS (use --tls-ca)",
        ));
    }
    let mut stream =
        connect_stream(host, port, tls_ca, tls_hostname, RPC_TIMEOUT).await?;
    let welcome = read_message(&mut stream, RPC_TIMEOUT).await?;
    if welcome.get("type").and_then(Value::as_str) != Some(protocol::WELCOME) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not a raydriver daemon",
        ));
    }
    let payload = welcome.get("payload").cloned().unwrap_or(Value::Null);
    if payload
        .get("auth_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let auth = protocol::request(
            0,
            protocol::AUTH,
            json!({"token": token.unwrap_or_default()}),
        );
        write_frame(&mut stream, &auth).await?;
        let reply = read_message(&mut stream, RPC_TIMEOUT).await?;
        if !reply.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "daemon authentication failed",
            ));
        }
    }

    let mut replies = Vec::with_capacity(requests.len());
    for (index, (type_, request_payload)) in requests.iter().enumerate() {
        let message =
            protocol::request(index as i64 + 1, type_, request_payload.clone());
        write_frame(&mut stream, &message).await?;
        replies.push(read_message(&mut stream, RPC_TIMEOUT).await?);
    }
    Ok((payload, replies))
}

/// Read just the welcome frame — no credentials needed.
pub async fn welcome_probe(
    host: &str,
    port: u16,
) -> Result<Value, std::io::Error> {
    let mut stream =
        connect_stream(host, port, None, None, RPC_TIMEOUT).await?;
    let welcome = read_message(&mut stream, RPC_TIMEOUT).await?;
    if welcome.get("type").and_then(Value::as_str) != Some(protocol::WELCOME) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not a raydriver daemon",
        ));
    }
    Ok(welcome.get("payload").cloned().unwrap_or(Value::Null))
}

/// Welcome plus sessions, clients and devices in one call.
pub async fn snapshot(
    host: &str,
    port: u16,
    token: Option<&str>,
    tls_ca: Option<&str>,
) -> Result<Value, std::io::Error> {
    let requests = [
        (protocol::SESSION_LIST, json!({})),
        (protocol::CLIENT_LIST, json!({})),
        (protocol::DEVICE_LIST, json!({})),
    ];
    let (welcome, replies) =
        daemon_rpc(host, port, token, tls_ca, None, &requests).await?;

    fn payload(reply: &Value, key: &str) -> Value {
        if !reply.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            return json!([]);
        }
        reply
            .get("payload")
            .and_then(|payload| payload.get(key))
            .cloned()
            .unwrap_or_else(|| json!([]))
    }

    Ok(json!({
        "welcome": welcome,
        "sessions": payload(&replies[0], "sessions"),
        "clients": payload(&replies[1], "clients"),
        "devices": payload(&replies[2], "ports"),
        "cameras": payload(&replies[2], "cameras"),
    }))
}
