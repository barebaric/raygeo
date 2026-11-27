//! Serial port enumeration for discovery and device pickers.
//!
//! Port metadata comes from the platform serial enumeration; on
//! sandboxed Linux (Snap) a glob-based fallback plus
//! /dev/serial/by-id symlink descriptions is used instead, since
//! direct enumeration of ttyS* nodes is forbidden there.

use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::driver::core::transport::serial::list_serial_ports as platform_ports;

const USB_PORT_MARKERS: [&str; 3] = ["ttyUSB", "ttyACM", "rfcomm"];

/// A serial port path together with optional metadata used to
/// identify the underlying USB device.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SerialPortInfo {
    pub device: String,
    pub description: Option<String>,
    pub manufacturer: Option<String>,
    pub vid: Option<u16>,
    pub pid: Option<u16>,
}

/// Heuristically determines if a port path refers to a USB serial
/// device. On non-POSIX systems (e.g. Windows), every port is treated
/// as potentially USB.
pub fn is_usb_serial_port(port: &str) -> bool {
    if !cfg!(target_family = "unix") {
        return true;
    }
    USB_PORT_MARKERS.iter().any(|marker| port.contains(marker))
}

/// One piece of a natural sort key: a number or a lowercase word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyPart {
    Num(u64),
    Text(String),
}

impl Ord for KeyPart {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (KeyPart::Num(a), KeyPart::Num(b)) => a.cmp(b),
            (KeyPart::Text(a), KeyPart::Text(b)) => a.cmp(b),
            // The split alternates text and numbers, so mixed
            // comparisons never occur in practice.
            (KeyPart::Num(_), KeyPart::Text(_)) => std::cmp::Ordering::Less,
            (KeyPart::Text(_), KeyPart::Num(_)) => std::cmp::Ordering::Greater,
        }
    }
}

impl PartialOrd for KeyPart {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Splits a string into lowercase words and numbers so ttyUSB2 sorts
/// before ttyUSB10.
pub fn natural_key(s: &str) -> Vec<KeyPart> {
    let mut parts = Vec::new();
    let mut word = String::new();
    let mut digits = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            if !word.is_empty() {
                parts.push(KeyPart::Text(std::mem::take(&mut word)));
            }
            digits.push(ch);
        } else {
            if !digits.is_empty() {
                parts.push(KeyPart::Num(
                    std::mem::take(&mut digits).parse().unwrap_or(u64::MAX),
                ));
            }
            word.extend(ch.to_lowercase());
        }
    }
    if !word.is_empty() {
        parts.push(KeyPart::Text(word));
    }
    if !digits.is_empty() {
        parts.push(KeyPart::Num(digits.parse().unwrap_or(u64::MAX)));
    }
    parts
}

fn port_sort_key(port: &str) -> (bool, Vec<KeyPart>) {
    (!is_usb_serial_port(port), natural_key(port))
}

/// Sorts ports so USB serial adapters come first, followed by all
/// other ports. Each group is ordered naturally, e.g. ttyUSB2 before
/// ttyUSB10.
pub fn sort_ports(ports: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut ports: Vec<String> = ports.into_iter().collect();
    ports.sort_by_key(|p| port_sort_key(p));
    ports
}

/// Glob-style prefix listing of one directory, the sandbox-safe
/// replacement for enumerating /dev directly.
pub fn glob_prefix(dir: &str, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.starts_with(prefix) {
            out.push(format!("{dir}/{name}"));
        }
    }
    out
}

/// A non-crashing port listing for sandboxed Linux envs.
///
/// Direct enumeration of /dev/ttyS* is forbidden by the snap sandbox
/// and crashes pyserial-style scanners; only the common USB-to-serial
/// patterns permitted by the serial-port interface are globbed.
pub fn safe_list_ports_linux() -> Vec<String> {
    let mut ports = Vec::new();
    for (dir, prefix) in [
        ("/dev", "ttyUSB"),
        ("/dev", "ttyACM"),
        ("/dev", "rfcomm"),
        ("/dev/serial/by-id", ""),
        ("/dev/serial/by-path", ""),
    ] {
        ports.extend(glob_prefix(dir, prefix));
    }
    ports.sort();
    ports
}

static BY_ID_SUFFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-if\d+(-port\d+)?$").unwrap());

/// Extracts a device description from a /dev/serial/by-id symlink
/// name, which embeds vendor and product strings, e.g.
/// 'usb-FTDI_FT232R_USB_UART_AH03K1A0-if00-port0'.
fn describe_by_id_link(path: &str) -> Option<String> {
    let name = Path::new(path).file_name()?.to_str()?;
    let name = name.strip_prefix("usb-")?;
    let name = BY_ID_SUFFIX_RE.replace(name, "");
    let name = name.replace('_', " ").trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Maps port paths to descriptions derived from /dev/serial/by-id
/// symlinks. Both the link path itself and the resolved device path
/// are mapped, so ttyUSB devices get described too.
pub fn by_id_descriptions(paths: &[String]) -> HashMap<String, String> {
    let mut descriptions = HashMap::new();
    for path in paths {
        if !path.contains("/by-id/") {
            continue;
        }
        let Some(desc) = describe_by_id_link(path) else {
            continue;
        };
        let Ok(resolved) = std::fs::canonicalize(path) else {
            continue;
        };
        let Some(resolved) = resolved.to_str() else {
            continue;
        };
        descriptions
            .entry(path.clone())
            .or_insert_with(|| desc.clone());
        descriptions.entry(resolved.to_string()).or_insert(desc);
    }
    descriptions
}

/// Whether running inside a Snap on POSIX (sandboxed enumeration).
pub fn in_snap() -> bool {
    cfg!(target_family = "unix") && std::env::var_os("SNAP").is_some()
}

/// Lists available serial ports, USB adapters first.
pub fn list_ports() -> Vec<String> {
    if in_snap() {
        return sort_ports(safe_list_ports_linux());
    }
    match platform_ports() {
        Ok(ports) => sort_ports(
            ports.into_iter().map(|p| p.port_name).collect::<Vec<_>>(),
        ),
        Err(e) => {
            log_crate::error!("Failed to list serial ports: {e}");
            Vec::new()
        }
    }
}

/// Lists available serial ports together with a human-readable
/// description when the platform provides one. USB adapters are
/// listed first.
pub fn list_port_info() -> Vec<SerialPortInfo> {
    if in_snap() {
        let paths = safe_list_ports_linux();
        let descriptions = by_id_descriptions(&paths);
        let mut infos: Vec<SerialPortInfo> = paths
            .into_iter()
            .map(|path| SerialPortInfo {
                description: descriptions.get(&path).cloned(),
                device: path,
                ..Default::default()
            })
            .collect();
        infos.sort_by_key(|i| port_sort_key(&i.device));
        return infos;
    }

    match platform_ports() {
        Ok(ports) => {
            let mut infos: Vec<SerialPortInfo> = ports
                .into_iter()
                .map(|port| {
                    let usb = match &port.port_type {
                        serialport::SerialPortType::UsbPort(usb) => Some(usb),
                        _ => None,
                    };
                    let description = usb
                        .and_then(|u| u.product.clone())
                        .filter(|d| d != "n/a");
                    SerialPortInfo {
                        device: port.port_name,
                        description,
                        manufacturer: usb.and_then(|u| u.manufacturer.clone()),
                        vid: usb.map(|u| u.vid),
                        pid: usb.map(|u| u.pid),
                    }
                })
                .collect();
            infos.sort_by(|a, b| {
                port_sort_key(&a.device).cmp(&port_sort_key(&b.device))
            });
            infos
        }
        Err(e) => {
            log_crate::error!("Failed to list serial ports: {e}");
            Vec::new()
        }
    }
}

/// Like list_ports, but only returns USB serial ports.
pub fn list_usb_ports() -> Vec<String> {
    let all_ports = list_ports();
    if !cfg!(target_family = "unix") {
        // On non-POSIX systems, we can't reliably filter, so return all.
        return all_ports;
    }
    all_ports
        .into_iter()
        .filter(|p| is_usb_serial_port(p))
        .collect()
}

/// The common serial baud rates offered to applications.
pub fn list_baud_rates() -> Vec<u32> {
    vec![
        9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600, 1_000_000,
        1_843_200,
    ]
}

/// Resolves a USB ``vid:pid`` pair to a device path by scanning the
/// available ports.
///
/// Unlike a fixed path, a VID:PID match survives the OS
/// re-enumerating devices (e.g. ttyUSB0 becoming ttyUSB1 after a
/// reconnect). If several devices share the same VID:PID, the
/// first one on the bus wins. Returns ``None`` when no matching
/// device is present.
pub fn resolve_vidpid(vid: u16, pid: u16) -> Option<String> {
    resolve_vidpid_in(&list_port_info(), vid, pid)
}

/// The matching logic over a caller-supplied port inventory, so the
/// enumeration backend stays replaceable (tests patch it).
pub fn resolve_vidpid_in(
    infos: &[SerialPortInfo],
    vid: u16,
    pid: u16,
) -> Option<String> {
    infos
        .iter()
        .find(|info| info.vid == Some(vid) && info.pid == Some(pid))
        .map(|info| info.device.clone())
}

/// Why serial ports may be unreachable for this process.
///
/// ``NoPorts`` and ``NoAccess`` are the plain conditions; the
/// ``Snap*`` variants apply when the process runs inside a snap,
/// whose confinement changes the remedy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionIssue {
    None,
    NoPorts,
    NoAccess,
    SnapNoPorts,
    SnapNoAccess,
}

impl PermissionIssue {
    /// The wire form: a stable lowercase name applications map to
    /// their own localized messages.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::NoPorts => "no_ports",
            Self::NoAccess => "no_access",
            Self::SnapNoPorts => "snap_no_ports",
            Self::SnapNoAccess => "snap_no_access",
        }
    }
}

#[cfg(unix)]
fn port_is_accessible(path: &str) -> bool {
    use std::ffi::CString;
    let Ok(c_path) = CString::new(path) else {
        return false;
    };
    // access(2) checks the real identity the same way the shell
    // would — the pure-metadata bits cannot model group membership,
    // which is exactly what this diagnostic looks for (dialout).
    unsafe { libc::access(c_path.as_ptr(), libc::R_OK | libc::W_OK) == 0 }
}

#[cfg(not(unix))]
fn port_is_accessible(_path: &str) -> bool {
    false
}

/// Diagnoses systemic serial-port access problems.
///
/// On POSIX systems, checks whether there are visible USB serial
/// ports at all and whether the process could open any of them — a
/// strong indicator that the user is not in the correct group
/// (e.g. ``dialout``) or, in a snap, lacks the necessary
/// permissions. Non-POSIX platforms always report
/// [`PermissionIssue::None`].
pub fn check_permission_issue() -> PermissionIssue {
    check_permission_issue_over(&list_usb_ports())
}

/// The diagnostic over a caller-supplied port inventory, so the
/// enumeration backend stays replaceable (tests patch it).
pub fn check_permission_issue_over(ports: &[String]) -> PermissionIssue {
    if !cfg!(unix) {
        return PermissionIssue::None;
    }
    let in_snap = std::env::var_os("SNAP").is_some();
    if ports.is_empty() {
        return if in_snap {
            PermissionIssue::SnapNoPorts
        } else {
            PermissionIssue::NoPorts
        };
    }
    if ports.iter().any(|path| port_is_accessible(path)) {
        return PermissionIssue::None;
    }
    if in_snap {
        PermissionIssue::SnapNoAccess
    } else {
        PermissionIssue::NoAccess
    }
}
