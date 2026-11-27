"""A Smoothieware firmware emulator (not a mock) for tests.

Speaks the protocol the Smoothie session expects: Grbl-style
`<...>` status reports in response to `?`, one `ok` per command
line (deferred until the queued motion finishes executing, like the
real planner), realtime `!`/`~` feed hold, Ctrl+C job abort, and
the ``version`` / ``config-get`` queries used by device probing.
"""

import asyncio
import random

TICK = 0.002


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


class SmoothieEmulator:
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
        self._state = "Idle"
        self._queue = []
        self._current = None
        self._hold_requested = False
        self._last_tick = None
        self.mpos = [0.0, 0.0, 0.0]
        self.absolute = True
        self.feed = 100.0
        self.config_values = {
            "alpha_max": "200.0000",
            "beta_max": "200.0000",
            "alpha_max_rate": "150.0000",
            "beta_max_rate": "150.0000",
            "acceleration": "3000.0000",
        }

    async def run(self):
        self._last_tick = self._now()
        while True:
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
            if not self._welcomed:
                self._welcomed = True
                self._out(b"Smoothie\r\n")
            if byte in (ord("?"), ord("!"), ord("~"), 0x03):
                self._realtime(bytes([byte]))
                continue
            self._line_buffer.append(byte)
            if byte == ord("\n"):
                line = bytes(self._line_buffer).decode(errors="replace")
                self._line_buffer.clear()
                self._handle_line(line.strip())

    def _realtime(self, data: bytes):
        if data == b"?":
            self._out(self._status_report())
        elif data == b"!":
            self._hold_requested = True
            if self._state != "Idle":
                self._state = "Hold"
        elif data == b"~":
            self._hold_requested = False
            if self._state == "Hold":
                self._state = "Run" if self._current else "Idle"
        elif data == b"\x03":
            self._queue.clear()
            self._current = None
            self._hold_requested = False
            self._state = "Idle"

    def _status_report(self) -> bytes:
        x, y, z = self.mpos
        return (
            f"<{self._state}|MPos:{x:.3f},{y:.3f},{z:.3f}|"
            f"WPos:{x:.3f},{y:.3f},{z:.3f}|FS:0,0>\r\n"
        ).encode()

    def _handle_line(self, line: str):
        if not line:
            return
        if line == "version":
            self._out(b"Build version: edge-94de12c, ")
            self._out(b"Build date: 20240404\r\n")
            self._out(b"ok\r\n")
            return
        if line.startswith("config-get "):
            key = line[len("config-get ") :].strip()
            value = self.config_values.get(key)
            if value is not None:
                self._out(f"{value}\r\n".encode())
            self._out(b"ok\r\n")
            return

        words = self._parse_words(line)
        if words.get("G") in (0, 1):
            if words.get("F") is not None:
                self.feed = words["F"]
            target = self._target(words)
            self._queue.append(_Motion(list(self.mpos), target, self.feed))
            self._state = "Run"
            return
        if words.get("G") == 90:
            self.absolute = True
        elif words.get("G") == 91:
            self.absolute = False

        self._out(b"ok\r\n")

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

        if self._current is None and not self._hold_requested:
            if self._queue:
                self._current = self._queue.pop(0)
            elif self._state == "Run":
                self._state = "Idle"

        if self._current is None or self._hold_requested:
            return

        self._current.elapsed += elapsed
        duration = max(self._current.duration / self.speed_factor, 0.001)
        if self._current.elapsed < duration:
            return

        self.mpos = list(self._current.target)
        self._current = None
        self._out(b"ok\r\n")
