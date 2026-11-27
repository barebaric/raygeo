"""A Ruida controller emulator (not a mock) for tests.

Speaks the controller's binary protocol over TCP: it acknowledges
every packet, answers ``GET_SETTING`` queries with typed replies and
tracks the job state commanded by ``START_JOB``/``STOP_JOB``/
``PAUSE_JOB``/``RESUME_JOB``/``END_JOB``. Tests inspect ``commands``
(the unswizzled command bytes, as hex) and drive the reported state
through ``set_job_running``, ``set_paused`` and ``set_position``.

Used by the test suite for hardware-free session coverage.
"""

from __future__ import annotations

import asyncio

ACK = 0xCC
CARD_ID_ADDRESS = 0x057E
MACHINE_STATUS_ADDRESS = 0x0400
POSITION_ADDRESSES = (0x0421, 0x0431, 0x0441, 0x0451)
BED_SIZE_ADDRESSES = (0x0026, 0x0036)
MAINBOARD_VERSION_ADDRESS = 0x057F
MACHINE_FEATURES_ADDRESS = 0x030F

STATUS_RUNNING = 0x00000001
STATUS_PAUSED = 0x00000002
STATUS_MOVING = 0x01000000


def swizzle_byte(byte: int, magic: int) -> int:
    byte ^= byte >> 7
    byte ^= (byte << 7) & 0xFF
    byte ^= byte >> 7
    byte ^= magic
    return (byte + 1) & 0xFF


def unswizzle_byte(byte: int, magic: int) -> int:
    byte = (byte - 1) & 0xFF
    byte ^= magic
    byte ^= byte >> 7
    byte ^= (byte << 7) & 0xFF
    byte ^= byte >> 7
    return byte


def uint35(value: int) -> bytes:
    return bytes((value >> shift) & 0x7F for shift in range(28, -1, -7))


def int35(value: int) -> bytes:
    negative = value < 0
    if negative:
        value = (1 << 34) + value
    data = bytearray(uint35(value))
    if negative:
        data[0] |= 0x40
    return bytes(data)


class RuidaEmulator:
    """A firmware-faithful Ruida controller for tests."""

    def __init__(self, magic: int = 0x88, card_id: int = 0x65106510):
        self.magic = magic
        self.card_id = card_id
        self.mainboard_version = "RDC6442S"
        self.features = 0x0001
        self.bed_size = [400.0, 400.0]
        self.position = [0.0, 0.0, 0.0, 0.0]
        self.status = 0
        self.commands: list[str] = []
        self.fail_next_ack = False
        self.port: int | None = None
        self._server: asyncio.AbstractServer | None = None
        self._clients: set = set()

    async def start(self) -> None:
        self._server = await asyncio.start_server(self._handle, "127.0.0.1", 0)
        self.port = self._server.sockets[0].getsockname()[1]

    async def stop(self) -> None:
        if self._server is not None:
            self._server.close()
            await self._server.wait_closed()
        for writer in list(self._clients):
            writer.close()

    # -- test-facing state ------------------------------------------------

    @property
    def job_running(self) -> bool:
        return bool(self.status & STATUS_RUNNING)

    @property
    def paused(self) -> bool:
        return bool(self.status & STATUS_PAUSED)

    def set_job_running(self, running: bool) -> None:
        if running:
            self.status |= STATUS_RUNNING
        else:
            self.status &= ~STATUS_RUNNING

    def set_paused(self, paused: bool) -> None:
        if paused:
            self.status |= STATUS_PAUSED
        else:
            self.status &= ~STATUS_PAUSED

    def set_moving(self, moving: bool) -> None:
        if moving:
            self.status |= STATUS_MOVING
        else:
            self.status &= ~STATUS_MOVING

    def set_position(self, x: float, y: float, z: float = 0.0) -> None:
        self.position = [x, y, z, 0.0]

    # -- protocol ----------------------------------------------------------

    def _swizzle(self, data: bytes) -> bytes:
        return bytes(swizzle_byte(byte, self.magic) for byte in data)

    def _unswizzle(self, data: bytes) -> bytes:
        return bytes(unswizzle_byte(byte, self.magic) for byte in data)

    def _value_for(self, address: int) -> bytes:
        if address == CARD_ID_ADDRESS:
            return uint35(self.card_id)
        if address == MACHINE_STATUS_ADDRESS:
            return uint35(self.status)
        if address in POSITION_ADDRESSES:
            index = POSITION_ADDRESSES.index(address)
            return int35(round(self.position[index] * 1000))
        if address in BED_SIZE_ADDRESSES:
            index = BED_SIZE_ADDRESSES.index(address)
            return int35(round(self.bed_size[index] * 1000))
        if address == MAINBOARD_VERSION_ADDRESS:
            return self.mainboard_version.encode() + b"\x00"
        if address == MACHINE_FEATURES_ADDRESS:
            return uint35(self.features)
        return uint35(0)

    def _reply(self, address: int, data: bytes) -> bytes:
        return self._swizzle(
            bytes(
                [
                    0xDA,
                    0x01,
                    (address >> 8) & 0x7F,
                    address & 0x7F,
                ]
            )
            + data
        )

    def _apply_commands(self, raw: bytes) -> None:
        for index in range(len(raw) - 1):
            prefix, opcode = raw[index], raw[index + 1]
            if prefix == 0xD8 and opcode == 0x00:
                self.set_job_running(True)
                self.set_paused(False)
            elif prefix == 0xD8 and opcode == 0x01:
                self.set_job_running(False)
                self.set_paused(False)
            elif prefix == 0xD8 and opcode == 0x02:
                self.set_paused(True)
            elif prefix == 0xD8 and opcode == 0x03:
                self.set_paused(False)
            elif prefix == 0xE5 and opcode == 0x05:
                self.set_job_running(False)
                self.set_paused(False)

    async def _handle(self, reader, writer):
        self._clients.add(writer)
        try:
            while True:
                data = await reader.read(4096)
                if not data:
                    break
                await self._process(data, writer)
        except (ConnectionError, asyncio.IncompleteReadError):
            pass
        finally:
            self._clients.discard(writer)
            writer.close()

    async def _process(self, data: bytes, writer) -> None:
        raw = self._unswizzle(data)
        self.commands.append(raw.hex())
        self._apply_commands(raw)
        if self.fail_next_ack:
            self.fail_next_ack = False
            return
        writer.write(self._swizzle(bytes([ACK])))
        await writer.drain()
        index = 0
        replies = bytearray()
        while index + 3 < len(raw):
            if raw[index] == 0xDA and raw[index + 1] == 0x00:
                address = (raw[index + 2] << 8) | raw[index + 3]
                replies.extend(self._reply(address, self._value_for(address)))
                index += 4
            else:
                index += 1
        if replies:
            writer.write(bytes(replies))
            await writer.drain()
