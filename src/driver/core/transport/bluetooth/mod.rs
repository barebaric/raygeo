//! Bluetooth Classic SPP/RFCOMM transport.
//!
//! The platform backends perform blocking socket or framework I/O on a
//! dedicated thread; this module only bridges that thread to the async
//! [`Transport`] surface.  Keeping the platform work off the tokio
//! workers matters most on macOS, where IOBluetooth delivers RFCOMM
//! callbacks through a run loop.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use super::{Transport, TransportError};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "windows"
)))]
mod unsupported;

#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_os = "windows")]
use windows as platform;

#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "windows"
)))]
use unsupported as platform;

/// Runs a blocking platform call off the async workers.
///
/// On native targets this is `spawn_blocking`; on wasm there is no
/// blocking thread pool (spawning one panics), so the call runs
/// inline — the only backend reachable there is `unsupported`, whose
/// calls return errors immediately without blocking.  The `Err`
/// variant reports the task-level failure (a panicking task on
/// native); call sites attach their own message prefix.
#[cfg(not(target_family = "wasm"))]
async fn run_platform<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(target_family = "wasm")]
async fn run_platform<T, F>(f: F) -> Result<T, String>
where
    F: FnOnce() -> T,
{
    Ok(f())
}

/// The Bluetooth SIG Serial Port Profile service UUID.
pub const SPP_UUID: &str = "00001101-0000-1000-8000-00805f9b34fb";

/// A paired device that can be offered for a Bluetooth SPP connection.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BluetoothDeviceInfo {
    pub address: String,
    pub name: Option<String>,
    /// The SDP-resolved RFCOMM channel, when the platform exposes it.
    pub channel: Option<u8>,
}

/// Normalizes a Bluetooth address to the canonical uppercase,
/// colon-separated form.  Both colon- and dash-separated input is
/// accepted because macOS reports dashes.
pub fn normalize_address(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("Bluetooth address must not be empty".to_string());
    }
    let separator = if value.contains(':') { ':' } else { '-' };
    let parts: Vec<&str> = value.split(separator).collect();
    if parts.len() != 6 {
        return Err(format!("invalid Bluetooth address '{value}'"));
    }
    let mut octets = Vec::with_capacity(6);
    for part in parts {
        if part.len() != 2 || !part.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("invalid Bluetooth address '{value}'"));
        }
        octets.push(part.to_ascii_uppercase());
    }
    Ok(octets.join(":"))
}

/// The six address octets in display order, for platform socket APIs.
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(crate) fn address_octets(address: &str) -> Result<[u8; 6], String> {
    let normalized = normalize_address(address)?;
    let mut octets = [0u8; 6];
    for (index, part) in normalized.split(':').enumerate() {
        octets[index] = u8::from_str_radix(part, 16)
            .map_err(|_| format!("invalid Bluetooth address '{address}'"))?;
    }
    Ok(octets)
}

/// The RFCOMM receive loop shared by the raw-socket platform
/// backends.
///
/// *read* performs one blocking socket read — the unsafe FFI stays
/// at the backend, this loop only owns the byte-chunking, error and
/// shutdown semantics.  Returns bytes read (> 0), 0 on EOF, negative
/// on error (mapped through `std::io::Error::last_os_error`).
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(crate) fn rfcomm_reader_loop(
    mut read: impl FnMut(&mut [u8]) -> isize,
    tx: &mpsc::UnboundedSender<Vec<u8>>,
    open: &AtomicBool,
) {
    let mut buf = [0u8; 1024];
    loop {
        let n = read(&mut buf);
        if n > 0 {
            if tx.send(buf[..n as usize].to_vec()).is_err() {
                break;
            }
        } else if n == 0 {
            break;
        } else {
            let err = std::io::Error::last_os_error();
            if err.kind() != std::io::ErrorKind::Interrupted {
                log_crate::debug!("Bluetooth RFCOMM read error: {err}");
                break;
            }
        }
    }
    open.store(false, Ordering::SeqCst);
}

/// A live RFCOMM link.  Platform implementations own any reader or
/// run-loop thread and stop it from [`Link::close`].
pub(crate) trait Link: Send + Sync {
    fn write(&self, data: &[u8]) -> std::io::Result<()>;
    fn close(&self);
}

pub struct BluetoothTransport {
    address: String,
    channel: u8,
    open: Arc<AtomicBool>,
    link: Mutex<Option<Arc<dyn Link>>>,
}

impl BluetoothTransport {
    pub fn new(address: &str, channel: u8) -> Arc<Self> {
        Arc::new(Self {
            address: normalize_address(address)
                .unwrap_or_else(|_| address.trim().to_ascii_uppercase()),
            channel: channel.clamp(1, 30),
            open: Arc::new(AtomicBool::new(false)),
            link: Mutex::new(None),
        })
    }
}

#[async_trait]
impl Transport for BluetoothTransport {
    async fn open(
        &self,
        tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Result<(), TransportError> {
        if self.is_open() {
            self.close().await;
        }
        let address = self.address.clone();
        let channel = self.channel;
        let open = self.open.clone();
        let link = run_platform(move || {
            platform::connect(&address, channel, tx, open)
        })
        .await
        .map_err(|e| {
            TransportError::Connection(format!(
                "Bluetooth connect task failed: {e}"
            ))
        })?
        .map_err(|e| TransportError::Connection(e.to_string()))?;
        *self.link.lock().await = Some(link);
        Ok(())
    }

    async fn close(&self) {
        self.open.store(false, Ordering::SeqCst);
        if let Some(link) = self.link.lock().await.take() {
            let _ = run_platform(move || link.close()).await;
        }
    }

    async fn write(&self, data: &[u8]) -> Result<(), TransportError> {
        let Some(link) = self.link.lock().await.clone() else {
            return Err(TransportError::NotConnected);
        };
        if !self.is_open() {
            return Err(TransportError::NotConnected);
        }
        let data = data.to_vec();
        match run_platform(move || link.write(&data)).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => {
                self.open.store(false, Ordering::SeqCst);
                Err(TransportError::Connection(e.to_string()))
            }
            Err(e) => {
                self.open.store(false, Ordering::SeqCst);
                Err(TransportError::Connection(format!(
                    "Bluetooth write task failed: {e}"
                )))
            }
        }
    }

    fn is_open(&self) -> bool {
        self.open.load(Ordering::SeqCst)
    }

    fn resource_uri(&self) -> Option<String> {
        Some(format!("bt://{}", self.address))
    }
}

/// Enumerate paired devices that advertise the Serial Port Profile.
///
/// This is a best-effort listing for device pickers.  Failures (no
/// adapter, no permission, no D-Bus in a sandbox) yield an empty list
/// rather than an error.
pub fn list_paired_devices() -> Vec<BluetoothDeviceInfo> {
    match platform::list_paired_devices() {
        Ok(devices) => devices,
        Err(err) => {
            log_crate::warn!("Failed to list paired Bluetooth devices: {err}");
            Vec::new()
        }
    }
}

/// Whether this build has a Bluetooth SPP backend.
pub fn is_supported() -> bool {
    cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    ))
}
