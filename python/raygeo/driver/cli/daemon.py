"""Daemon CLI: host sessions next to the hardware.

    raydriver daemon add /dev/ttyUSB0    # configure a device
    raydriver daemon remove /dev/ttyUSB0 # unconfigure it
    raydriver daemon --setup             # write a token config only
    raydriver daemon                     # serve the default config
    raydriver daemon --config my.json    # explicit config
    raydriver daemon install             # install + start the service
    raydriver daemon uninstall           # stop + remove the service
    raydriver daemon status              # service + daemon state + token

Service installation is platform-native: systemd on Linux (user
unit by default, --system for a system unit), launchd on macOS, a
logon scheduled task on Windows. The config file is JSON
(stdlib-parseable on Python 3.10) in the platform's per-user config
location. See docs/daemon.md for the full reference. Network
endpoints follow the analyzer security policy: loopback by default,
token and TLS required for exposed listeners.
"""

import argparse
import asyncio
import json
import os
import secrets
import sys
from pathlib import Path
from typing import Any

from raygeo.driver import drivers, service
from raygeo.driver.core import list_serial_ports
from raygeo.driver.paths import default_config_path
from raygeo.driver.remote import inspect as rins
from raygeo.driver.remote import mdns
from raygeo.driver.remote.daemon import Daemon, DaemonConfig

DEFAULT_CONFIG_PATH = default_config_path()

DRIVER_NAMES = {cls.__name__ for cls in drivers}

OPENSSL_HINT = (
    "openssl req -x509 -newkey rsa:2048 -nodes "
    "-keyout daemon-key.pem -out daemon-cert.pem -days 365 "
    "-subj /CN=raydriver "
    "-addext subjectAltName=DNS:localhost,IP:127.0.0.1"
)


def config_from_dict(data: dict[str, Any]) -> DaemonConfig:
    """Map the JSON config onto DaemonConfig."""
    defaults = DaemonConfig()
    return DaemonConfig(
        host=data.get("host", defaults.host),
        port=int(data.get("port", defaults.port)),
        token=data.get("token"),
        tls_cert=data.get("tls_cert"),
        tls_key=data.get("tls_key"),
        keepalive_interval=float(
            data.get("keepalive_interval", defaults.keepalive_interval)
        ),
        keepalive_misses=int(
            data.get("keepalive_misses", defaults.keepalive_misses)
        ),
        lease_grace=float(data.get("lease_grace", defaults.lease_grace)),
        ring_size=int(data.get("ring_size", defaults.ring_size)),
        cameras=data.get("cameras") or [],
        sessions=data.get("sessions") or [],
        mdns=bool(data.get("mdns", defaults.mdns)),
    )


def load_config(path: Path) -> DaemonConfig:
    with open(path, encoding="utf-8") as handle:
        return config_from_dict(json.load(handle))


def write_setup_config(path: Path) -> str:
    """Write a loopback config with a generated token; return it."""
    if path.exists():
        raise FileExistsError(f"config already exists: {path}")
    token = secrets.token_urlsafe(32)
    data = {
        "host": "127.0.0.1",
        "port": DaemonConfig().port,
        "token": token,
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    os.chmod(path, 0o600)
    return token


def add_device(
    path: Path, port: str, protocol_name: str, baudrate: int
) -> str:
    """Add or update a serial device in the config (upsert by port).

    Creates the config with a generated token when none exists yet.
    """
    if protocol_name not in DRIVER_NAMES:
        raise ValueError(
            f"unknown driver {protocol_name!r}; "
            f"choose from {', '.join(sorted(DRIVER_NAMES))}"
        )
    lines = []
    if not path.exists():
        token = write_setup_config(path)
        lines.append(f"wrote {path} (generated token, mode 0600)")
        lines.append(f"token: {token}")
    data = json.loads(path.read_text(encoding="utf-8"))
    sessions = data.setdefault("sessions", [])
    entry = {
        "protocol": protocol_name,
        "config": {"port": port, "baudrate": baudrate},
    }
    for index, session in enumerate(sessions):
        if (session.get("config") or {}).get("port") == port:
            sessions[index] = entry
            lines.append(f"updated {protocol_name} {port} ({baudrate} baud)")
            break
    else:
        sessions.append(entry)
        lines.append(f"added {protocol_name} {port} ({baudrate} baud)")
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    if port not in {item["port"] for item in list_serial_ports()}:
        lines.append(f"note: {port} is not currently visible (device off?)")
    return "\n".join(lines)


def remove_device(path: Path, port: str) -> str:
    """Remove the configured device at *port*."""
    if not path.exists():
        raise ValueError(f"no config at {path}; nothing to remove")
    data = json.loads(path.read_text(encoding="utf-8"))
    sessions = data.get("sessions") or []
    kept = [
        session
        for session in sessions
        if (session.get("config") or {}).get("port") != port
    ]
    if len(kept) == len(sessions):
        raise ValueError(f"no configured device at {port}")
    data["sessions"] = kept
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    return f"removed {port}"


def _device_action(args: argparse.Namespace, path: Path) -> int:
    if not args.device:
        print(
            f"error: daemon {args.action} requires a serial port",
            file=sys.stderr,
        )
        return 2
    try:
        if args.action == "add":
            print(add_device(path, args.device, args.protocol, args.baudrate))
            print("Restart the daemon to open the device.")
        else:
            print(remove_device(path, args.device))
            print("Restart the daemon to close the device.")
    except ValueError as err:
        print(f"error: {err}", file=sys.stderr)
        return 1
    return 0


async def _service_action(args: argparse.Namespace, path: Path) -> int:
    if args.action in ("install", "uninstall"):
        try:
            if args.action == "install":
                print(service.install(path, system=args.system))
            else:
                print(service.uninstall())
        except service.ServiceError as err:
            print(f"error: {err}", file=sys.stderr)
            return 1
        return 0
    try:
        print(service.status())
    except service.ServiceError:
        print("service: not installed")
    if path.exists():
        config = load_config(path)
        if config.token:
            print(f"token: {config.token}")
        host = config.host
        if host in ("0.0.0.0", "::", "*", ""):
            host = "127.0.0.1"
        try:
            welcome = await rins.welcome_probe(host, config.port)
            if welcome is None:
                print("daemon: not reachable")
                return 0
            auth = (
                "auth required" if welcome.get("auth_required") else "no auth"
            )
            print(
                f"daemon: listening on {host}:{config.port} "
                f"(raydriver {welcome.get('raydriver')}, {auth})"
            )
        except (ConnectionError, OSError, asyncio.TimeoutError):
            print("daemon: not reachable")
    return 0


async def cmd_daemon(args) -> int:
    path = Path(args.config) if args.config else DEFAULT_CONFIG_PATH
    if args.action in ("install", "uninstall", "status"):
        return await _service_action(args, path)
    if args.action in ("add", "remove"):
        return _device_action(args, path)
    if args.setup:
        try:
            token = write_setup_config(path)
        except FileExistsError as err:
            print(f"error: {err}", file=sys.stderr)
            return 1
        print(f"wrote {path} (generated token, mode 0600)")
        print(f"token: {token}")
        print("For an exposed listener, generate a certificate:")
        print(f"  {OPENSSL_HINT}")
        print("then set tls_cert and tls_key in the config.")
        return 0
    if not path.exists():
        print(
            f"error: no config at {path}; "
            "run 'raydriver daemon --setup' first",
            file=sys.stderr,
        )
        return 2
    config = load_config(path)
    daemon = Daemon(config)
    server = await daemon.start()
    sockets = server.sockets or []
    port = sockets[0].getsockname()[1] if sockets else config.port
    print(
        f"raydriver daemon listening on {config.host}:{port}",
        flush=True,
    )
    announcer = None
    if config.mdns:
        announcer = await mdns.announce(config, port)
    try:
        await asyncio.Event().wait()
    except asyncio.CancelledError:
        pass
    finally:
        if announcer is not None:
            await announcer.close()
        await daemon.stop()
    return 0


def add_daemon_parser(subparsers) -> None:
    parser = subparsers.add_parser(
        "daemon", help="host sessions for remote clients"
    )
    parser.add_argument(
        "action",
        nargs="?",
        choices=["install", "uninstall", "status", "add", "remove"],
        default=None,
        help="manage the service or configured devices instead of serving",
    )
    parser.add_argument(
        "device",
        nargs="?",
        metavar="PORT",
        help="serial port for add/remove (e.g. /dev/ttyUSB0)",
    )
    parser.add_argument(
        "--protocol",
        default="GrblSerialDriver",
        choices=sorted(DRIVER_NAMES),
        help="driver for add (default: GrblSerialDriver)",
    )
    parser.add_argument(
        "--baudrate",
        type=int,
        default=115200,
        help="serial baudrate for add (default: 115200)",
    )
    parser.add_argument(
        "--config",
        default=None,
        help=f"config file (default: {DEFAULT_CONFIG_PATH})",
    )
    parser.add_argument(
        "--setup",
        action="store_true",
        help="write a config with a generated token and exit",
    )
    parser.add_argument(
        "--system",
        action="store_true",
        help="install a system-wide service (Linux systemd; requires root)",
    )
    parser.add_argument(
        "--log-file",
        default=None,
        help="write Rust-side session diagnostics to this file "
        "(default: stderr)",
    )
    parser.set_defaults(func=cmd_daemon)
