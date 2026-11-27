"""Tests for raygeo.driver.service (platform service installation).

The definition generators are tested directly; install/uninstall
dispatch is tested with the native tools mocked, so the suite runs
on every platform.
"""

import sys
from pathlib import Path, PurePosixPath

import pytest

from raygeo.driver import service


@pytest.fixture
def config_file(tmp_path):
    path = tmp_path / "daemon.json"
    path.write_text("{}")
    return path


@pytest.fixture
def mock_run(monkeypatch):
    calls: list[list[str]] = []
    monkeypatch.setattr(service, "_run", lambda cmd: calls.append(cmd) or "")
    return calls


def test_systemd_unit_user():
    unit = service.systemd_unit(PurePosixPath("/cfg/daemon.json"))
    assert "-m raygeo.driver daemon --config /cfg/daemon.json" in unit
    assert "WantedBy=default.target" in unit
    assert "User=" not in unit
    assert "Restart=on-failure" in unit


def test_systemd_unit_system():
    unit = service.systemd_unit(PurePosixPath("/cfg/daemon.json"), system=True)
    assert "WantedBy=multi-user.target" in unit
    assert "User=" in unit


def test_systemd_unit_quotes_spaces():
    unit = service.systemd_unit(PurePosixPath("/cfg with space/daemon.json"))
    assert "'/cfg with space/daemon.json'" in unit


def test_launchd_plist():
    plist = service.launchd_plist(PurePosixPath("/cfg/daemon.json"))
    assert plist.startswith("<?xml")
    assert "<string>org.raygeo.driver.daemon</string>" in plist
    assert "<key>RunAtLoad</key><true/>" in plist
    assert "/cfg/daemon.json" in plist


def test_windows_task_command():
    cmd = service.windows_task_command(Path("C:\\cfg\\daemon.json"))
    assert cmd[0] == "schtasks"
    assert "/create" in cmd
    assert "onlogon" in cmd
    assert service.SERVICE_NAME in cmd
    assert "C:\\cfg\\daemon.json" in cmd[cmd.index("/tr") + 1]


def test_install_requires_config(tmp_path, monkeypatch):
    monkeypatch.setattr(sys, "platform", "linux")
    with pytest.raises(service.ServiceError, match="config not found"):
        service.install(tmp_path / "missing.json")


def test_install_linux_user(config_file, monkeypatch, mock_run):
    monkeypatch.setattr(sys, "platform", "linux")
    home = config_file.parent
    monkeypatch.setattr(Path, "home", lambda: home)
    message = service.install(config_file)
    unit = home / ".config" / "systemd" / "user" / "raydriver-daemon.service"
    assert unit.exists()
    assert ["systemctl", "--user", "daemon-reload"] in mock_run
    assert [
        "systemctl",
        "--user",
        "enable",
        "--now",
        "raydriver-daemon",
    ] in mock_run
    assert "installed" in message


def test_install_linux_system_requires_root(
    config_file, monkeypatch, mock_run
):
    monkeypatch.setattr(sys, "platform", "linux")
    monkeypatch.setattr(service, "_is_root", lambda: False)
    with pytest.raises(service.ServiceError, match="root"):
        service.install(config_file, system=True)


def test_uninstall_linux_user(config_file, monkeypatch, mock_run):
    monkeypatch.setattr(sys, "platform", "linux")
    home = config_file.parent
    monkeypatch.setattr(Path, "home", lambda: home)
    service.install(config_file)
    message = service.uninstall()
    unit = home / ".config" / "systemd" / "user" / "raydriver-daemon.service"
    assert not unit.exists()
    assert "removed" in message


def test_install_macos(config_file, monkeypatch, mock_run):
    monkeypatch.setattr(sys, "platform", "darwin")
    home = config_file.parent
    monkeypatch.setattr(Path, "home", lambda: home)
    monkeypatch.setattr(service.os, "getuid", lambda: 501, raising=False)
    service.install(config_file)
    plist = (
        home / "Library" / "LaunchAgents" / "org.raygeo.driver.daemon.plist"
    )
    assert plist.exists()
    assert any(
        cmd[0] == "launchctl" and cmd[1] == "bootstrap" for cmd in mock_run
    )


def test_uninstall_macos(config_file, monkeypatch, mock_run):
    monkeypatch.setattr(sys, "platform", "darwin")
    home = config_file.parent
    monkeypatch.setattr(Path, "home", lambda: home)
    monkeypatch.setattr(service.os, "getuid", lambda: 501, raising=False)
    service.install(config_file)
    message = service.uninstall()
    plist = (
        home / "Library" / "LaunchAgents" / "org.raygeo.driver.daemon.plist"
    )
    assert not plist.exists()
    assert "removed" in message


def test_install_windows(config_file, monkeypatch, mock_run):
    monkeypatch.setattr(sys, "platform", "win32")
    service.install(config_file)
    assert mock_run[0][0] == "schtasks"
    assert "/create" in mock_run[0]
    assert mock_run[1] == ["schtasks", "/run", "/tn", service.SERVICE_NAME]


def test_uninstall_windows(monkeypatch, mock_run):
    monkeypatch.setattr(sys, "platform", "win32")
    service.uninstall()
    assert mock_run[0] == [
        "schtasks",
        "/delete",
        "/f",
        "/tn",
        service.SERVICE_NAME,
    ]


def test_unsupported_platform(config_file, monkeypatch):
    monkeypatch.setattr(sys, "platform", "freebsd14")
    with pytest.raises(service.ServiceError, match="unsupported"):
        service.install(config_file)
    with pytest.raises(service.ServiceError, match="unsupported"):
        service.uninstall()
    with pytest.raises(service.ServiceError, match="unsupported"):
        service.status()
