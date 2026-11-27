//! mDNS network service scanning.
//!
//! A browse runs for a short, bounded window; every service
//! announcement on the watched types is resolved to an address and
//! captured as an [`MdnsService`]. Deciding which driver could talk
//! to a discovered service lives with the embedding application.
//!
//! A scanner is created and closed per scan: nothing keeps sockets
//! open between two discovery runs. Failures (no network, no
//! responder) yield an empty result rather than an error.

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use mdns_sd::{ServiceDaemon, ServiceEvent};

/// Grace period for resolving services announced near the end of
/// the browse window.
const RESOLVE_GRACE: Duration = Duration::from_secs(1);
/// Poll granularity while collecting browse events.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// One service resolved during an mDNS browse.
pub struct MdnsService {
    /// Normalized service type, e.g. `_octoprint._tcp`.
    pub service_type: String,
    /// Instance name, e.g. `OctoPrint on octopi`.
    pub name: String,
    /// Address to connect to; IPv4 preferred over IPv6.
    pub host: String,
    pub port: u16,
    /// The server's own hostname, e.g. `octopi.local`.
    pub server: String,
    /// TXT record contents, values decoded to strings.
    pub txt: HashMap<String, String>,
}

/// Reduces a service type to its canonical `_name._transport`
/// form: `_octoprint._tcp.local.` becomes `_octoprint._tcp`.
pub fn normalize_service_type(service_type: &str) -> String {
    service_type
        .trim_matches('.')
        .split('.')
        .filter(|part| !part.is_empty() && !part.eq_ignore_ascii_case("local"))
        .collect::<Vec<_>>()
        .join(".")
}

/// The full `.local.`-suffixed form a browse expects.
fn full_service_type(service_type: &str) -> String {
    let normalized = normalize_service_type(service_type);
    format!("{normalized}.local.")
}

/// Browses *service_types* over mDNS for *browse_window* and returns
/// every service that could be resolved to an address. Never raises:
/// a failing or slow browse yields an empty list.
pub async fn scan_mdns_services(
    service_types: Vec<String>,
    browse_window: Duration,
) -> Vec<MdnsService> {
    tokio::task::spawn_blocking(move || {
        browse_blocking(&service_types, browse_window)
    })
    .await
    .unwrap_or_default()
}

fn browse_blocking(
    service_types: &[String],
    browse_window: Duration,
) -> Vec<MdnsService> {
    let normalized: Vec<String> = service_types
        .iter()
        .map(|t| normalize_service_type(t))
        .filter(|t| !t.is_empty())
        .collect();
    if normalized.is_empty() {
        return Vec::new();
    }

    let discovery = match ServiceDaemon::new() {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let mut receivers = Vec::new();
    for service_type in &normalized {
        if let Ok(receiver) = discovery.browse(&full_service_type(service_type))
        {
            receivers.push(receiver);
        }
    }

    let mut services: HashMap<String, MdnsService> = HashMap::new();
    let deadline = Instant::now() + browse_window;
    collect_events(&receivers, &mut services, deadline);
    // Grace period for services announced near the end of the window.
    collect_events(&receivers, &mut services, Instant::now() + RESOLVE_GRACE);

    for service_type in &normalized {
        let _ = discovery.stop_browse(&full_service_type(service_type));
    }
    let _ = discovery.shutdown();

    services.into_values().collect()
}

/// Drains resolved-service events from every browse receiver until
/// *deadline*, keeping one entry per instance name.
fn collect_events(
    receivers: &[mdns_sd::Receiver<ServiceEvent>],
    services: &mut HashMap<String, MdnsService>,
    deadline: Instant,
) {
    while Instant::now() < deadline {
        for receiver in receivers {
            match receiver.recv_timeout(POLL_INTERVAL) {
                Ok(ServiceEvent::ServiceResolved(resolved)) => {
                    if let Some(service) = build_service(&resolved) {
                        services.insert(
                            resolved.get_fullname().to_string(),
                            service,
                        );
                    }
                }
                Ok(ServiceEvent::ServiceRemoved(_, fullname)) => {
                    services.remove(&fullname);
                }
                Ok(_) => {}
                Err(_) => {}
            }
        }
    }
}

/// Converts a resolved service into an [`MdnsService`], preferring
/// IPv4 addresses. Returns None when no address was resolved.
fn build_service(resolved: &mdns_sd::ResolvedService) -> Option<MdnsService> {
    let ipv4 = resolved.get_addresses_v4();
    let chosen: Option<IpAddr> =
        ipv4.into_iter().next().map(IpAddr::V4).or_else(|| {
            resolved
                .get_addresses()
                .iter()
                .next()
                .map(|ip| ip.to_ip_addr())
        });
    let host = chosen?.to_string();

    let fullname = resolved.get_fullname();
    let (name, ty_domain) = fullname.split_once('.')?;
    let mut txt = HashMap::new();
    for property in resolved.get_properties().iter() {
        txt.insert(property.key().to_string(), property.val_str().to_string());
    }
    Some(MdnsService {
        service_type: normalize_service_type(ty_domain),
        name: name.to_string(),
        host,
        port: resolved.get_port(),
        server: resolved.get_hostname().trim_end_matches('.').to_string(),
        txt,
    })
}
