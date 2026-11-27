"""Rig tests: the PTY serial device and the UDP packet sink."""

import asyncio
import sys

import pytest

from raygeo.driver.rigs.pty_serial import PtySerialDevice
from raygeo.driver.rigs.udp import UdpDevice

pytestmark = pytest.mark.skipif(
    sys.platform == "win32", reason="PTY rigs require Unix"
)


class TestPtySerialDevice:
    def test_creates_a_real_device_path(self):
        rig = PtySerialDevice()
        try:
            import os

            assert os.path.exists(rig.port)
            assert rig.port.startswith("/dev/")
        finally:
            rig.close()

    def test_banner_reaches_the_client_port(self):
        rig = PtySerialDevice(banner=b"[VER:1.1h:]\r\n")
        try:
            with open(rig.port, "rb", buffering=0) as client:
                assert client.readline() == b"[VER:1.1h:]\r\n"
        finally:
            rig.close()

    def test_responder_answers_client_writes(self):
        def responder(data: bytes) -> bytes | None:
            return b"ok\r\n" if data == b"\n" else None

        rig = PtySerialDevice(responder=responder)
        try:
            with open(rig.port, "r+b", buffering=0) as client:
                client.write(b"\n")
                assert client.readline() == b"ok\r\n"
        finally:
            rig.close()

    def test_write_pushes_bytes_to_the_client(self):
        rig = PtySerialDevice()
        try:
            with open(rig.port, "rb", buffering=0) as client:
                rig.write(b"<Idle|MPos:0,0,0>\r\n")
                assert client.readline() == b"<Idle|MPos:0,0,0>\r\n"
        finally:
            rig.close()


class TestUdpDevice:
    async def test_binds_and_receives(self):
        received = []

        def on_datagram(data: bytes, addr):
            received.append(data)

        rig = UdpDevice(on_datagram)
        await rig.start()
        assert rig.port is not None
        try:
            loop = asyncio.get_running_loop()
            transport, _ = await loop.create_datagram_endpoint(
                asyncio.DatagramProtocol,
                remote_addr=("127.0.0.1", rig.port),
            )
            transport.sendto(b"ping")
            for _ in range(100):
                if received:
                    break
                await asyncio.sleep(0.01)
            transport.close()
            assert received == [b"ping"]
        finally:
            await rig.stop()

    async def test_reply_reaches_the_sender(self):
        def on_datagram(data: bytes, addr):
            return b"pong"

        rig = UdpDevice(on_datagram)
        await rig.start()
        try:
            loop = asyncio.get_running_loop()
            queue: asyncio.Queue = asyncio.Queue()

            class _Receiver(asyncio.DatagramProtocol):
                def datagram_received(self, data, addr):
                    loop.call_soon_threadsafe(queue.put_nowait, data)

            # One socket sends the request and receives the reply:
            # the rig answers the datagram's source address.
            transport, _ = await loop.create_datagram_endpoint(
                _Receiver, local_addr=("127.0.0.1", 0)
            )
            transport.sendto(b"ping", ("127.0.0.1", rig.port))
            reply = await asyncio.wait_for(queue.get(), timeout=2.0)
            transport.close()
            assert reply == b"pong"
        finally:
            await rig.stop()
