"""A UDP packet sink a session can talk to: the device side of a
UDP link without hardware."""

from __future__ import annotations

import asyncio

__all__ = [
    "UdpDevice",
]


class UdpDevice:
    """A UDP packet sink a session can talk to.

    Binds a local socket and hands every received datagram to
    *on_datagram* — a callable taking ``(data, addr)`` whose return
    value (if any) is sent back to the sender, so a rig can answer
    requests like a device would. *port* ``0`` binds an ephemeral
    port; the chosen port is in :attr:`port` after :meth:`start`.
    """

    def __init__(self, on_datagram, port: int = 0, host: str = "127.0.0.1"):
        self._on_datagram = on_datagram
        self._host = host
        self._requested_port = port
        self.port: int | None = None
        self._transport: asyncio.DatagramTransport | None = None

    async def start(self) -> None:
        loop = asyncio.get_running_loop()
        self._transport, _ = await loop.create_datagram_endpoint(
            lambda: _UdpDeviceProtocol(self),
            local_addr=(self._host, self._requested_port),
        )
        sock = self._transport.get_extra_info("socket")
        if sock:
            self.port = sock.getsockname()[1]

    async def stop(self) -> None:
        if self._transport is not None:
            self._transport.close()
            self._transport = None

    def send_to(self, data: bytes, addr: tuple[str, int]) -> None:
        if self._transport is None:
            raise ConnectionError("UdpDevice is not started")
        self._transport.sendto(data, addr)


class _UdpDeviceProtocol(asyncio.DatagramProtocol):
    def __init__(self, rig: UdpDevice):
        self._rig = rig

    def datagram_received(self, data: bytes, addr: tuple[str, int]) -> None:
        reply = self._rig._on_datagram(data, addr)
        if reply:
            self._rig.send_to(reply, addr)
