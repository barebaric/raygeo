"""A Marlin firmware emulator (not a mock) for tests.

Speaks the protocol the Marlin session expects: the `start`
handshake with a boot banner (no trailing ok, like real firmware),
one `ok` per command line (deferred until the queued motion finishes
executing), `M114` position reports, `M115` firmware info, `M149`
unit output, and the `M410` quick stop.
"""

import asyncio
import random

TICK = 0.002

BOOT_LINES = (
    "start",
    "Marlin 2.1.2",
    "echo:Last Updated: 2024-01-01 | Author: none",
    "echo:Compiled: Jan  1 2024",
)


class _Motion:
    def __init__(self, start, target, feed):
        self.start = start
        self.target = target
        self.feed = feed
        self.elapsed = 0.0

    @property
    def duration(self):
        distance = sum((t - s) ** 2 for t, s in zip(self.target, self.start))
        return distance**0.5


class MarlinEmulator:
    """Runs as an asyncio task: ``task = asyncio.ensure_future(
    emulator.run())``; consumes ``mock.take_new_sent()`` and answers
    via ``mock.push()``."""

    def __init__(self, mock, speed_factor=2000.0, fragment_output=False):
        self.mock = mock
        self.speed_factor = speed_factor
        self.fragment_output = fragment_output
        self.tx_log = bytearray()
        self._out_rng = random.Random(0)
        self._line_buffer = bytearray()
        self._welcomed = False
        self._queue = []
        self._current = None
        self._last_tick = None
        self.mpos = [0.0, 0.0, 0.0]
        self.e_pos = 0.0
        self.absolute = True
        self.feed = 1500.0
        self.units = "mm"
        self.boot_lines = []

    async def run(self):
        self._last_tick = self._now()
        while True:
            # A real device prints its boot banner as soon as the
            # port opens (DTR toggle), on every reconnect.
            if not self._welcomed and self.mock.is_connected:
                self._welcomed = True
                self._emit_boot()
            for chunk in self.mock.take_new_sent():
                self._feed_bytes(chunk)
            self._tick()
            await asyncio.sleep(TICK)

    @staticmethod
    def _now():
        return asyncio.get_running_loop().time()

    def _out(self, data: bytes):
        self.tx_log.extend(data)
        if self.fragment_output:
            pos = 0
            while pos < len(data):
                size = self._out_rng.randint(1, 12)
                self.mock.push(data[pos : pos + size])
                pos += size
        else:
            self.mock.push(data)

    def _feed_bytes(self, data: bytes):
        for byte in data:
            self._line_buffer.append(byte)
            if byte == ord("\n"):
                line = bytes(self._line_buffer).decode(errors="replace")
                self._line_buffer.clear()
                self._handle_line(line.strip())

    def _emit_boot(self):
        for boot_line in BOOT_LINES:
            self._out(f"{boot_line}\r\n".encode())

    def _handle_line(self, line: str):
        if not line:
            return

        words = self._parse_words(line)
        if words.get("M") == 410:
            # Quick stop: takes effect immediately and flushes the
            # pending queue, so the queued motions never ack.
            self._queue.clear()
            self._current = None
            self._out(b"ok\r\n")
            return
        if words.get("M") == 114:
            self._queue.append(
                lambda: (
                    self._out(self._m114_report()),
                    self._out(b"ok\r\n"),
                )
            )
            return
        if words.get("M") == 115:
            self._queue.append(
                lambda: (
                    self._out(
                        b"FIRMWARE_NAME:Marlin 2.1.2 (Github) "
                        b"SOURCE_CODE_URL:github.com\r\n"
                    ),
                    self._out(b"MACHINE_TYPE:3D Printer\r\n"),
                    self._out(b"ok\r\n"),
                )
            )
            return
        if words.get("M") == 149:
            self._queue.append(
                lambda: (
                    self._out(
                        f"echo: M149 Units in {self.units}\r\n".encode()
                    ),
                    self._out(b"ok\r\n"),
                )
            )
            return
        if words.get("G") == 28:
            target = [0.0, 0.0, self.mpos[2]]
            self._queue.append(_Motion(list(self.mpos), target, self.feed))
            return
        if words.get("G") in (0, 1):
            if words.get("F") is not None:
                self.feed = words["F"]
            target = self._target(words)
            self._queue.append(_Motion(list(self.mpos), target, self.feed))
            return
        if words.get("G") == 90:
            self.absolute = True
        elif words.get("G") == 91:
            self.absolute = False

        self._queue.append(lambda: self._out(b"ok\r\n"))

    def _m114_report(self) -> bytes:
        x, y, z = self.mpos
        return (
            f"X:{x:.2f} Y:{y:.2f} Z:{z:.2f} E:{self.e_pos:.2f} "
            f"Count X:{int(x * 100)} Y:{int(y * 100)} Z:{int(z * 100)}"
            "\r\n"
        ).encode()

    def _parse_words(self, line: str) -> dict:
        words = {}
        for token in line.replace("\t", " ").split():
            letter = token[0].upper()
            try:
                words[letter] = float(token[1:])
            except ValueError:
                continue
        return words

    def _target(self, words) -> list[float]:
        target = list(self.mpos)
        for index, letter in enumerate("XYZ"):
            if letter in words:
                value = words[letter]
                target[index] = (
                    target[index] + value if not self.absolute else value
                )
        return target

    def _tick(self):
        now = self._now()
        elapsed = now - (self._last_tick if self._last_tick else now)
        self._last_tick = now
        if elapsed < 0:
            elapsed = 0.0

        if self._current is None:
            if self._queue:
                self._current = self._queue.pop(0)
                if not isinstance(self._current, _Motion):
                    # A queued non-motion command: respond in turn.
                    command = self._current
                    self._current = None
                    command()
                    return

        if self._current is None:
            return

        self._current.elapsed += elapsed
        duration = max(self._current.duration / self.speed_factor, 0.001)
        if self._current.elapsed < duration:
            return

        self.mpos = list(self._current.target)
        self._current = None
        self._out(b"ok\r\n")
