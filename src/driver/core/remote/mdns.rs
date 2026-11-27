//! mDNS announce and browse for the daemon (`_raydriver._tcp`).

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};

pub const SERVICE_TYPE: &str = "_raydriver._tcp.local.";

fn host_name() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "localhost".to_string())
}

fn local_ip() -> String {
    if_addrs::get_if_addrs()
        .ok()
        .and_then(|interfaces| {
            interfaces
                .into_iter()
                .find_map(|interface| match interface.addr {
                    if_addrs::IfAddr::V4(v4) if !v4.ip.is_loopback() => {
                        Some(v4.ip.to_string())
                    }
                    _ => None,
                })
        })
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

fn is_loopback(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    host.parse::<IpAddr>()
        .map(|address| address.is_loopback())
        .unwrap_or(false)
}

/// The address clients should connect to, matching the bind.
pub fn announce_address(host: &str) -> String {
    if is_loopback(host) {
        return "127.0.0.1".to_string();
    }
    if matches!(host, "0.0.0.0" | "::" | "*" | "") {
        return local_ip();
    }
    host.to_string()
}

/// A DNS-safe service instance name (each label <= 63 chars).
pub fn service_name() -> String {
    service_name_for(&host_name())
}

/// A DNS-safe service instance name for *host*.
pub fn service_name_for(host: &str) -> String {
    let host = host.replace('.', "-");
    let mut label = format!("raydriver-{host}");
    if label.len() > 63 {
        let mut hasher = Sha1::new();
        hasher.update(host.as_bytes());
        let digest = hasher.finalize();
        let hash = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        label = format!("raydriver-{}-{}", &host[..44], &hash[..8]);
    }
    label
}

pub struct MdnsAnnouncer {
    daemon: ServiceDaemon,
    fullname: String,
}

impl MdnsAnnouncer {
    pub fn fullname(&self) -> &str {
        &self.fullname
    }

    pub fn close(&mut self) {
        if let Err(err) = self.daemon.unregister(&self.fullname) {
            log_crate::debug!("mDNS unregister failed: {err}");
        }
        if let Err(err) = self.daemon.shutdown() {
            log_crate::debug!("mDNS shutdown failed: {err}");
        }
    }
}

/// Announce the daemon on the LAN so clients discover it.
///
/// Best-effort: returns `None` (and logs) if mDNS cannot be set up,
/// so a discovery problem never prevents the daemon from serving.
pub fn announce(
    bind_host: &str,
    port: u16,
    requires_auth: bool,
    tls: bool,
    camera_count: usize,
    instance: Option<&str>,
) -> Option<MdnsAnnouncer> {
    let daemon = match ServiceDaemon::new() {
        Ok(daemon) => daemon,
        Err(err) => {
            log_crate::warn!("mDNS announce failed: {err}");
            return None;
        }
    };
    let address = announce_address(bind_host);
    let host = format!("{}.local.", host_name().replace('.', "-"));
    let instance = instance
        .map(|name| name.split('.').next().unwrap_or(name).to_string())
        .unwrap_or_else(service_name);
    let properties = [
        ("protocol", "1".to_string()),
        ("requires_auth", requires_auth.to_string()),
        ("tls", tls.to_string()),
        ("cameras", camera_count.to_string()),
    ];
    let info = match ServiceInfo::new(
        SERVICE_TYPE,
        &instance,
        &host,
        address.as_str(),
        port,
        &properties[..],
    ) {
        Ok(info) => info,
        Err(err) => {
            log_crate::warn!("mDNS announce failed: {err}");
            let _ = daemon.shutdown();
            return None;
        }
    };
    let fullname = info.get_fullname().to_string();
    if let Err(err) = daemon.register(info) {
        log_crate::warn!("mDNS announce failed: {err}");
        let _ = daemon.shutdown();
        return None;
    }
    Some(MdnsAnnouncer { daemon, fullname })
}

/// Browse for daemons for *timeout* seconds.
pub async fn browse_daemons(timeout: Duration) -> Vec<Value> {
    tokio::task::spawn_blocking(move || browse_blocking(timeout))
        .await
        .unwrap_or_default()
}

fn browse_blocking(timeout: Duration) -> Vec<Value> {
    let daemon = match ServiceDaemon::new() {
        Ok(daemon) => daemon,
        Err(_) => return Vec::new(),
    };
    let receiver = match daemon.browse(SERVICE_TYPE) {
        Ok(receiver) => receiver,
        Err(_) => {
            let _ = daemon.shutdown();
            return Vec::new();
        }
    };

    let mut found: HashMap<String, Value> = HashMap::new();
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(ServiceEvent::ServiceResolved(resolved)) => {
                let ipv4 = resolved.get_addresses_v4();
                let address = ipv4
                    .into_iter()
                    .next()
                    .map(|ip| IpAddr::V4(ip).to_string())
                    .or_else(|| {
                        resolved
                            .get_addresses()
                            .iter()
                            .next()
                            .map(|ip| ip.to_ip_addr().to_string())
                    })
                    .unwrap_or_default();
                if address.is_empty() {
                    continue;
                }
                let txt: serde_json::Map<String, Value> = resolved
                    .get_properties()
                    .iter()
                    .map(|property| {
                        (property.key().to_string(), json!(property.val_str()))
                    })
                    .collect();
                found.insert(
                    resolved.get_fullname().to_string(),
                    json!({
                        "name": resolved.get_fullname(),
                        "host": address,
                        "port": resolved.get_port(),
                        "txt": txt,
                    }),
                );
            }
            Ok(ServiceEvent::ServiceRemoved(_, fullname)) => {
                found.remove(&fullname);
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }
    let _ = daemon.stop_browse(SERVICE_TYPE);
    let _ = daemon.shutdown();
    let mut entries: Vec<Value> = found.into_values().collect();
    entries.sort_by_key(|entry| {
        entry
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    });
    entries
}
