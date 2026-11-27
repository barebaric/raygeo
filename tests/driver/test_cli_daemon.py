"""Tests for the daemon CLI, config handling and mDNS announce."""

import json
import os
import socket
import subprocess
import sys
import uuid

import pytest

from raygeo.driver import service
from raygeo.driver.cli.daemon import (
    add_device,
    config_from_dict,
    load_config,
    remove_device,
    write_setup_config,
)
from raygeo.driver.cli.main import amain
from raygeo.driver.remote import mdns
from raygeo.driver.remote.daemon import DaemonConfig


def test_setup_writes_token_config(tmp_path):
    path = tmp_path / "sub" / "daemon.json"
    token = write_setup_config(path)
    assert path.exists()
    # Windows maps chmod onto ACLs, not POSIX mode bits.
    if os.name != "nt":
        assert (path.stat().st_mode & 0o777) == 0o600
    data = json.loads(path.read_text())
    assert data["token"] == token
    assert data["host"] == "127.0.0.1"
    with pytest.raises(FileExistsError):
        write_setup_config(path)


def test_config_from_dict_defaults():
    config = config_from_dict({})
    defaults = DaemonConfig()
    assert config.host == defaults.host
    assert config.port == defaults.port
    assert config.keepalive_interval == defaults.keepalive_interval
    assert config.keepalive_misses == defaults.keepalive_misses
    assert config.mdns is True


def test_config_from_dict_overrides():
    config = config_from_dict(
        {
            "host": "0.0.0.0",
            "port": 9999,
            "token": "t",
            "tls_cert": "c.pem",
            "tls_key": "k.pem",
            "keepalive_interval": 1.5,
            "keepalive_misses": 5,
            "lease_grace": 2.5,
            "ring_size": 50,
            "cameras": [
                {
                    "name": "cam",
                    "uri": "http://x/stream",
                    "type": "http_stream",
                }
            ],
            "sessions": [
                {
                    "protocol": "GrblSerialDriver",
                    "config": {"port": "/dev/ttyUSB0"},
                }
            ],
            "mdns": True,
        }
    )
    assert config.host == "0.0.0.0"
    assert config.port == 9999
    assert config.token == "t"
    assert config.tls_cert == "c.pem"
    assert config.keepalive_interval == 1.5
    assert config.keepalive_misses == 5
    assert config.lease_grace == 2.5
    assert config.ring_size == 50
    assert config.cameras[0]["name"] == "cam"
    assert config.sessions[0]["protocol"] == "GrblSerialDriver"
    assert config.mdns is True


def test_load_config(tmp_path):
    path = tmp_path / "daemon.json"
    path.write_text(json.dumps({"port": 1234}))
    assert load_config(path).port == 1234


def test_add_device_creates_config(tmp_path):
    path = tmp_path / "sub" / "daemon.json"
    message = add_device(path, "/dev/ttyUSB0", "GrblSerialDriver", 115200)
    assert "token:" in message
    assert "added GrblSerialDriver /dev/ttyUSB0" in message
    data = json.loads(path.read_text())
    assert data["token"]
    assert data["host"] == "127.0.0.1"
    assert data["sessions"] == [
        {
            "protocol": "GrblSerialDriver",
            "config": {"port": "/dev/ttyUSB0", "baudrate": 115200},
        }
    ]
    if os.name != "nt":
        assert (path.stat().st_mode & 0o777) == 0o600


def test_add_device_preserves_config_and_upserts(tmp_path):
    path = tmp_path / "daemon.json"
    path.write_text(
        json.dumps(
            {
                "host": "127.0.0.1",
                "port": 9000,
                "token": "t",
                "sessions": [
                    {
                        "protocol": "GrblSerialDriver",
                        "config": {
                            "port": "/dev/ttyUSB0",
                            "baudrate": 9600,
                        },
                    }
                ],
            }
        )
    )
    message = add_device(path, "/dev/ttyUSB0", "GrblSerialDriver", 115200)
    assert "updated" in message
    data = json.loads(path.read_text())
    assert data["port"] == 9000
    assert data["token"] == "t"
    assert len(data["sessions"]) == 1
    assert data["sessions"][0]["config"]["baudrate"] == 115200
    add_device(path, "/dev/ttyACM0", "MarlinSerialDriver", 250000)
    data = json.loads(path.read_text())
    assert len(data["sessions"]) == 2
    assert data["sessions"][1]["protocol"] == "MarlinSerialDriver"


def test_add_device_unknown_protocol(tmp_path):
    with pytest.raises(ValueError, match="unknown driver"):
        add_device(tmp_path / "daemon.json", "/dev/ttyUSB0", "nope", 115200)


def test_remove_device(tmp_path):
    path = tmp_path / "daemon.json"
    add_device(path, "/dev/ttyUSB0", "GrblSerialDriver", 115200)
    assert "removed /dev/ttyUSB0" in remove_device(path, "/dev/ttyUSB0")
    assert json.loads(path.read_text())["sessions"] == []
    with pytest.raises(ValueError, match="no configured device"):
        remove_device(path, "/dev/ttyUSB0")


def test_remove_device_no_config(tmp_path):
    with pytest.raises(ValueError, match="no config"):
        remove_device(tmp_path / "missing.json", "/dev/ttyUSB0")


async def test_amain_daemon_add_remove(tmp_path, capsys):
    config = tmp_path / "daemon.json"
    result = await amain(
        ["daemon", "add", "/dev/ttyUSB0", "--config", str(config)]
    )
    assert result == 0
    out = capsys.readouterr().out
    assert "added GrblSerialDriver /dev/ttyUSB0" in out
    assert "token:" in out
    result = await amain(
        ["daemon", "remove", "/dev/ttyUSB0", "--config", str(config)]
    )
    assert result == 0
    assert json.loads(config.read_text())["sessions"] == []


async def test_amain_daemon_add_requires_port(tmp_path, capsys):
    result = await amain(
        ["daemon", "add", "--config", str(tmp_path / "d.json")]
    )
    assert result == 2


async def test_amain_daemon_port_without_action(tmp_path):
    # argparse rejects a bare port: the first positional is the action.
    with pytest.raises(SystemExit) as exc:
        await amain(
            ["daemon", "/dev/ttyUSB0", "--config", str(tmp_path / "d.json")]
        )
    assert exc.value.code == 2


async def test_amain_daemon_status_without_service(
    tmp_path, capsys, monkeypatch
):
    # Foreground users have no service: status must still show the
    # token and probe the daemon instead of erroring out.
    config = tmp_path / "daemon.json"
    add_device(config, "/dev/ttyUSB0", "GrblSerialDriver", 115200)
    data = json.loads(config.read_text())
    data["port"] = 1
    config.write_text(json.dumps(data))
    capsys.readouterr()

    def not_installed():
        raise service.ServiceError("unit not found")

    monkeypatch.setattr(service, "status", not_installed)
    result = await amain(["daemon", "status", "--config", str(config)])
    assert result == 0
    out = capsys.readouterr().out
    assert "service: not installed" in out
    assert "token:" in out
    assert "daemon: not reachable" in out


def test_cli_setup_and_serve(tmp_path):
    """End-to-end: python -m raygeo.driver daemon --setup, then serve."""
    config = tmp_path / "daemon.json"
    setup = subprocess.run(
        [
            sys.executable,
            "-m",
            "raygeo.driver",
            "daemon",
            "--setup",
            "--config",
            str(config),
        ],
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert setup.returncode == 0, setup.stderr
    assert "openssl" in setup.stdout
    assert config.exists()

    data = json.loads(config.read_text())
    data["port"] = 0
    config.write_text(json.dumps(data))
    proc = subprocess.Popen(
        [
            sys.executable,
            "-m",
            "raygeo.driver",
            "daemon",
            "--config",
            str(config),
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        stdout = proc.stdout
        assert stdout is not None
        # readline is portable (select on pipes is not, on Windows);
        # the daemon flushes the listening line, and a dead process
        # yields EOF. pytest-timeout backstops a silent hang.
        line = stdout.readline()
        assert "listening" in line, "daemon did not announce its listener"
        port = int(line.strip().rsplit(":", 1)[1])
        with socket.create_connection(("127.0.0.1", port), timeout=5) as sock:
            prefix = sock.recv(4)
            assert len(prefix) == 4
            size = int.from_bytes(prefix, "big")
            payload = b""
            while len(payload) < size:
                payload += sock.recv(size - len(payload))
            welcome = json.loads(payload)
        assert welcome["type"] == "welcome"
        assert welcome["payload"]["auth_required"] is True
    finally:
        proc.terminate()
        proc.wait(timeout=10)


async def test_mdns_announce_registers_service(monkeypatch):
    pytest.importorskip("zeroconf")
    # A unique service name so the test does not collide with a daemon
    # already announcing on this machine.
    config = DaemonConfig(
        token="t",
        cameras=[{"name": "c", "uri": "u", "type": "http_stream"}],
    )
    announcer = await mdns.announce(
        config,
        8765,
        name=f"raydriver-test-{uuid.uuid4().hex}",
    )
    assert announcer is not None
    try:
        assert announcer.info.port == 8765
        # A loopback bind advertises loopback, not the hostname address.
        assert announcer.info.parsed_addresses() == ["127.0.0.1"]
        assert announcer.info.properties[b"requires_auth"] == b"true"
        assert announcer.info.properties[b"tls"] == b"false"
        assert announcer.info.properties[b"cameras"] == b"1"
    finally:
        await announcer.close()


def test_mdns_announce_address_matches_bind():
    assert (
        mdns._announce_address(DaemonConfig(host="127.0.0.1")) == "127.0.0.1"
    )
    assert (
        mdns._announce_address(DaemonConfig(host="localhost")) == "127.0.0.1"
    )
    assert isinstance(
        mdns._announce_address(DaemonConfig(host="0.0.0.0")), str
    )
    assert (
        mdns._announce_address(DaemonConfig(host="192.168.1.50"))
        == "192.168.1.50"
    )


def test_mdns_service_name_short_hostname():
    assert (
        mdns._service_name_for("myhost")
        == "raydriver-myhost._raydriver._tcp.local."
    )


def test_mdns_service_name_long_hostname():
    # CI runners can have hostnames that overflow the 63-char DNS
    # label limit; the instance name must stay valid.
    label = mdns._service_name_for("a" * 100).split(".")[0]
    assert len(label) <= 63
    assert label.startswith("raydriver-")
