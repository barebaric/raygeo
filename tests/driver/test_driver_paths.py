"""Tests for raygeo.driver.paths (platform-aware config locations)."""

import sys
from pathlib import Path

from raygeo.driver import paths


def test_linux_xdg(monkeypatch):
    monkeypatch.setattr(sys, "platform", "linux")
    monkeypatch.setenv("XDG_CONFIG_HOME", "/tmp/xdg")
    assert paths.config_dir() == Path("/tmp/xdg/raydriver")


def test_linux_default(monkeypatch):
    monkeypatch.setattr(sys, "platform", "linux")
    monkeypatch.delenv("XDG_CONFIG_HOME", raising=False)
    assert paths.config_dir() == Path.home() / ".config" / "raydriver"


def test_macos(monkeypatch):
    monkeypatch.setattr(sys, "platform", "darwin")
    assert (
        paths.config_dir()
        == Path.home() / "Library" / "Application Support" / "raydriver"
    )


def test_windows(monkeypatch):
    monkeypatch.setattr(sys, "platform", "win32")
    monkeypatch.setenv("APPDATA", "C:\\Users\\x\\AppData\\Roaming")
    assert (
        paths.config_dir()
        == Path("C:\\Users\\x\\AppData\\Roaming") / "raydriver"
    )


def test_default_config_path():
    assert paths.default_config_path().name == "daemon.json"
