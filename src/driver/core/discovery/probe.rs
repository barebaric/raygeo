//! Protocol-neutral TCP and HTTP confirmation probes.
//!
//! A probe connects to a host and confirms the declared firmware:
//! the TCP probe optionally sends a command and returns the first
//! response line starting with the declared prefix; the HTTP probe
//! GETs a path and matches the response body against a pattern.
//! They are used both by the subnet scanner (confirming reachable
//! listeners) and by mDNS fingerprinting (confirming generic service
//! announcements) before a driver claims a device.

use std::time::Duration;

use regex::Regex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Instant};

use crate::driver::core::http;

/// Longest line reported back to the caller.
const MAX_BANNER_LEN: usize = 80;
/// Maximum response bytes retained during a probe.
const MAX_RESPONSE_BYTES: usize = 4096;
/// Poll granularity while waiting for a response.
const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// One probe configuration.
pub struct TcpProbe<'a> {
    /// Maximum time spent establishing the connection.
    pub connect_timeout: Duration,
    /// Window for a banner the peer sends before any command.
    pub banner_wait: Duration,
    /// Window for the response to `command`.
    pub response_wait: Duration,
    /// Command sent to confirm the firmware; `None` only listens for
    /// an unsolicited banner.
    pub command: Option<&'a str>,
    /// Response-line prefix identifying the firmware.
    pub match_prefix: &'a str,
}

/// Runs *probe* against *host*:*port* and returns the matching
/// response line, or `None` when the peer does not answer like the
/// declared firmware.
pub async fn probe_tcp(
    host: &str,
    port: u16,
    probe: &TcpProbe<'_>,
) -> Option<String> {
    let mut stream =
        timeout(probe.connect_timeout, TcpStream::connect((host, port)))
            .await
            .ok()?
            .ok()?;

    if !probe.banner_wait.is_zero() {
        if let Some(line) =
            read_match(&mut stream, probe.match_prefix, probe.banner_wait).await
        {
            return Some(line);
        }
    }

    let command = probe.command?;
    stream.write_all(command.as_bytes()).await.ok()?;
    read_match(&mut stream, probe.match_prefix, probe.response_wait).await
}

/// Reads for *window* and returns the first line starting with
/// *prefix*.
async fn read_match(
    stream: &mut TcpStream,
    prefix: &str,
    window: Duration,
) -> Option<String> {
    if window.is_zero() {
        return None;
    }
    let deadline = Instant::now() + window;
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 256];
    while Instant::now() < deadline && buf.len() < MAX_RESPONSE_BYTES {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let poll = remaining.min(POLL_INTERVAL);
        match timeout(poll, stream.read(&mut chunk)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => {
                buf.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&buf);
                if let Some(line) = match_line(&text, prefix) {
                    return Some(line);
                }
            }
            _ => continue,
        }
    }
    None
}

/// Returns the first non-empty line starting with *prefix*, truncated
/// to the maximum banner length.
fn match_line(text: &str, prefix: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with(prefix) {
            return Some(truncate(line));
        }
    }
    None
}

/// Truncates *line* to at most `MAX_BANNER_LEN` characters.
fn truncate(line: &str) -> String {
    let end = line
        .char_indices()
        .map(|(i, _)| i)
        .nth(MAX_BANNER_LEN)
        .unwrap_or(line.len());
    line[..end].to_string()
}

/// One HTTP probe configuration.
pub struct HttpProbe<'a> {
    /// Request path, including any query string.
    pub path: &'a str,
    /// Case-insensitive pattern the response body must match.
    pub match_regex: &'a Regex,
    /// Maximum time for connect and exchange.
    pub timeout: Duration,
    /// Maximum body bytes retained for matching.
    pub max_bytes: usize,
}

/// Performs a GET against *host*:*port* and returns the matching
/// body excerpt, or `None` when the endpoint does not answer like
/// the declared firmware. Never fails: a transport error, a non-200
/// status and a non-matching body all yield `None`.
pub async fn probe_http(
    host: &str,
    port: u16,
    probe: &HttpProbe<'_>,
) -> Option<String> {
    let response = http::get(host, port, probe.path, probe.timeout)
        .await
        .ok()?;
    if response.status != 200 {
        return None;
    }
    let end = response.body.len().min(probe.max_bytes);
    let text = String::from_utf8_lossy(&response.body[..end]);
    probe
        .match_regex
        .find(&text)
        .map(|matched| matched.as_str().to_string())
}
