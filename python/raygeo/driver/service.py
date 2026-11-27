"""Per-user service installation for the raydriver daemon.

Platform backends, all stdlib-only:

- Linux: systemd user unit (``system=True`` for a system unit).
- macOS: launchd LaunchAgent.
- Windows: scheduled task running at logon.

Every operation shells out to the platform's native tools. The
definition generators are pure functions, so they can be tested on
any platform.
"""

from __future__ import annotations

import getpass
import os
import shlex
import subprocess
import sys
from pathlib import Path, PurePath
from xml.sax.saxutils import escape

SERVICE_NAME = "raydriver-daemon"
LAUNCHD_LABEL = "org.raygeo.driver.daemon"


class ServiceError(Exception):
    """A service operation failed; the message is user-facing."""


def _run(cmd: list[str]) -> str:
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise ServiceError(f"{shlex.join(cmd)} failed: {detail}")
    return result.stdout


def _exec_command(config_path: PurePath) -> list[str]:
    return [
        sys.executable,
        "-m",
        "raygeo.driver",
        "daemon",
        "--config",
        str(config_path),
    ]


def _is_root() -> bool:
    return getattr(os, "geteuid", lambda: 0)() == 0


def systemd_unit(config_path: PurePath, system: bool = False) -> str:
    """The systemd unit file content for the daemon."""
    target = "multi-user.target" if system else "default.target"
    user_line = ""
    if system:
        user_line = f"User={getpass.getuser()}\n"
    exec_start = shlex.join(_exec_command(config_path))
    return (
        "[Unit]\n"
        "Description=raydriver device daemon\n"
        "After=network-online.target\n"
        "\n"
        "[Service]\n"
        f"ExecStart={exec_start}\n"
        f"{user_line}"
        "Restart=on-failure\n"
        "RestartSec=2\n"
        "\n"
        "[Install]\n"
        f"WantedBy={target}\n"
    )


def launchd_plist(config_path: PurePath) -> str:
    """The launchd LaunchAgent plist content for the daemon."""
    args = "".join(
        f"<string>{escape(arg)}</string>" for arg in _exec_command(config_path)
    )
    log = Path.home() / "Library" / "Logs" / f"{SERVICE_NAME}.log"
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" '
        '"http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n'
        '<plist version="1.0">\n'
        "<dict>\n"
        f"<key>Label</key><string>{LAUNCHD_LABEL}</string>\n"
        f"<key>ProgramArguments</key><array>{args}</array>\n"
        "<key>RunAtLoad</key><true/>\n"
        "<key>KeepAlive</key>\n"
        "<dict><key>SuccessfulExit</key><false/></dict>\n"
        f"<key>StandardOutPath</key><string>{escape(str(log))}</string>\n"
        f"<key>StandardErrorPath</key><string>{escape(str(log))}</string>\n"
        "</dict>\n"
        "</plist>\n"
    )


def windows_task_command(config_path: PurePath) -> list[str]:
    """The schtasks command that creates the logon task."""
    python = Path(sys.executable)
    pythonw = python.with_name("pythonw.exe")
    exe = str(pythonw if pythonw.exists() else python)
    task = f'"{exe}" -m raygeo.driver daemon --config "{config_path}"'
    return [
        "schtasks",
        "/create",
        "/f",
        "/tn",
        SERVICE_NAME,
        "/tr",
        task,
        "/sc",
        "onlogon",
    ]


def install(config_path: Path, system: bool = False) -> str:
    """Install and start the daemon service for this platform."""
    config_path = config_path.resolve()
    if not config_path.exists():
        raise ServiceError(
            f"config not found: {config_path}; "
            "run 'raydriver daemon --setup' first"
        )
    if sys.platform.startswith("linux"):
        return _install_systemd(config_path, system)
    if sys.platform == "darwin":
        return _install_launchd(config_path)
    if sys.platform == "win32":
        return _install_windows(config_path)
    raise ServiceError(f"unsupported platform: {sys.platform}")


def uninstall() -> str:
    """Stop and remove the daemon service for this platform."""
    if sys.platform.startswith("linux"):
        return _uninstall_systemd()
    if sys.platform == "darwin":
        return _uninstall_launchd()
    if sys.platform == "win32":
        _run(["schtasks", "/delete", "/f", "/tn", SERVICE_NAME])
        return f"removed scheduled task '{SERVICE_NAME}'"
    raise ServiceError(f"unsupported platform: {sys.platform}")


def status() -> str:
    """The native service manager's status for the daemon."""
    if sys.platform.startswith("linux"):
        return _status_systemd()
    if sys.platform == "darwin":
        uid = getattr(os, "getuid", lambda: 0)()
        return _run(["launchctl", "print", f"gui/{uid}/{LAUNCHD_LABEL}"])
    if sys.platform == "win32":
        return _run(["schtasks", "/query", "/tn", SERVICE_NAME, "/fo", "LIST"])
    raise ServiceError(f"unsupported platform: {sys.platform}")


def _install_systemd(config_path: Path, system: bool) -> str:
    if system:
        if not _is_root():
            raise ServiceError("--system requires root (run with sudo)")
        unit_dir = Path("/etc/systemd/system")
        cmd = ["systemctl"]
    else:
        unit_dir = Path.home() / ".config" / "systemd" / "user"
        cmd = ["systemctl", "--user"]
    unit_dir.mkdir(parents=True, exist_ok=True)
    unit_path = unit_dir / f"{SERVICE_NAME}.service"
    unit_path.write_text(systemd_unit(config_path, system), encoding="utf-8")
    _run(cmd + ["daemon-reload"])
    _run(cmd + ["enable", "--now", SERVICE_NAME])
    hint = ""
    if not system:
        hint = "; for boot start without login: loginctl enable-linger $USER"
    return f"installed {unit_path} and started {SERVICE_NAME}{hint}"


def _uninstall_systemd() -> str:
    user_unit = (
        Path.home()
        / ".config"
        / "systemd"
        / "user"
        / f"{SERVICE_NAME}.service"
    )
    system_unit = Path("/etc/systemd/system") / f"{SERVICE_NAME}.service"
    if user_unit.exists():
        _run(["systemctl", "--user", "disable", "--now", SERVICE_NAME])
        user_unit.unlink()
        _run(["systemctl", "--user", "daemon-reload"])
        return f"removed {user_unit}"
    if system_unit.exists():
        if not _is_root():
            raise ServiceError(
                "removing the system unit requires root (run with sudo)"
            )
        _run(["systemctl", "disable", "--now", SERVICE_NAME])
        system_unit.unlink()
        _run(["systemctl", "daemon-reload"])
        return f"removed {system_unit}"
    raise ServiceError(f"no {SERVICE_NAME} service installed")


def _status_systemd() -> str:
    user_unit = (
        Path.home()
        / ".config"
        / "systemd"
        / "user"
        / f"{SERVICE_NAME}.service"
    )
    if user_unit.exists():
        return _run(["systemctl", "--user", "status", SERVICE_NAME])
    return _run(["systemctl", "status", SERVICE_NAME])


def _install_launchd(config_path: Path) -> str:
    agents = Path.home() / "Library" / "LaunchAgents"
    agents.mkdir(parents=True, exist_ok=True)
    plist = agents / f"{LAUNCHD_LABEL}.plist"
    plist.write_text(launchd_plist(config_path), encoding="utf-8")
    uid = getattr(os, "getuid", lambda: 0)()
    _run(["launchctl", "bootstrap", f"gui/{uid}", str(plist)])
    return f"installed {plist} and started {LAUNCHD_LABEL}"


def _uninstall_launchd() -> str:
    plist = Path.home() / "Library" / "LaunchAgents" / f"{LAUNCHD_LABEL}.plist"
    uid = getattr(os, "getuid", lambda: 0)()
    if plist.exists():
        _run(["launchctl", "bootout", f"gui/{uid}/{LAUNCHD_LABEL}"])
        plist.unlink()
        return f"removed {plist}"
    raise ServiceError(f"no {LAUNCHD_LABEL} service installed")


def _install_windows(config_path: Path) -> str:
    _run(windows_task_command(config_path))
    _run(["schtasks", "/run", "/tn", SERVICE_NAME])
    return (
        f"installed scheduled task '{SERVICE_NAME}' "
        "(runs at logon) and started it"
    )
