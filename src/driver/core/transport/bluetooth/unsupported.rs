//! Stub backend for platforms without a Bluetooth SPP implementation.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tokio::sync::mpsc;

use super::{BluetoothDeviceInfo, Link};

pub(crate) fn connect(
    _address: &str,
    _channel: u8,
    _tx: mpsc::UnboundedSender<Vec<u8>>,
    _open: Arc<AtomicBool>,
) -> std::io::Result<Arc<dyn Link>> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Bluetooth SPP is not supported on this platform",
    ))
}

pub(crate) fn list_paired_devices() -> std::io::Result<Vec<BluetoothDeviceInfo>>
{
    Ok(Vec::new())
}
