import asyncio

import pytest

from raygeo.driver.core.discovery import mdns_channel
from raygeo.driver.core.discovery.fingerprints import (
    HttpFingerprint,
    TcpFingerprint,
)
from raygeo.driver.core.discovery.mdns_channel import find_network_devices
from raygeo.driver.core.discovery.mdns_scan import MDNSService
from raygeo.driver.core.discovery.spec import DiscoverySpec, MdnsRecognizer


async def _start_server(handler):
    server = await asyncio.start_server(handler, "127.0.0.1", 0)
    port = server.sockets[0].getsockname()[1]
    return server, port


async def _stop_server(server):
    server.close()
    await server.wait_closed()


def _http_handler(responses):
    """An HTTP server answering every request with the next
    ``(status, reason, body)`` tuple from *responses*."""

    async def handler(reader, writer):
        try:
            await reader.readuntil(b"\r\n\r\n")
            status, reason, body = responses[0]
            writer.write(
                f"HTTP/1.1 {status} {reason}\r\n"
                f"Content-Type: text/plain\r\n"
                f"Content-Length: {len(body)}\r\n"
                f"Connection: close\r\n\r\n".encode()
                + body
            )
            await writer.drain()
        finally:
            writer.close()

    return handler


@pytest.mark.asyncio
async def test_tcp_fingerprint_probes_with_command():
    """A command/reply probe claims a listener that answers with the
    matching line."""
    seen = []

    async def handler(reader, writer):
        try:
            seen.append(await reader.readline())
            writer.write(b"[VER:1.1h.20240101:]\r\n[OPT:V,15,128]\r\n")
            await writer.drain()
            await reader.read()
        finally:
            writer.close()

    server, port = await _start_server(handler)
    fingerprint = TcpFingerprint(
        probe_command="$I\n",
        match_prefix="[VER:",
        banner_wait=0.1,
        timeout=1.0,
    )
    try:
        matched = await fingerprint("127.0.0.1", port)
    finally:
        await _stop_server(server)
    assert matched
    assert seen == [b"$I\n"]


@pytest.mark.asyncio
async def test_tcp_fingerprint_accepts_unsolicited_banner():
    """A listener that announces itself on connect is claimed without
    sending the probe command."""

    async def handler(reader, writer):
        try:
            writer.write(b"[VER:1.1h.20240101:]\r\n")
            await writer.drain()
            await reader.read()
        finally:
            writer.close()

    server, port = await _start_server(handler)
    fingerprint = TcpFingerprint(
        probe_command="$I\n",
        match_prefix="[VER:",
        banner_wait=0.5,
        timeout=1.0,
    )
    try:
        matched = await fingerprint("127.0.0.1", port)
    finally:
        await _stop_server(server)
    assert matched


@pytest.mark.asyncio
async def test_tcp_fingerprint_rejects_other_listener():
    """An unrelated service answering the probe is not claimed."""

    async def handler(reader, writer):
        try:
            await reader.readline()
            writer.write(b"Welcome to the telnet console\r\n")
            await writer.drain()
            await reader.read()
        finally:
            writer.close()

    server, port = await _start_server(handler)
    fingerprint = TcpFingerprint(
        probe_command="$I\n",
        match_prefix="[VER:",
        banner_wait=0.1,
        timeout=0.5,
    )
    try:
        matched = await fingerprint("127.0.0.1", port)
    finally:
        await _stop_server(server)
    assert matched is False


@pytest.mark.asyncio
async def test_tcp_fingerprint_claim_via_mdns_channel(monkeypatch):
    """A candidate offered by the network channel is confirmed by the
    same probe and becomes a discovered device."""

    async def handler(reader, writer):
        try:
            await reader.readline()
            writer.write(b"[VER:1.1h.20240101:]\r\n")
            await writer.drain()
            await reader.read()
        finally:
            writer.close()

    server, port = await _start_server(handler)

    async def fake_scan(service_types):
        return [
            MDNSService(
                service_type="_telnet._tcp",
                name="grblHAL",
                host="127.0.0.1",
                port=port,
                server="grblhal.local",
                txt={},
            )
        ]

    class TelnetDriver:
        DISCOVERY = DiscoverySpec(
            mdns=MdnsRecognizer(
                fingerprint_services=("_telnet._tcp",),
                fingerprint=TcpFingerprint(
                    probe_command="$I\n",
                    match_prefix="[VER:",
                    banner_wait=0.1,
                    timeout=1.0,
                ),
            )
        )
        label = "grblHAL (Telnet)"

    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    try:
        devices = await find_network_devices([TelnetDriver])
    finally:
        await _stop_server(server)
    assert len(devices) == 1
    assert devices[0].params == {"host": "127.0.0.1", "port": port}


@pytest.mark.asyncio
async def test_http_fingerprint_matches_body():
    """A firmware-info body naming GRBL-family firmware is claimed,
    regardless of the match's case."""

    async def handler(reader, writer):
        try:
            await reader.readuntil(b"\r\n\r\n")
            body = b"FluidNC v3.6.1 [ESP800]\r\nok\r\n"
            writer.write(
                b"HTTP/1.1 200 OK\r\n"
                b"Content-Type: text/plain\r\n"
                b"Content-Length: 27\r\n"
                b"Connection: close\r\n\r\n" + body
            )
            await writer.drain()
        finally:
            writer.close()

    server, port = await _start_server(handler)
    fingerprint = HttpFingerprint(
        "/command?plain=%5BESP800%5D&PAGEID=",
        r"\b(fluidnc|esp3d|grblhal|grbl)\b",
        timeout=1.0,
    )
    try:
        matched = await fingerprint("127.0.0.1", port)
    finally:
        await _stop_server(server)
    assert matched


@pytest.mark.asyncio
async def test_http_fingerprint_rejects_other_body():
    """An unrelated HTTP endpoint is not claimed."""

    async def handler(reader, writer):
        try:
            await reader.readuntil(b"\r\n\r\n")
            body = b"<html>Welcome to the router admin</html>"
            writer.write(
                b"HTTP/1.1 200 OK\r\n"
                b"Content-Type: text/html\r\n"
                b"Content-Length: 40\r\n"
                b"Connection: close\r\n\r\n" + body
            )
            await writer.drain()
        finally:
            writer.close()

    server, port = await _start_server(handler)
    fingerprint = HttpFingerprint(
        "/command?plain=%5BESP800%5D&PAGEID=",
        r"\b(fluidnc|esp3d|grblhal|grbl)\b",
        timeout=1.0,
    )
    try:
        matched = await fingerprint("127.0.0.1", port)
    finally:
        await _stop_server(server)
    assert matched is False


@pytest.mark.asyncio
async def test_http_fingerprint_rejects_error_status():
    """A non-200 answer is not a match."""

    async def handler(reader, writer):
        try:
            await reader.readuntil(b"\r\n\r\n")
            body = b"FluidNC"
            writer.write(
                b"HTTP/1.1 404 Not Found\r\n"
                b"Content-Length: 7\r\n"
                b"Connection: close\r\n\r\n" + body
            )
            await writer.drain()
        finally:
            writer.close()

    server, port = await _start_server(handler)
    fingerprint = HttpFingerprint("/x", r"fluidnc", timeout=1.0)
    try:
        matched = await fingerprint("127.0.0.1", port)
    finally:
        await _stop_server(server)
    assert matched is False


@pytest.mark.asyncio
async def test_http_fingerprint_honors_max_bytes():
    """A match beyond the retained body window is not seen."""

    async def handler(reader, writer):
        try:
            await reader.readuntil(b"\r\n\r\n")
            body = b"X" * 40 + b"grbl"
            writer.write(
                b"HTTP/1.1 200 OK\r\n"
                b"Content-Length: 44\r\n"
                b"Connection: close\r\n\r\n" + body
            )
            await writer.drain()
        finally:
            writer.close()

    server, port = await _start_server(handler)
    narrow = HttpFingerprint("/x", r"grbl", timeout=1.0, max_bytes=10)
    wide = HttpFingerprint("/x", r"grbl", timeout=1.0)
    try:
        assert await narrow("127.0.0.1", port) is False
        assert await wide("127.0.0.1", port)
    finally:
        await _stop_server(server)


@pytest.mark.asyncio
async def test_http_fingerprint_dead_endpoint():
    """A connection failure is not a match."""

    fingerprint = HttpFingerprint("/x", r"grbl", timeout=0.5)
    assert await fingerprint("127.0.0.1", 1) is False


def test_http_fingerprint_rejects_invalid_regex():
    with pytest.raises(ValueError):
        HttpFingerprint("/x", "(unclosed")


@pytest.mark.asyncio
async def test_http_fingerprint_claim_via_mdns_channel(monkeypatch):
    """A generic `_http._tcp` candidate is confirmed by the HTTP
    probe and becomes a discovered device."""

    async def handler(reader, writer):
        try:
            await reader.readuntil(b"\r\n\r\n")
            body = b"[ESP800] FluidNC v3.6.1"
            writer.write(
                b"HTTP/1.1 200 OK\r\n"
                b"Content-Length: 24\r\n"
                b"Connection: close\r\n\r\n" + body
            )
            await writer.drain()
        finally:
            writer.close()

    server, port = await _start_server(handler)

    async def fake_scan(service_types):
        return [
            MDNSService(
                service_type="_http._tcp",
                name="fluidnc.local",
                host="127.0.0.1",
                port=port,
                server="fluidnc.local",
                txt={},
            )
        ]

    class NetworkDriver:
        DISCOVERY = DiscoverySpec(
            mdns=MdnsRecognizer(
                fingerprint=HttpFingerprint(
                    "/command?plain=%5BESP800%5D&PAGEID=",
                    r"\b(fluidnc|esp3d|grblhal|grbl)\b",
                    timeout=1.0,
                ),
            )
        )
        label = "FluidNC / ESP3D (Web API)"

    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    try:
        devices = await find_network_devices([NetworkDriver])
    finally:
        await _stop_server(server)
    assert len(devices) == 1
    assert devices[0].params == {"host": "127.0.0.1", "port": port}
