//! Serial port scanning.
//!
//! Every port is opened exactly once per scan and probed at a
//! sequence of baud rates; whatever the connected device sends
//! (welcome banner first, then responses to nudge characters) is
//! captured as a [`PortObservation`]. All ports are probed
//! concurrently and every phase is bounded by short timeouts so a
//! full scan stays fast.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::Mutex as AsyncMutex;

/// Baud rates tried in order. 115200 is by far the most common for
/// laser controllers, so it is probed across all ports first.
pub const BAUD_CANDIDATES: [u32; 3] = [115200, 57600, 9600];

/// How long to listen for an unsolicited welcome banner before
/// nudging.
const BANNER_TIMEOUT: Duration = Duration::from_secs(2);
/// How long to wait for a response after sending a nudge character.
const NUDGE_TIMEOUT: Duration = Duration::from_secs(1);
/// Once a device is talking, how long the line must stay quiet
/// before its output burst is considered complete.
const QUIET_GAP: Duration = Duration::from_millis(300);
/// Upper bound for the final drain of trailing burst data.
const DRAIN_TIMEOUT: Duration = Duration::from_secs(1);
const READ_CHUNK: Duration = Duration::from_millis(200);

const NUDGES: [&[u8]; 2] = [b"\n", b"?"];

/// Raw output captured from one port during a scan.
pub struct PortObservation {
    pub port: String,
    pub baud_rate: u32,
    /// Everything the device sent while the port was open.
    pub data: Vec<u8>,
}

/// Generic, firmware-agnostic test for "a device is talking on this
/// port": the capture contains at least one line of mostly-printable
/// ASCII. Output received at a wrong baud rate is typically framing
/// garbage and fails this test.
pub fn looks_like_device_output(data: &[u8]) -> bool {
    for line in data.split(|&b| b == b'\n') {
        let line: Vec<u8> = line
            .iter()
            .copied()
            .filter(|&b| !matches!(b, b' ' | b'\t' | b'\r' | 0))
            .collect();
        if line.is_empty() {
            continue;
        }
        let printable = line
            .iter()
            .filter(|&&b| (0x20..=0x7E).contains(&b) || b == 0x09)
            .count();
        if printable as f64 / line.len() as f64 >= 0.8 {
            return true;
        }
    }
    false
}

// Per-port locks so concurrent scans don't fight over the same
// physical port (serial ports open exclusively).
static PORT_LOCKS: LazyLock<Mutex<HashMap<String, Arc<AsyncMutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn port_lock(port: &str) -> Arc<AsyncMutex<()>> {
    PORT_LOCKS
        .lock()
        .unwrap()
        .entry(port.to_string())
        .or_default()
        .clone()
}

/// Probes every *port* at *baud_rates*, all ports in parallel, and
/// returns one observation per port that produced device-like
/// output. Ports that are busy, absent or silent are skipped
/// silently.
///
/// When *timeout* is given the scan stops waiting after that long:
/// observations collected so far are still returned and probes still
/// in flight are aborted, so a slow or silent adapter can never push
/// a responsive machine past the deadline.
pub async fn scan_serial_ports(
    ports: Vec<String>,
    baud_rates: Vec<u32>,
    timeout: Option<Duration>,
) -> Vec<PortObservation> {
    let rates = if baud_rates.is_empty() {
        BAUD_CANDIDATES.to_vec()
    } else {
        baud_rates
    };
    let mut handles = Vec::new();
    for port in ports {
        let rates = rates.clone();
        handles.push(tokio::spawn(async move {
            let lock = port_lock(&port);
            let _guard = lock.lock().await;
            let (baud_rate, data) = probe_port(&port, &rates).await?;
            Some(PortObservation {
                port,
                baud_rate,
                data,
            })
        }));
    }

    let deadline = timeout.map(|t| tokio::time::Instant::now() + t);
    let mut out = Vec::new();
    for handle in &mut handles {
        let remaining = deadline
            .map(|d| d.saturating_duration_since(tokio::time::Instant::now()));
        let result = match remaining {
            Some(r) => match tokio::time::timeout(r, &mut *handle).await {
                Ok(joined) => joined,
                Err(_) => {
                    handle.abort();
                    continue;
                }
            },
            None => handle.await,
        };
        if let Ok(Some(observation)) = result {
            out.push(observation);
        }
    }
    out
}

/// Attempts to get a device on *port* to talk. Tries *rates* in
/// order and returns the first rate at which the device produces
/// plausible output, with everything it sent.
async fn probe_port(port: &str, rates: &[u32]) -> Option<(u32, Vec<u8>)> {
    for &baudrate in rates {
        let port = port.to_string();
        let result = tokio::task::spawn_blocking(move || {
            probe_port_blocking(&port, baudrate)
        })
        .await
        .ok()
        .flatten();
        if let Some(data) = result {
            return Some((baudrate, data));
        }
    }
    None
}

/// Synchronous probe of a single port/baud combination. Opens the
/// port, listens for a banner, nudges a silent device, and returns
/// everything it sent if the output looks like a device.
///
/// Recognition happens later, on the captured bytes, so the probe
/// always collects as much signal as it can: every nudge is sent
/// even after the device starts talking (different firmwares
/// identify themselves in different responses), and a trailing
/// drain catches the rest of a multi-line burst.
fn probe_port_blocking(port: &str, baudrate: u32) -> Option<Vec<u8>> {
    let mut ser = serialport::new(port, baudrate)
        .timeout(READ_CHUNK)
        .open()
        .ok()?;
    let mut buf: Vec<u8> = Vec::new();
    let mut saw_output = listen(&mut ser, &mut buf, BANNER_TIMEOUT);

    for nudge in NUDGES {
        ser.write_all(nudge).ok()?;
        ser.flush().ok()?;
        saw_output |= listen(&mut ser, &mut buf, NUDGE_TIMEOUT);
    }

    if !saw_output || !looks_like_device_output(&buf) {
        return None;
    }
    drain(&mut ser, &mut buf);
    Some(buf)
}

/// Reads from *ser* into *buf* for up to *timeout*, returning true
/// as soon as the data captured during this call looks like device
/// output. Only new data is tested: plausible output from earlier
/// phases must not cut short later ones.
fn listen(ser: &mut impl Read, buf: &mut Vec<u8>, timeout: Duration) -> bool {
    let start = buf.len();
    let deadline = Instant::now() + timeout;
    let mut chunk = [0u8; 256];
    while Instant::now() < deadline {
        let n = match ser.read(&mut chunk) {
            Ok(n) => n,
            Err(_) => return false,
        };
        if n == 0 {
            continue;
        }
        buf.extend_from_slice(&chunk[..n]);
        if looks_like_device_output(&buf[start..]) {
            return true;
        }
    }
    looks_like_device_output(&buf[start..])
}

/// Collects trailing burst data until the line stays quiet.
fn drain(ser: &mut impl Read, buf: &mut Vec<u8>) {
    let deadline = Instant::now() + DRAIN_TIMEOUT;
    let mut last = Instant::now();
    let mut chunk = [0u8; 256];
    while Instant::now() < deadline {
        let n = match ser.read(&mut chunk) {
            Ok(n) => n,
            Err(_) => return,
        };
        let now = Instant::now();
        if n > 0 {
            buf.extend_from_slice(&chunk[..n]);
        } else if now.duration_since(last) >= QUIET_GAP {
            return;
        }
        last = now;
    }
}
