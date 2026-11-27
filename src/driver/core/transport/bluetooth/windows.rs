//! Windows RFCOMM backend.
//!
//! Windows exposes Bluetooth Classic as Winsock sockets in the
//! AF_BTH address family.  Connecting with the SPP service UUID and a
//! zero RFCOMM port lets Winsock resolve the channel through SDP.

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;

use tokio::sync::mpsc;
use windows_sys::Win32::Devices::Bluetooth::{
    BluetoothFindDeviceClose, BluetoothFindFirstDevice,
    BluetoothFindNextDevice, BLUETOOTH_DEVICE_INFO,
    BLUETOOTH_DEVICE_SEARCH_PARAMS, BTHPROTO_RFCOMM, SOCKADDR_BTH,
};
use windows_sys::Win32::Foundation::ERROR_NO_MORE_ITEMS;
use windows_sys::Win32::Networking::WinSock::{
    closesocket, connect as winsock_connect, recv, send, setsockopt, shutdown,
    WSASocketW, WSAStartup, INVALID_SOCKET, SD_BOTH, SOCKADDR, SOCKET,
    SOCKET_ERROR, SOCK_STREAM, WSADATA,
};

use super::{address_octets, BluetoothDeviceInfo, Link};

const SPP_SERVICE_UUID: windows_sys::core::GUID = windows_sys::core::GUID {
    data1: 0x0000_1101,
    data2: 0x0000,
    data3: 0x1000,
    data4: [0x80, 0x00, 0x00, 0x80, 0x5f, 0x9b, 0x34, 0xfb],
};

struct SocketHandle(SOCKET);

impl Drop for SocketHandle {
    fn drop(&mut self) {
        unsafe {
            closesocket(self.0);
        }
    }
}

struct WindowsLink {
    socket: Arc<SocketHandle>,
    writer: Mutex<()>,
    reader: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for WindowsLink {
    fn drop(&mut self) {
        self.close();
    }
}

impl Link for WindowsLink {
    fn write(&self, data: &[u8]) -> io::Result<()> {
        let _guard = self.writer.lock().unwrap();
        let mut written = 0;
        while written < data.len() {
            let remaining = &data[written..];
            let result = unsafe {
                send(
                    self.socket.0,
                    remaining.as_ptr(),
                    remaining.len() as i32,
                    0,
                )
            };
            if result == SOCKET_ERROR {
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
            // SAFETY: `socket.0` is a valid open socket handle; the
            // `SocketHandle` Drop closes it only after all readers
            // and writers drop their `Arc`.
            shutdown(self.socket.0, SD_BOTH);
        }
        if let Some(handle) = self.reader.lock().unwrap().take() {
            let _ = handle.join();
        }
    }
}

fn winsock_error() -> io::Result<()> {
    static START: OnceLock<i32> = OnceLock::new();
    let code = *START.get_or_init(|| unsafe {
        // SAFETY: `WSADATA` is a plain-old-data out-parameter the
        // API initializes; zeroed is the documented initialization
        // pattern.  Once-initialized WinSock needs no cleanup for
        // the process lifetime.
        let mut data: WSADATA = std::mem::zeroed();
        WSAStartup(0x0202, &mut data)
    });
    if code == 0 {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(code))
    }
}

pub(crate) fn connect(
    address: &str,
    _channel: u8,
    tx: mpsc::UnboundedSender<Vec<u8>>,
    open: Arc<AtomicBool>,
) -> io::Result<Arc<dyn Link>> {
    winsock_error()?;
    let octets = address_octets(address)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let bt_addr = octets
        .iter()
        .fold(0u64, |value, octet| (value << 8) | u64::from(*octet));

    // SAFETY: constant arguments; a null protocol info and no
    // flags produce a plain Bluetooth RFCOMM socket whose validity
    // is checked against INVALID_SOCKET below.
    let socket = unsafe {
        WSASocketW(
            i32::from(windows_sys::Win32::Devices::Bluetooth::AF_BTH),
            SOCK_STREAM,
            BTHPROTO_RFCOMM as i32,
            std::ptr::null(),
            0,
            0,
        )
    };
    if socket == INVALID_SOCKET {
        return Err(io::Error::last_os_error());
    }
    let socket = Arc::new(SocketHandle(socket));

    let enable: i32 = 1;
    for option in [
        windows_sys::Win32::Devices::Bluetooth::SO_BTH_AUTHENTICATE,
        windows_sys::Win32::Devices::Bluetooth::SO_BTH_ENCRYPT,
    ] {
        unsafe {
            // SAFETY: `socket` is valid; the option value is a live
            // `i32` whose pointer/size describe it.
            setsockopt(
                socket.0,
                windows_sys::Win32::Devices::Bluetooth::SOL_RFCOMM as i32,
                option as i32,
                (&enable as *const i32).cast::<u8>(),
                std::mem::size_of::<i32>() as i32,
            );
        }
    }

    let sockaddr = SOCKADDR_BTH {
        addressFamily: windows_sys::Win32::Devices::Bluetooth::AF_BTH,
        btAddr: bt_addr,
        serviceClassId: SPP_SERVICE_UUID,
        port: 0,
    };
    let result = unsafe {
        // SAFETY: `socket` is valid; `sockaddr` is a `#[repr(C)]`
        // `SOCKADDR_BTH` (family, address, SPP service class, port)
        // cast to the generic `SOCKADDR` the API expects, and it
        // outlives the call.
        winsock_connect(
            socket.0,
            (&sockaddr as *const SOCKADDR_BTH).cast::<SOCKADDR>(),
            std::mem::size_of::<SOCKADDR_BTH>() as i32,
        )
    };
    if result == SOCKET_ERROR {
        return Err(io::Error::last_os_error());
    }

    let reader_socket = socket.clone();
    let reader_open = open.clone();
    let handle = std::thread::Builder::new()
        .name("raydriver-rfcomm-reader".to_string())
        .spawn(move || reader_loop(reader_socket, tx, reader_open))?;

    open.store(true, Ordering::SeqCst);
    Ok(Arc::new(WindowsLink {
        socket,
        writer: Mutex::new(()),
        reader: Mutex::new(Some(handle)),
    }))
}

fn reader_loop(
    socket: Arc<SocketHandle>,
    tx: mpsc::UnboundedSender<Vec<u8>>,
    open: Arc<AtomicBool>,
) {
    super::rfcomm_reader_loop(
        // WinSock `recv` returns `int` where POSIX `recv` returns
        // `ssize_t`; the cast sign-extends negatives intact.
        |buf| unsafe {
            // SAFETY: the socket handle is valid for the whole
            // reader thread (the link keeps it alive and `close`
            // shuts the socket down before joining); the pointer and
            // length describe `buf`, which is never aliased.
            recv(socket.0, buf.as_mut_ptr(), buf.len() as i32, 0) as isize
        },
        &tx,
        &open,
    );
}

pub(crate) fn list_paired_devices() -> io::Result<Vec<BluetoothDeviceInfo>> {
    let params = BLUETOOTH_DEVICE_SEARCH_PARAMS {
        dwSize: std::mem::size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32,
        fReturnAuthenticated: 1,
        fReturnRemembered: 1,
        fReturnConnected: 1,
        fReturnUnknown: 0,
        fIssueInquiry: 0,
        cTimeoutMultiplier: 0,
        hRadio: std::ptr::null_mut(),
    };
    let mut info = BLUETOOTH_DEVICE_INFO {
        dwSize: std::mem::size_of::<BLUETOOTH_DEVICE_INFO>() as u32,
        ..Default::default()
    };
    // SAFETY: both pointers are valid for the call: `params` is a
    // fully initialized search structure, `info` a `dwSize`-tagged
    // out record.
    let find = unsafe { BluetoothFindFirstDevice(&params, &mut info) };
    if find.is_null() {
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(ERROR_NO_MORE_ITEMS as i32) {
            return Ok(Vec::new());
        }
        return Err(err);
    }

    let mut devices = Vec::new();
    loop {
        if let Some(device) = info_to_device(&info) {
            devices.push(device);
        }
        // SAFETY: `find` is a live enumeration handle from
        // `BluetoothFindFirstDevice` (non-null, checked above);
        // `info` is the same record the API filled before.
        let has_next = unsafe { BluetoothFindNextDevice(find, &mut info) };
        if has_next == 0 {
            break;
        }
    }
    // SAFETY: `find` is the live enumeration handle; every exit
    // path from the loop above reaches this close.
    unsafe {
        BluetoothFindDeviceClose(find);
    }
    devices.sort_by(|a, b| a.address.cmp(&b.address));
    Ok(devices)
}

fn info_to_device(info: &BLUETOOTH_DEVICE_INFO) -> Option<BluetoothDeviceInfo> {
    // SAFETY: reading the `btAddr` union member the API just
    // initialized (`BTH_ADDR` is stored in the union's ullLong
    // variant).
    let raw = unsafe { info.Address.Anonymous.ullLong };
    if raw == 0 {
        return None;
    }
    let address = [
        (raw >> 40) as u8,
        (raw >> 32) as u8,
        (raw >> 24) as u8,
        (raw >> 16) as u8,
        (raw >> 8) as u8,
        raw as u8,
    ]
    .iter()
    .map(|octet| format!("{octet:02X}"))
    .collect::<Vec<_>>()
    .join(":");
    let name = utf16_name(&info.szName);
    Some(BluetoothDeviceInfo {
        address,
        name,
        channel: None,
    })
}

fn utf16_name(raw: &[u16]) -> Option<String> {
    let end = raw
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(raw.len());
    let name = String::from_utf16_lossy(&raw[..end]).trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}
