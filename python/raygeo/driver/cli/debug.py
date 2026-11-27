"""Debug CLI: inspect devices, daemons and connections.

    raydriver scan [--network]   # local serial ports (+ daemons)
    raydriver discover           # run the discovery engine
    raydriver info TARGET        # a serial port or a host:port daemon
    raydriver list               # configured devices + live state
    raydriver connection         # clients connected to the daemon

Every command accepts --json for machine-readable output. Daemon
commands read the endpoint and credentials from the daemon config
(--config to override, --token/--tls-ca to override credentials).
"""

from __future__ import annotations

import argparse
import asyncio
import json
import sys
from pathlib import Path
from typing import Any
from urllib.parse import urlsplit

from raygeo.driver.cli.daemon import load_config
from raygeo.driver.core import list_serial_ports
from raygeo.driver.core.discovery import engine
from raygeo.driver.core.discovery.engine import find_all_devices
from raygeo.driver.core.discovery.serial_scan import looks_like_device_output
from raygeo.driver.core.discovery.spec import (
    DiscoverySpec,
    SerialRecognizer,
    TcpRecognizer,
)
from raygeo.driver.paths import default_config_path
from raygeo.driver.remote import inspect as rins
from raygeo.driver.remote import mdns
from raygeo.driver.remote.daemon import DaemonConfig

_WILDCARD_HOSTS = ("0.0.0.0", "::", "*", "")


def _config_path(args: argparse.Namespace) -> Path:
    return Path(args.config) if args.config else default_config_path()


def _load_config(args: argparse.Namespace) -> DaemonConfig:
    """The daemon config, or defaults when no config file exists yet
    (so the debug commands work before ``daemon --setup``)."""
    path = _config_path(args)
    if not path.exists():
        return DaemonConfig()
    return load_config(path)


def _endpoint(
    args: argparse.Namespace,
) -> tuple[str, int, str | None, str | None]:
    config = _load_config(args)
    host = config.host
    if host in _WILDCARD_HOSTS:
        host = "127.0.0.1"
    token = args.token if args.token is not None else config.token
    return host, config.port, token, args.tls_ca


def _as_dict(value: object) -> dict[str, Any]:
    assert isinstance(value, dict)
    return value


def _as_dicts(value: object) -> list[dict[str, Any]]:
    assert isinstance(value, list)
    return value


def _emit(args: argparse.Namespace, data: Any, rows: list[str]) -> None:
    if args.json:
        print(json.dumps(data, indent=2))
    else:
        for line in rows:
            print(line)


def _port_line(port: dict[str, Any]) -> str:
    vid = port.get("vid")
    pid = port.get("pid")
    vidpid = f"{vid:04x}:{pid:04x}" if vid is not None else "----:----"
    product = port.get("product") or ""
    maker = port.get("manufacturer") or ""
    return f"  {port['port']}  {vidpid}  {product}  {maker}".rstrip()


def _first_informative_line(data: bytes) -> str | None:
    for raw in data.decode("ascii", errors="replace").splitlines():
        line = raw.strip()
        if line and line != "ok":
            return line[:80]
    return None


class _SerialProbeDriver:
    """Generic serial declaration for ``discover``: any device-like
    output counts; the first informative line becomes the banner."""

    label = "Serial device"
    DISCOVERY = DiscoverySpec(
        serial=SerialRecognizer(
            label=lambda: "Serial device",
            matches=looks_like_device_output,
            name=_first_informative_line,
        )
    )


async def cmd_discover(args: argparse.Namespace) -> int:
    if args.timeout is not None:
        engine.SUBNET_SCAN_TIMEOUT = args.timeout
    drivers: list[type] = [_SerialProbeDriver]
    if args.tcp is not None:
        drivers.append(
            type(
                "TcpProbeDriver",
                (),
                {
                    "label": "TCP device",
                    "DISCOVERY": DiscoverySpec(
                        tcp=TcpRecognizer(
                            port=args.tcp,
                            probe_command=args.probe,
                            match_prefix=args.match,
                        )
                    ),
                },
            )
        )
    devices = await find_all_devices(
        drivers, ports=args.ports, baud_rates=args.baud
    )
    rows = ["Discovered devices:"]
    if not devices:
        rows.append("  (none found)")
    for device in devices:
        rows.append(f"  {device.label}  {device.detail}  [{device.key}]")
    data = [
        {
            "driver": d.driver_name,
            "label": d.label,
            "detail": d.detail,
            "key": d.key,
            "params": d.params,
            "banner": d.identity.banner,
        }
        for d in devices
    ]
    _emit(args, data, rows)
    return 0


async def cmd_scan(args: argparse.Namespace) -> int:
    ports = list_serial_ports()
    daemons: list[dict[str, Any]] = []
    if args.network:
        daemons = await mdns.browse_daemons()
    rows = ["Serial ports:"]
    if not ports:
        rows.append("  (none found)")
    rows.extend(_port_line(port) for port in ports)
    if args.network:
        rows.append("Daemons (_raydriver._tcp):")
        if not daemons:
            rows.append("  (none found)")
        for daemon in daemons:
            rows.append(
                f"  {daemon['name']}  {daemon['host']}:{daemon['port']}"
            )
    _emit(args, {"ports": ports, "daemons": daemons}, rows)
    return 0


def _parse_endpoint(target: str) -> tuple[str | None, int, str | None]:
    if target.startswith("raydriver://"):
        parts = urlsplit(target)
        return parts.hostname, parts.port or 8765, parts.fragment or None
    host, sep, port = target.rpartition(":")
    if sep and host and port.isdigit():
        return host, int(port), None
    return None, 0, None


async def cmd_info(args: argparse.Namespace) -> int:
    host, port, url_token = _parse_endpoint(args.target)
    if host is None:
        for entry in list_serial_ports():
            if entry["port"] == args.target:
                rows = [f"{key}: {value}" for key, value in entry.items()]
                _emit(args, entry, rows)
                return 0
        print(f"error: no serial port {args.target!r} found", file=sys.stderr)
        return 1
    token = args.token if args.token is not None else url_token
    if token is None:
        token = _load_config(args).token
    try:
        snap = await rins.snapshot(host, port, token, args.tls_ca)
    except (ConnectionError, OSError, asyncio.TimeoutError) as err:
        print(f"error: {err}", file=sys.stderr)
        return 1
    welcome = _as_dict(snap["welcome"])
    sessions = _as_dicts(snap["sessions"])
    rows = [
        f"Daemon {host}:{port}",
        f"  raydriver {welcome.get('raydriver')} "
        f"(protocol {welcome.get('protocol')})",
        f"  sessions: {len(sessions)}  "
        f"clients: {len(_as_dicts(snap['clients']))}",
    ]
    for session in sessions:
        rows.append(
            f"  session {session['session_id']} [{session['protocol']}] "
            f"{session.get('connection_status')} "
            f"config={session.get('config')}"
        )
    rows.append("  devices:")
    rows.extend(f"    {entry['port']}" for entry in _as_dicts(snap["devices"]))
    for camera in _as_dicts(snap["cameras"]):
        rows.append(f"  camera {camera.get('name')}: {camera.get('uri')}")
    _emit(args, snap, rows)
    return 0


async def cmd_list(args: argparse.Namespace) -> int:
    path = _config_path(args)
    config = _load_config(args)
    host, port, token, tls_ca = _endpoint(args)
    live: list[dict[str, Any]] = []
    state = "daemon not running"
    try:
        snap = await rins.snapshot(host, port, token, tls_ca)
        live = _as_dicts(snap["sessions"])
        state = "running"
    except (ConnectionError, OSError, asyncio.TimeoutError):
        pass
    rows = [f"Configured devices ({path}):"]
    if not config.sessions:
        rows.append("  (none configured)")
    matched: set[str] = set()
    for entry in config.sessions:
        entry_config = _as_dict(entry.get("config") or {})
        device = entry_config.get("port") or entry_config.get("host") or "?"
        match = next(
            (
                session
                for session in live
                if session["protocol"] == entry.get("protocol")
                and session.get("config") == entry_config
            ),
            None,
        )
        if match is not None:
            matched.add(match["session_id"])
            job = match.get("job")
            job_state = "job running" if job and job.get("running") else "idle"
            rows.append(
                f"  {entry.get('protocol')} {device}: "
                f"{match.get('connection_status')} ({job_state})"
            )
        else:
            rows.append(
                f"  {entry.get('protocol')} {device}: not open ({state})"
            )
    for session in live:
        if session["session_id"] not in matched:
            rows.append(
                f"  {session['protocol']} (ad-hoc {session['session_id']}): "
                f"{session.get('connection_status')}"
            )
    _emit(
        args,
        {"configured": config.sessions, "live": live, "daemon": state},
        rows,
    )
    return 0


async def cmd_connection(args: argparse.Namespace) -> int:
    host, port, token, tls_ca = _endpoint(args)
    try:
        snap = await rins.snapshot(host, port, token, tls_ca)
    except (ConnectionError, OSError, asyncio.TimeoutError) as err:
        print(
            f"error: cannot reach daemon at {host}:{port}: {err}",
            file=sys.stderr,
        )
        return 1
    rows = [f"Rayforge connections to {host}:{port}:"]
    if not snap["clients"]:
        rows.append("  (no clients connected)")
    for client in _as_dicts(snap["clients"]):
        roles = (
            ", ".join(
                f"{session}:{role}"
                for session, role in _as_dict(client["roles"]).items()
            )
            or "no sessions"
        )
        rows.append(
            f"  {client['address']} (id {client['id']}, "
            f"auth={client['authenticated']}) {roles}"
        )
    for session in _as_dicts(snap["sessions"]):
        holder = session.get("lease_holder") or "-"
        rows.append(
            f"  session {session['session_id']}: lease={holder} "
            f"watchers={session.get('watchers')}"
        )
    _emit(args, snap, rows)
    return 0


def _add_daemon_options(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--token", default=None, help="daemon token")
    parser.add_argument(
        "--tls-ca", default=None, help="PEM file pinning the daemon cert"
    )
    parser.add_argument(
        "--json", action="store_true", help="machine-readable output"
    )


def _add_config_option(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--config",
        default=None,
        help=f"daemon config file (default: {default_config_path()})",
    )


def add_debug_parser(subparsers) -> None:
    scan = subparsers.add_parser(
        "scan", help="list local serial ports (and daemons with --network)"
    )
    scan.add_argument(
        "--network",
        action="store_true",
        help="also browse for raydriver daemons via mDNS",
    )
    scan.add_argument(
        "--json", action="store_true", help="machine-readable output"
    )
    scan.set_defaults(func=cmd_scan)

    discover = subparsers.add_parser(
        "discover",
        help="run device discovery (serial capture, mDNS, subnet)",
    )
    discover.add_argument(
        "--ports",
        nargs="*",
        help="serial ports to probe (default: all USB ports)",
    )
    discover.add_argument(
        "--baud",
        nargs="*",
        type=int,
        help="baud rates to try (default: 115200 57600 9600)",
    )
    discover.add_argument(
        "--tcp",
        type=int,
        metavar="PORT",
        help="also probe this TCP port on the local subnets",
    )
    discover.add_argument(
        "--probe",
        help="command sent to confirm firmware on --tcp listeners",
    )
    discover.add_argument(
        "--match",
        help="response-line prefix identifying the firmware",
    )
    discover.add_argument(
        "--timeout",
        type=float,
        help="subnet scan budget in seconds (default: 30)",
    )
    discover.add_argument(
        "--json", action="store_true", help="machine-readable output"
    )
    discover.set_defaults(func=cmd_discover)

    info = subparsers.add_parser(
        "info", help="show a serial port or a daemon (host:port)"
    )
    info.add_argument("target", help="serial port path or host:port")
    _add_config_option(info)
    _add_daemon_options(info)
    info.set_defaults(func=cmd_info)

    listing = subparsers.add_parser(
        "list", help="list configured devices with live connection state"
    )
    _add_config_option(listing)
    _add_daemon_options(listing)
    listing.set_defaults(func=cmd_list)

    connection = subparsers.add_parser(
        "connection", help="show Rayforge clients connected to the daemon"
    )
    _add_config_option(connection)
    _add_daemon_options(connection)
    connection.set_defaults(func=cmd_connection)
