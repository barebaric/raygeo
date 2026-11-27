"""Platform-aware user paths (stdlib only).

Linux/BSD use the XDG config home, macOS uses
``~/Library/Application Support`` and Windows uses ``%APPDATA%``,
so the daemon config and service definitions land in the
conventional per-user location on every platform.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

APP_NAME = "raydriver"


def config_dir() -> Path:
    """The per-user configuration directory for raygeo.driver."""
    if sys.platform == "win32":
        base = os.environ.get("APPDATA") or str(
            Path.home() / "AppData" / "Roaming"
        )
        return Path(base) / APP_NAME
    if sys.platform == "darwin":
        return Path.home() / "Library" / "Application Support" / APP_NAME
    base = os.environ.get("XDG_CONFIG_HOME") or str(Path.home() / ".config")
    return Path(base) / APP_NAME


def default_config_path() -> Path:
    """The default daemon config file location."""
    return config_dir() / "daemon.json"
