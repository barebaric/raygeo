"""Tests for raygeo.driver.analyzer.listener.

These drive the real network code paths — real sockets, real TLS
handshakes, real token checks — and assert on their outcomes;
printing failures without assertions is what lets a broken
security path ship.
"""

import asyncio
from pathlib import Path

import pytest
from conftest import server_port

from raygeo.driver.analyzer.listener import (
    TOKEN_REJECTED,
    InsecureListenerError,
    connect_authenticated,
    is_loopback,
    serve,
    validate_bind_policy,
)

CERTS = Path(__file__).parents[1] / "certs"
CERT = CERTS / "analyzer-test-cert.pem"
KEY = CERTS / "analyzer-test-key.pem"


async def echo_handler(reader, writer):
    while True:
        data = await reader.read(1024)
        if not data:
            return
        writer.write(data.upper())
        await writer.drain()


def test_is_loopback():
    assert is_loopback("127.0.0.1")
    assert is_loopback("localhost")
    assert is_loopback("::1")
    assert not is_loopback("192.168.1.5")
    assert not is_loopback("example.org")


def test_policy_allows_loopback_without_auth():
    validate_bind_policy("127.0.0.1", None, None)
    validate_bind_policy("localhost", None, None)
    validate_bind_policy("::1", None, None)


@pytest.mark.parametrize("host", ["0.0.0.0", "::", "*", "", "10.0.0.5"])
def test_policy_rejects_exposed_without_auth(host):
    with pytest.raises(InsecureListenerError):
        validate_bind_policy(host, None, None)
    with pytest.raises(InsecureListenerError):
        validate_bind_policy(host, "secret", None)
    with pytest.raises(InsecureListenerError):
        validate_bind_policy(host, None, str(CERT))
    validate_bind_policy(host, "secret", str(CERT))


async def test_server_requires_tls_key_with_cert():
    with pytest.raises(InsecureListenerError):
        await serve(echo_handler, host="127.0.0.1", tls_cert=str(CERT))


async def test_plain_loopback_roundtrip():
    server = await serve(echo_handler)
    port = server_port(server)
    reader, writer = await connect_authenticated("127.0.0.1", port)
    writer.write(b"hello")
    await writer.drain()
    assert await reader.readexactly(5) == b"HELLO"
    writer.close()
    server.close()
    await server.wait_closed()


async def test_token_accepted():
    server = await serve(echo_handler, token="secret")
    port = server_port(server)
    reader, writer = await connect_authenticated(
        "127.0.0.1", port, token="secret"
    )
    writer.write(b"hello")
    await writer.drain()
    assert await reader.readexactly(5) == b"HELLO"
    writer.close()
    server.close()
    await server.wait_closed()


async def test_token_rejected():
    handled = []

    async def handler(reader, writer):
        handled.append(True)

    server = await serve(handler, token="secret")
    port = server_port(server)
    reader, writer = await connect_authenticated(
        "127.0.0.1", port, token="wrong"
    )
    assert await reader.readuntil(b"\n") == TOKEN_REJECTED
    assert await reader.read() == b""
    writer.close()
    server.close()
    await server.wait_closed()
    assert not handled


async def test_client_refuses_to_send_token_unprompted():
    async def silent_greeting(reader, writer):
        writer.write(b"welcome\n")
        await writer.drain()
        writer.close()

    rogue = await asyncio.start_server(silent_greeting, "127.0.0.1", 0)
    port = server_port(rogue)
    try:
        reader, writer = await connect_authenticated(
            "127.0.0.1", port, token="secret"
        )
    except ConnectionError as err:
        assert "did not ask for a token" in str(err)
    else:
        writer.close()
        raise AssertionError("expected ConnectionError")
    finally:
        rogue.close()
        await rogue.wait_closed()


async def test_tls_roundtrip():
    server = await serve(
        echo_handler, token="secret", tls_cert=str(CERT), tls_key=str(KEY)
    )
    port = server_port(server)
    reader, writer = await connect_authenticated(
        "127.0.0.1",
        port,
        token="secret",
        tls_ca=str(CERT),
        tls_hostname="localhost",
    )
    writer.write(b"hello")
    await writer.drain()
    assert await reader.readexactly(5) == b"HELLO"
    writer.close()
    server.close()
    await server.wait_closed()


async def test_tls_rejects_plaintext_client():
    server = await serve(echo_handler, tls_cert=str(CERT), tls_key=str(KEY))
    port = server_port(server)
    reader, writer = await asyncio.open_connection("127.0.0.1", port)
    writer.write(b"hello")
    await writer.drain()
    # The TLS server drops the handshake; no application data ever
    # comes back and the connection ends.
    data = await asyncio.wait_for(reader.read(), timeout=10.0)
    assert data == b""
    writer.close()
    server.close()
    await server.wait_closed()
