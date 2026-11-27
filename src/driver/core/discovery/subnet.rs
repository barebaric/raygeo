//! Subnet scanning for networked controllers.
//!
//! Some controllers (e.g. grblHAL boards with the raw telnet
//! service) announce nothing via mDNS. To find them, this scanner
//! TCP-connects to a fixed port on every host of the local subnets,
//! then confirms each reachable host with a caller-supplied probe
//! (a command to send and a response-line prefix to match) before
//! claiming it for that caller. The probe is plain data, so this
//! module stays protocol-neutral.
//!
//! Scans stay narrow because probing a whole subnet can trip
//! endpoint-protection tools: only the declared ports are probed,
//! connects and probes are bounded in concurrency and per-host
//! time, and subnets wider than /24 are narrowed to a /24.

use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::sync::Semaphore;

use super::probe::{probe_tcp, TcpProbe};

/// Per-host TCP connect timeout.
const CONNECT_TIMEOUT: Duration = Duration::from_millis(500);
/// Maximum connects in flight during the reachability pass.
const SCAN_CONCURRENCY: usize = 32;
/// Maximum probes in flight during the confirmation pass.
const PROBE_CONCURRENCY: usize = 8;
/// Per-host probe timeout (connect + command + response).
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);
/// How long to read a probe response before giving up.
const PROBE_READ_WINDOW: Duration = Duration::from_secs(1);

/// One caller's declaration of how its devices appear on the
/// network: a TCP port plus an optional confirmation probe.
#[derive(Clone)]
pub struct SubnetSpec {
    /// Opaque caller identifier, echoed back in each hit.
    pub id: String,
    /// TCP port the device listens on.
    pub port: u16,
    /// Command sent to confirm the firmware; ``None`` claims any
    /// listener on the port.
    pub probe_command: Option<String>,
    /// Response-line prefix identifying the firmware.
    pub match_prefix: Option<String>,
}

/// A confirmed device: the claiming caller's id plus where to
/// connect and what the device reported.
pub struct SubnetHit {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub banner: Option<String>,
}

/// Largest subnet scanned per interface, as a CIDR prefix. Wider
/// interfaces (e.g. /16) are narrowed to the /24 containing the
/// interface address so a scan stays a few hundred connects.
const MAX_SCAN_PREFIX: u8 = 24;

/// The local non-loopback IPv4 interfaces, as (address, prefixlen).
pub fn local_subnets() -> Vec<(Ipv4Addr, u8)> {
    let mut out = Vec::new();
    if let Ok(interfaces) = if_addrs::get_if_addrs() {
        for interface in interfaces {
            if let if_addrs::IfAddr::V4(v4) = interface.addr {
                if !v4.is_loopback() && !v4.ip.is_link_local() {
                    out.push((v4.ip, v4.prefixlen));
                }
            }
        }
    }
    out
}

/// The host addresses to probe on the subnet containing *addr*.
/// Networks wider than /24 are narrowed to the /24 around *addr*;
/// the network and broadcast addresses are excluded.
pub fn subnet_hosts(addr: Ipv4Addr, prefixlen: u8) -> Vec<String> {
    let prefix = prefixlen.max(MAX_SCAN_PREFIX);
    let mask = u32::MAX.checked_shl(32 - u32::from(prefix)).unwrap_or(0);
    let base = u32::from(addr) & mask;
    let count = 1u64 << (32 - u32::from(prefix));
    if count <= 2 {
        return (0..count)
            .map(|i| Ipv4Addr::from(base + i as u32).to_string())
            .collect();
    }
    (1..count - 1)
        .map(|i| Ipv4Addr::from(base + i as u32).to_string())
        .collect()
}

/// TCP-connects to *port* on every host, at most *concurrency*
/// connects in flight and *connect_timeout* per host, and returns
/// the hosts that accepted a connection, in input order.
pub async fn scan_subnet(
    hosts: Vec<String>,
    port: u16,
    connect_timeout: Duration,
    concurrency: usize,
) -> Vec<String> {
    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let mut handles = Vec::new();
    for host in hosts {
        let semaphore = semaphore.clone();
        handles.push(tokio::spawn(async move {
            let _permit = semaphore.acquire().await.ok()?;
            let reachable = tokio::time::timeout(
                connect_timeout,
                TcpStream::connect((host.as_str(), port)),
            )
            .await
            .map(|result| result.is_ok())
            .unwrap_or(false);
            reachable.then_some(host)
        }));
    }
    let mut out = Vec::new();
    for handle in handles {
        if let Ok(Some(host)) = handle.await {
            out.push(host);
        }
    }
    out
}

/// Scans the local subnets for the ports declared by *specs* and
/// confirms each reachable host with its spec's probe, returning
/// one hit per claimed host. The first spec (in input order) whose
/// probe matches claims a host. *subnets* overrides the interface
/// enumeration (test seam); ``None`` uses the local interfaces.
///
/// Never raises: a failing or slow scan yields no hits.
pub async fn find_subnet_devices(
    specs: Vec<SubnetSpec>,
    subnets: Option<Vec<(Ipv4Addr, u8)>>,
) -> Vec<SubnetHit> {
    if specs.is_empty() {
        return Vec::new();
    }
    let mut ports: Vec<u16> = specs.iter().map(|s| s.port).collect();
    ports.sort_unstable();
    ports.dedup();

    let mut hosts_by_port: HashMap<u16, Vec<String>> = HashMap::new();
    for (addr, prefixlen) in subnets.unwrap_or_else(local_subnets) {
        let hosts = subnet_hosts(addr, prefixlen);
        for port in &ports {
            let bucket = hosts_by_port.entry(*port).or_default();
            for host in &hosts {
                if !bucket.contains(host) {
                    bucket.push(host.clone());
                }
            }
        }
    }

    let mut hits = Vec::new();
    for port in ports {
        let Some(hosts) = hosts_by_port.remove(&port) else {
            continue;
        };
        let reachable =
            scan_subnet(hosts, port, CONNECT_TIMEOUT, SCAN_CONCURRENCY).await;
        hits.extend(confirm_hosts(reachable, port, &specs).await);
    }
    hits
}

/// Probes every reachable host with the specs that declared its
/// port; the first matching spec claims the host.
async fn confirm_hosts(
    hosts: Vec<String>,
    port: u16,
    specs: &[SubnetSpec],
) -> Vec<SubnetHit> {
    let mut probes: Vec<(SubnetSpec, String)> = Vec::new();
    for spec in specs.iter().filter(|spec| spec.port == port) {
        for host in &hosts {
            probes.push((spec.clone(), host.clone()));
        }
    }
    if probes.is_empty() {
        return Vec::new();
    }

    let semaphore = Arc::new(Semaphore::new(PROBE_CONCURRENCY));
    let mut handles = Vec::new();
    for (spec, host) in probes {
        let semaphore = semaphore.clone();
        handles.push(tokio::spawn(async move {
            let _permit = semaphore.acquire().await.ok()?;
            let banner = match &spec.probe_command {
                None => Some(None),
                Some(_) => tokio::time::timeout(
                    PROBE_TIMEOUT,
                    probe_host(&host, port, &spec),
                )
                .await
                .ok()
                .flatten(),
            };
            banner.map(|banner| SubnetHit {
                id: spec.id.clone(),
                host,
                port,
                banner,
            })
        }));
    }

    let mut hits = Vec::new();
    let mut claimed: Vec<String> = Vec::new();
    for handle in handles {
        if let Ok(Some(hit)) = handle.await {
            if !claimed.contains(&hit.host) {
                claimed.push(hit.host.clone());
                hits.push(hit);
            }
        }
    }
    hits
}

/// Sends the spec's probe command and returns the first response
/// line starting with its match prefix, or ``None`` when the host
/// does not answer like the spec's firmware.
async fn probe_host(
    host: &str,
    port: u16,
    spec: &SubnetSpec,
) -> Option<Option<String>> {
    let command = spec.probe_command.as_ref()?;
    let prefix = spec.match_prefix.as_deref().unwrap_or("");
    let line = probe_tcp(
        host,
        port,
        &TcpProbe {
            connect_timeout: PROBE_TIMEOUT,
            banner_wait: Duration::ZERO,
            response_wait: PROBE_READ_WINDOW,
            command: Some(command),
            match_prefix: prefix,
        },
    )
    .await;
    line.map(Some)
}
