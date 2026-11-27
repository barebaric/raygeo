"""Tests for the debug CLI commands against a live daemon."""

import argparse
import json

import pytest_asyncio

from raygeo.driver.cli import debug
from raygeo.driver.cli.main import amain
from raygeo.driver.remote.daemon import Daemon, DaemonConfig


def ns(**kwargs):
    base = {"json": True, "config": None, "token": None, "tls_ca": None}
    base.update(kwargs)
    return argparse.Namespace(**base)


@pytest_asyncio.fixture
async def daemon_fixture(tmp_path):
    sessions = [
        {
            "protocol": "GrblSerialDriver",
            "config": {"port": "/dev/ttyUSB0", "baudrate": 115200},
        }
    ]
    daemon = Daemon(DaemonConfig(port=0, sessions=sessions))
    server = await daemon.start()
    port = server.sockets[0].getsockname()[1]
    config_path = tmp_path / "daemon.json"
    config_path.write_text(
        json.dumps({"host": "127.0.0.1", "port": port, "sessions": sessions})
    )
    try:
        yield port, config_path
    finally:
        await daemon.stop()


async def test_scan_json(capsys):
    assert await debug.cmd_scan(ns(network=False)) == 0
    data = json.loads(capsys.readouterr().out)
    assert isinstance(data["ports"], list)
    assert data["daemons"] == []


async def test_info_daemon(daemon_fixture, capsys):
    port, _ = daemon_fixture
    assert await debug.cmd_info(ns(target=f"127.0.0.1:{port}")) == 0
    data = json.loads(capsys.readouterr().out)
    assert len(data["sessions"]) == 1
    assert data["sessions"][0]["protocol"] == "GrblSerialDriver"
    assert data["welcome"]["protocol"] == 1


async def test_info_serial_missing(capsys):
    assert await debug.cmd_info(ns(target="/dev/nonexistent-raydriver")) == 1


@pytest_asyncio.fixture
async def token_daemon_fixture(tmp_path):
    token = "test-token-123"
    daemon = Daemon(DaemonConfig(port=0, token=token))
    server = await daemon.start()
    port = server.sockets[0].getsockname()[1]
    config_path = tmp_path / "daemon.json"
    config_path.write_text(
        json.dumps({"host": "127.0.0.1", "port": port, "token": token})
    )
    try:
        yield port, config_path, token
    finally:
        await daemon.stop()


async def test_info_daemon_token_from_config(token_daemon_fixture, capsys):
    # info must read the token from the config like list/connection
    # do; it used to fail with "daemon authentication failed".
    port, config_path, _ = token_daemon_fixture
    result = await debug.cmd_info(
        ns(target=f"127.0.0.1:{port}", config=str(config_path))
    )
    assert result == 0
    data = json.loads(capsys.readouterr().out)
    assert data["welcome"]["auth_required"] is True


async def test_info_daemon_explicit_token(token_daemon_fixture, capsys):
    port, _, token = token_daemon_fixture
    result = await debug.cmd_info(ns(target=f"127.0.0.1:{port}", token=token))
    assert result == 0
    data = json.loads(capsys.readouterr().out)
    assert data["welcome"]["auth_required"] is True


async def test_info_daemon_url_token(token_daemon_fixture, capsys):
    port, _, token = token_daemon_fixture
    result = await debug.cmd_info(
        ns(target=f"raydriver://127.0.0.1:{port}#{token}")
    )
    assert result == 0
    data = json.loads(capsys.readouterr().out)
    assert data["welcome"]["auth_required"] is True


async def test_list_configured(daemon_fixture, capsys):
    _, config_path = daemon_fixture
    assert await debug.cmd_list(ns(config=str(config_path))) == 0
    data = json.loads(capsys.readouterr().out)
    assert data["daemon"] == "running"
    assert len(data["live"]) == 1
    assert data["live"][0]["connection_status"] == "DISCONNECTED"
    assert len(data["configured"]) == 1


async def test_list_daemon_stopped(tmp_path, capsys):
    config_path = tmp_path / "daemon.json"
    config_path.write_text(
        json.dumps(
            {
                "host": "127.0.0.1",
                "port": 1,
                "sessions": [
                    {
                        "protocol": "GrblSerialDriver",
                        "config": {"port": "/dev/ttyUSB0"},
                    }
                ],
            }
        )
    )
    assert await debug.cmd_list(ns(config=str(config_path))) == 0
    data = json.loads(capsys.readouterr().out)
    assert data["daemon"] == "daemon not running"
    assert data["live"] == []


async def test_connection(daemon_fixture, capsys):
    _, config_path = daemon_fixture
    assert await debug.cmd_connection(ns(config=str(config_path))) == 0
    data = json.loads(capsys.readouterr().out)
    assert len(data["clients"]) >= 1
    assert data["clients"][0]["address"].startswith("127.0.0.1:")


async def test_connection_daemon_stopped(tmp_path, capsys):
    config_path = tmp_path / "daemon.json"
    config_path.write_text(
        json.dumps({"host": "127.0.0.1", "port": 1, "sessions": []})
    )
    assert await debug.cmd_connection(ns(config=str(config_path))) == 1


# Entry-point regressions: these go through amain (the real CLI
# path), unlike the cmd_* tests above which bypass argument parsing.


async def test_amain_scan_runs_without_log_file(capsys):
    # amain used to read args.log_file unconditionally, but the debug
    # subcommands do not define --log-file (AttributeError on scan).
    assert await amain(["scan", "--json"]) == 0
    data = json.loads(capsys.readouterr().out)
    assert isinstance(data["ports"], list)


async def test_amain_list_without_config_file(tmp_path, capsys):
    # No config yet (before `daemon --setup`): list must not crash on
    # a missing file; it falls back to defaults.
    missing = tmp_path / "missing.json"
    assert await amain(["list", "--json", "--config", str(missing)]) == 0
    data = json.loads(capsys.readouterr().out)
    assert data["configured"] == []
    assert "daemon" in data


async def test_amain_connection_without_config_file(tmp_path, capsys):
    # Missing config must not crash; with no daemon at the default
    # endpoint this is a graceful error (1), or 0 if one is running.
    missing = tmp_path / "missing.json"
    result = await amain(["connection", "--json", "--config", str(missing)])
    assert result in (0, 1)
