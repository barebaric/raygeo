//! Linux RFCOMM backend.
//!
//! RFCOMM sockets are plain AF_BLUETOOTH stream sockets, so this
//! module talks to them directly instead of depending on a BlueZ
//! library.  Enumeration uses BlueZ's D-Bus object manager through
//! zbus, which is pure Rust and keeps the wheel build free of libdbus
//! headers.

use std::collections::HashMap;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use tokio::sync::mpsc;

use super::{address_octets, BluetoothDeviceInfo, Link, SPP_UUID};

const BTPROTO_RFCOMM: libc::c_int = 3;
const DEVICE_INTERFACE: &str = "org.bluez.Device1";

#[repr(C)]
struct SockaddrRc {
    rc_family: libc::sa_family_t,
    rc_bdaddr: [u8; 6],
    rc_channel: u8,
}

struct LinuxLink {
    fd: Arc<OwnedFd>,
    writer: Mutex<()>,
    reader: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for LinuxLink {
    fn drop(&mut self) {
        self.close();
    }
}

impl Link for LinuxLink {
    fn write(&self, data: &[u8]) -> io::Result<()> {
        let _guard = self.writer.lock().unwrap();
        let mut written = 0;
        while written < data.len() {
            let remaining = &data[written..];
            let result = unsafe {
                // SAFETY: `fd` is a valid open socket for the
                // lifetime of the link (owned `OwnedFd`); the pointer
                // and length describe `remaining`.  MSG_NOSIGNAL
                // avoids SIGPIPE on a closed connection.
                libc::send(
                    self.fd.as_raw_fd(),
                    remaining.as_ptr().cast(),
                    remaining.len(),
                    libc::MSG_NOSIGNAL,
                )
            };
            if result < 0 {
                return Err(io::Error::last_os_error());
            }
            if result == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "Bluetooth socket accepted no data",
                ));
            }
            written += result as usize;
        }
        Ok(())
    }

    fn close(&self) {
        unsafe {
            // SAFETY: `fd` is a valid open socket; shutdown only
            // queues a disconnect, it does not invalidate the fd.
            libc::shutdown(self.fd.as_raw_fd(), libc::SHUT_RDWR);
        }
        if let Some(handle) = self.reader.lock().unwrap().take() {
            let _ = handle.join();
        }
    }
}

pub(crate) fn connect(
    address: &str,
    channel: u8,
    tx: mpsc::UnboundedSender<Vec<u8>>,
    open: Arc<AtomicBool>,
) -> io::Result<Arc<dyn Link>> {
    let octets = address_octets(address)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    // SAFETY: plain syscall with constant arguments; ownership of
    // the returned descriptor is taken below only after the < 0
    // check.
    let raw_fd = unsafe {
        libc::socket(libc::AF_BLUETOOTH, libc::SOCK_STREAM, BTPROTO_RFCOMM)
    };
    if raw_fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `raw_fd` is a valid, freshly created socket we now own
    // exclusively; `OwnedFd` closes it on drop.
    let fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };

    let sockaddr = SockaddrRc {
        rc_family: libc::AF_BLUETOOTH as libc::sa_family_t,
        rc_bdaddr: octets,
        rc_channel: channel,
    };
    let result = unsafe {
        // SAFETY: `fd` is valid; `sockaddr` is a `#[repr(C)]` struct
        // laid out exactly like `sockaddr_rc` (family, 6-byte
        // address, channel) and outlives the call.
        libc::connect(
            fd.as_raw_fd(),
            (&sockaddr as *const SockaddrRc).cast(),
            std::mem::size_of::<SockaddrRc>() as libc::socklen_t,
        )
    };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }

    let fd = Arc::new(fd);
    let reader_fd = fd.clone();
    let reader_open = open.clone();
    let handle = std::thread::Builder::new()
        .name("raydriver-rfcomm-reader".to_string())
        .spawn(move || {
            reader_loop(reader_fd, tx, reader_open);
        })?;

    open.store(true, Ordering::SeqCst);
    Ok(Arc::new(LinuxLink {
        fd,
        writer: Mutex::new(()),
        reader: Mutex::new(Some(handle)),
    }))
}

fn reader_loop(
    fd: Arc<OwnedFd>,
    tx: mpsc::UnboundedSender<Vec<u8>>,
    open: Arc<AtomicBool>,
) {
    super::rfcomm_reader_loop(
        |buf| unsafe {
            // SAFETY: the fd is valid for the whole reader thread
            // (the link keeps the `OwnedFd` alive and `close`
            // shuts the socket down before joining this thread);
            // the pointer/len describe `buf`, which is never
            // aliased.
            libc::recv(fd.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len(), 0)
        },
        &tx,
        &open,
    );
}

pub(crate) fn list_paired_devices() -> io::Result<Vec<BluetoothDeviceInfo>> {
    let handle = std::thread::Builder::new()
        .name("raydriver-bluetooth-scan".to_string())
        .spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| io::Error::other(e.to_string()))?;
            runtime
                .block_on(list_paired_devices_async())
                .map_err(|e| io::Error::other(e.to_string()))
        })?;
    handle
        .join()
        .map_err(|_| io::Error::other("Bluetooth scan thread panicked"))?
}

async fn list_paired_devices_async() -> zbus::Result<Vec<BluetoothDeviceInfo>> {
    let connection = zbus::Connection::system().await?;
    let reply = connection
        .call_method(
            Some("org.bluez"),
            "/",
            Some("org.bluez.ObjectManager"),
            "GetManagedObjects",
            &(),
        )
        .await?;
    let objects: HashMap<
        String,
        HashMap<String, HashMap<String, zbus::OwnedVariant>>,
    > = reply.body().deserialize()?;

    let mut devices = Vec::new();
    for interfaces in objects.values() {
        let Some(properties) = interfaces.get(DEVICE_INTERFACE) else {
            continue;
        };
        let paired = properties
            .get("Paired")
            .and_then(|value| bool::try_from(value).ok())
            .unwrap_or(false);
        if !paired {
            continue;
        }
        let uuids: Vec<String> = properties
            .get("UUIDs")
            .and_then(|value| value.try_clone().ok())
            .and_then(|value| Vec::<String>::try_from(value).ok())
            .unwrap_or_default();
        let supports_spp =
            uuids.iter().any(|uuid| uuid.eq_ignore_ascii_case(SPP_UUID));
        if !supports_spp && !uuids.is_empty() {
            continue;
        }
        let address = properties
            .get("Address")
            .and_then(|value| <&str>::try_from(value).ok())
            .unwrap_or_default()
            .to_string();
        if address.is_empty() {
            continue;
        }
        let name = properties
            .get("Alias")
            .or_else(|| properties.get("Name"))
            .and_then(|value| <&str>::try_from(value).ok())
            .map(str::to_string);
        devices.push(BluetoothDeviceInfo {
            address,
            name,
            channel: None,
        });
    }
    devices.sort_by(|a, b| a.address.cmp(&b.address));
    Ok(devices)
}
