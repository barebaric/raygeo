"""End-to-end tests for the ``raydriver discover`` CLI command.

The serial side uses a real pty with a pre-written banner; the TCP
side runs a local listener speaking grblHAL-style build info on a
LAN interface, found through the engine's subnet scan.
"""

import argparse
import asyncio
import json
import os
import sys

import pytest

from raygeo.driver.cli import debug
from raygeo.driver.core.discovery import local_subnets

try:
    import tty
except ImportError:  # Windows has no termios/tty
    tty = None

requires_pty = pytest.mark.skipif(
    not hasattr(os, "openpty") or tty is None or sys.platform == "darwin",
    reason="requires a POSIX pty; macOS ptys do not deliver master "
    "writes to a serialport reader on CI runners",
)


def ns(**kwargs):
    base = {
        "json": True,
        "ports": None,
        "baud": None,
        "tcp": None,
        "probe": None,
        "match": None,
        "timeout": None,
    }
    base.update(kwargs)
    return argparse.Namespace(**base)


async def test_discover_no_devices(capsys):
    assert (
        await debug.cmd_discover(ns(ports=["/dev/raydriver-does-not-exist"]))
        == 0
    )
    assert json.loads(capsys.readouterr().out) == []


@requires_pty
async def test_discover_serial_pty(capsys):
    master, slave = os.openpty()
    if tty is not None:
        tty.setraw(slave)
    port = os.ttyname(slave)
    os.write(master, b"Grbl 1.1f ['$' for help]\r\n")
    try:
        assert await debug.cmd_discover(ns(ports=[port], baud=[115200])) == 0
    finally:
        os.close(slave)
        os.close(master)
    data = json.loads(capsys.readouterr().out)
    assert len(data) == 1
    device = data[0]
    assert device["params"]["port"] == port
    assert device["params"]["baudrate"] == 115200
    assert device["banner"] == "Grbl 1.1f ['$' for help]"


async def test_discover_tcp_subnet(capsys, monkeypatch):
    subnets = local_subnets()
    if not subnets:
        pytest.skip("no non-loopback IPv4 interface")
    host = subnets[0][0]
    # Machines with many virtual interfaces (docker bridges, ...)
    # need more than the default budget to walk every subnet.
    from raygeo.driver.core.discovery import engine

    monkeypatch.setattr(engine, "SUBNET_SCAN_TIMEOUT", 120.0)

    async def handle(reader, writer):
        try:
            await asyncio.wait_for(reader.readuntil(b"\n"), timeout=2.0)
            writer.write(b"[VER:1.0.15,20240923:]\r\n")
            await writer.drain()
        except (asyncio.TimeoutError, ConnectionError):
            pass
        finally:
            writer.close()

    server = await asyncio.start_server(handle, host, 0)
    port = server.sockets[0].getsockname()[1]
    try:
        assert (
            await debug.cmd_discover(
                ns(
                    ports=[],
                    tcp=port,
                    probe="$I\n",
                    match="[VER:",
                )
            )
            == 0
        )
    finally:
        server.close()
        await server.wait_closed()
    data = json.loads(capsys.readouterr().out)
    assert len(data) == 1
    device = data[0]
    assert device["params"] == {"host": host, "port": port}
    assert device["banner"] == "[VER:1.0.15,20240923:]"
