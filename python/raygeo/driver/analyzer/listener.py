"""Security-hardened TCP endpoints for the analyzer.

Diagnostic endpoints are easy to leave unsecured, and
distributions reject packages that ship unsecured network servers,
so the analyzer enforces its policy before any socket opens:

- listeners bind loopback only, without authentication, by default;
- binding a non-loopback or wildcard address requires BOTH a token
  and TLS — there is no override;
- authentication that was explicitly configured is never dropped,
  not even on loopback.

Tests must drive these real code paths (real sockets, real
handshakes) with assertions; printing failures without failing the
test hides the breakage it should have caught.
"""

from __future__ import annotations

import asyncio
import hmac
import ipaddress
import ssl

from raygeo.driver.errors import InsecureListenerError
from raygeo.driver.remote.daemon import validate_bind_policy

#: The server sends this line when a token is required; the client
#: must answer with the token to be connected.
TOKEN_CHALLENGE = b"raydriver-analyzer: send token\n"

#: Sent before closing an unauthorized connection.
TOKEN_REJECTED = b"raydriver-analyzer: unauthorized\n"


def is_loopback(host: str) -> bool:
    """True when *host* addresses loopback only."""
    if host == "localhost":
        return True
    try:
        return ipaddress.ip_address(host).is_loopback
    except ValueError:
        return False


def make_server_ssl_context(tls_cert: str, tls_key: str) -> ssl.SSLContext:
    """A server TLS context for the given certificate pair."""
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.minimum_version = ssl.TLSVersion.TLSv1_2
    context.load_cert_chain(tls_cert, tls_key)
    return context


def make_client_ssl_context(tls_ca: str) -> ssl.SSLContext:
    """A client TLS context verifying the server against *tls_ca*."""
    context = ssl.create_default_context(
        ssl.Purpose.SERVER_AUTH, cafile=tls_ca
    )
    return context


async def _check_token(reader, writer, token: str) -> bool:
    writer.write(TOKEN_CHALLENGE)
    await writer.drain()
    try:
        answer = await asyncio.wait_for(reader.readline(), timeout=10.0)
    except (asyncio.TimeoutError, OSError):
        return False
    return hmac.compare_digest(answer.strip(), token.encode())


async def serve(
    client_handler,
    *,
    host: str = "127.0.0.1",
    port: int = 0,
    token: str | None = None,
    tls_cert: str | None = None,
    tls_key: str | None = None,
    connections: set | None = None,
) -> asyncio.Server:
    """Start a policy-checked server for *client_handler*.

    *client_handler* is awaited per connection as
    ``handler(reader, writer)`` — but only after the token check
    (when a token is configured) passed.  When *connections* is a
    set, the per-connection tasks are tracked in it, so callers
    can cancel stragglers before ``wait_closed()``.  Returns the
    running :class:`asyncio.Server`.
    """
    validate_bind_policy(host, token, tls_cert)
    ssl_context = None
    if tls_cert is not None:
        if tls_key is None:
            raise InsecureListenerError("--tls-cert also requires --tls-key")
        ssl_context = make_server_ssl_context(tls_cert, tls_key)

    async def handle_client(reader, writer):
        task = asyncio.current_task()
        if connections is not None:
            connections.add(task)
        try:
            if token is not None and not await _check_token(
                reader, writer, token
            ):
                writer.write(TOKEN_REJECTED)
                await writer.drain()
                return
            await client_handler(reader, writer)
        except (ConnectionError, ssl.SSLError, asyncio.IncompleteReadError):
            pass
        finally:
            writer.close()
            try:
                await writer.wait_closed()
            except (ConnectionError, ssl.SSLError):
                pass
            if connections is not None:
                connections.discard(task)

    return await asyncio.start_server(
        handle_client, host, port, ssl=ssl_context
    )


async def connect_authenticated(
    host: str,
    port: int,
    *,
    token: str | None = None,
    tls_ca: str | None = None,
    tls_hostname: str | None = None,
) -> tuple[asyncio.StreamReader, asyncio.StreamWriter]:
    """Connect to an analyzer endpoint, answering its token
    challenge when the server asks for one."""
    ssl_context = (
        make_client_ssl_context(tls_ca) if tls_ca is not None else None
    )
    reader, writer = await asyncio.open_connection(
        host,
        port,
        ssl=ssl_context,
        server_hostname=tls_hostname if ssl_context else None,
    )
    if token is not None:
        try:
            challenge = await asyncio.wait_for(reader.readline(), timeout=10.0)
        except TimeoutError:
            challenge = None
        if challenge != TOKEN_CHALLENGE:
            writer.close()
            try:
                await writer.wait_closed()
            except (ConnectionError, ssl.SSLError):
                pass
            raise ConnectionError(
                "server did not ask for a token; refusing to send it"
            )
        writer.write(token.encode() + b"\n")
        await writer.drain()
    return reader, writer
