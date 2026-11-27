"""Tests for the Rust daemon camera query (including token and TLS).

Runs against the real Python daemon so the wire protocol, auth and
TLS handshake are exercised end-to-end.
"""

from pathlib import Path

import pytest

from raygeo.driver.core.discovery import daemon as discovery_daemon
from raygeo.driver.remote.daemon import Daemon, DaemonConfig

CERTS = Path(__file__).parents[1] / "certs"
CA = CERTS / "analyzer-test-cert.pem"
CERT = CERTS / "analyzer-test-server-cert.pem"
KEY = CERTS / "analyzer-test-server-key.pem"

CAMERAS = [
    {
        "name": "Enclosure",
        "uri": "http://pi.local:8080/stream",
        "type": "http_stream",
    },
    {
        "name": "Bed",
        "uri": "http://pi.local:8080/bed",
        "type": "http_snapshot",
    },
]


async def start_daemon(**kwargs):
    cameras = kwargs.pop("cameras", CAMERAS)
    daemon = Daemon(DaemonConfig(port=0, cameras=list(cameras), **kwargs))
    server = await daemon.start()
    port = server.sockets[0].getsockname()[1]
    return daemon, port


async def test_list_cameras_plain():
    daemon, port = await start_daemon()
    try:
        cameras = await discovery_daemon.list_cameras("127.0.0.1", port)
        assert cameras == CAMERAS
    finally:
        await daemon.stop()


async def test_list_cameras_empty():
    daemon, port = await start_daemon(cameras=[])
    try:
        assert await discovery_daemon.list_cameras("127.0.0.1", port) == []
    finally:
        await daemon.stop()


async def test_list_cameras_token_on_loopback():
    daemon, port = await start_daemon(token="secret")
    try:
        cameras = await discovery_daemon.list_cameras(
            "127.0.0.1", port, token="secret"
        )
        assert cameras == CAMERAS
    finally:
        await daemon.stop()


async def test_list_cameras_token_without_tls_refused_off_loopback():
    with pytest.raises(ConnectionError, match="without TLS"):
        await discovery_daemon.list_cameras("192.0.2.1", 8765, token="secret")


async def test_list_cameras_missing_token():
    daemon, port = await start_daemon(token="secret")
    try:
        with pytest.raises(ConnectionError, match="requires authentication"):
            await discovery_daemon.list_cameras("127.0.0.1", port)
    finally:
        await daemon.stop()


async def test_list_cameras_wrong_token():
    daemon, port = await start_daemon(token="secret")
    try:
        with pytest.raises(ConnectionError, match="authentication failed"):
            await discovery_daemon.list_cameras(
                "127.0.0.1", port, token="wrong"
            )
    finally:
        await daemon.stop()


async def test_list_cameras_tls_roundtrip():
    daemon, port = await start_daemon(
        token="secret",
        tls_cert=str(CERT),
        tls_key=str(KEY),
    )
    try:
        cameras = await discovery_daemon.list_cameras(
            "localhost",
            port,
            token="secret",
            tls_ca=str(CA),
        )
        assert cameras == CAMERAS
    finally:
        await daemon.stop()


async def test_list_cameras_tls_hostname_override():
    daemon, port = await start_daemon(
        token="secret",
        tls_cert=str(CERT),
        tls_key=str(KEY),
    )
    try:
        cameras = await discovery_daemon.list_cameras(
            "127.0.0.1",
            port,
            token="secret",
            tls_ca=str(CA),
            tls_hostname="localhost",
        )
        assert cameras == CAMERAS
    finally:
        await daemon.stop()
