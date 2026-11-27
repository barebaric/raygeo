"""Man-in-the-middle TCP proxy with capture.

The proxy sits between an application and a network-attached
device: the application connects to the analyzer's listener, the
analyzer connects to the real device, and every byte crossing in
either direction is recorded into a capture (and decoded live by
the CLI).  This is the characterization workflow: point a GRBL or
Ruida application at the proxy and capture exactly what it sends.

The listener side enforces the analyzer security policy
(:mod:`raygeo.driver.analyzer.listener`): loopback by default, token
and TLS required for anything exposed.  Only TCP upstreams are
supported today; a serial MITM needs raw transport bindings and
will follow.
"""

from __future__ import annotations

import asyncio

from raygeo.driver.analyzer.capture import RX, TX, Capture
from raygeo.driver.analyzer.listener import serve

#: Split recorded chunks into line frames (text protocols).
FRAMING_LINES = "lines"
#: Record chunks as-is (binary protocols).
FRAMING_RAW = "raw"

_READ_CHUNK = 4096


class _LineAssembler:
    """Buffers a byte stream and yields complete lines."""

    def __init__(self):
        self._buffer = bytearray()

    def feed(self, chunk: bytes) -> list[bytes]:
        self._buffer += chunk
        lines = []
        while True:
            newline = self._buffer.find(b"\n")
            if newline < 0:
                break
            lines.append(bytes(self._buffer[: newline + 1]))
            del self._buffer[: newline + 1]
        return lines

    def flush(self) -> bytes:
        rest = bytes(self._buffer)
        self._buffer.clear()
        return rest


class ProxyServer:
    """A running proxy; see :func:`run_proxy`."""

    def __init__(
        self,
        server: asyncio.Server,
        capture: Capture,
        connections: set,
    ):
        self._server = server
        self._connections = connections
        self.capture = capture

    @property
    def port(self) -> int:
        """The port clients connect to."""
        sockets = self._server.sockets
        if not sockets:
            raise RuntimeError("proxy server is closed")
        return sockets[0].getsockname()[1]

    async def stop(self) -> None:
        """Stop listening and tear down open client sessions."""
        self._server.close()
        for task in list(self._connections):
            task.cancel()
        if self._connections:
            await asyncio.gather(*self._connections, return_exceptions=True)
        await self._server.wait_closed()


async def _pump(
    reader,
    writer,
    direction: str,
    capture: Capture,
    framing: str,
) -> None:
    assembler = _LineAssembler() if framing == FRAMING_LINES else None
    try:
        while True:
            chunk = await reader.read(_READ_CHUNK)
            if not chunk:
                break
            if assembler is None:
                capture.append(direction, data=chunk)
            else:
                for line in assembler.feed(chunk):
                    capture.append(direction, text=_decode(line))
            writer.write(chunk)
            await writer.drain()
    finally:
        if assembler is not None and assembler.flush():
            capture.append(direction, data=assembler.flush())
        try:
            writer.close()
        except (ConnectionError, OSError):
            pass


def _decode(line: bytes) -> str:
    return line.decode("utf-8", errors="replace").rstrip("\r\n")


async def run_proxy(
    upstream_host: str,
    upstream_port: int,
    *,
    listen_host: str = "127.0.0.1",
    listen_port: int = 0,
    token: str | None = None,
    tls_cert: str | None = None,
    tls_key: str | None = None,
    capture: Capture | None = None,
    framing: str = FRAMING_LINES,
) -> ProxyServer:
    """Start a MITM proxy forwarding to *upstream_host:upstream_port*.

    Returns a :class:`ProxyServer` whose ``capture`` accumulates
    both directions; ``await stop()`` shuts it down.
    """
    if framing not in (FRAMING_LINES, FRAMING_RAW):
        raise ValueError(f"unknown framing: {framing!r}")
    capture = capture if capture is not None else Capture()
    connections: set = set()

    async def handle_client(client_reader, client_writer):
        peer = client_writer.get_extra_info("peername")
        capture.append_note(f"client connected from {peer}")
        try:
            device_reader, device_writer = await asyncio.open_connection(
                upstream_host, upstream_port
            )
        except OSError as err:
            capture.append_note(f"upstream unreachable: {err}")
            client_writer.close()
            return
        try:
            await asyncio.gather(
                _pump(client_reader, device_writer, TX, capture, framing),
                _pump(device_reader, client_writer, RX, capture, framing),
            )
        finally:
            capture.append_note("client disconnected")

    server = await serve(
        handle_client,
        host=listen_host,
        port=listen_port,
        token=token,
        tls_cert=tls_cert,
        tls_key=tls_key,
        connections=connections,
    )
    return ProxyServer(server, capture, connections)
