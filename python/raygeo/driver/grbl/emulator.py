"""A GRBL 1.1h firmware emulator.

This is *not* a mock: it models the observable behavior of real
Grbl firmware — the character-counting RX buffer, the 15-block
planner with deferred acknowledgements, realtime command
interception, modal G-code state, motion simulation with feed-rate
timing, alarms, homing, probing and the ``$`` system commands — so
the driver is exercised against the same protocol dynamics it sees
on hardware.

Used by the test suite and by ``raydriver run --emulator`` for
hardware-free dry runs.

Timing: all motion durations are scaled by ``speed_factor``
(default 2000x) so jobs finish in milliseconds while preserving the
ordering and buffering semantics of real motion.

The welcome banner is emitted when the first byte arrives after a
(dark) boot — modeling the DTR reset a USB-serial open causes on
real boards.
"""

import asyncio
import math
import random
import re

GRBL_VERSION = "1.1h"
RX_BUFFER_SIZE = 128
PLANNER_BLOCKS = 15
LINE_BUFFER_SIZE = 70

# Grbl 1.1 default EEPROM settings.
DEFAULT_SETTINGS = {
    "0": 10,
    "1": 25,
    "2": 0,
    "3": 0,
    "4": 0,
    "5": 0,
    "6": 0,
    "10": 3,
    "11": 0.020,
    "12": 0.002,
    "13": 0,
    "20": 0,
    "21": 0,
    "22": 0,
    "23": 0,
    "24": 250.0,
    "25": 500.0,
    "26": 250,
    "27": 1.000,
    "30": 1000.0,
    "31": 0.0,
    "32": 0,
    "100": 250.000,
    "101": 250.000,
    "102": 250.000,
    "110": 500.000,
    "111": 500.000,
    "112": 500.000,
    "120": 10.000,
    "121": 10.000,
    "122": 10.000,
    "130": 400.000,
    "131": 300.000,
    "132": 100.000,
}

WELCOME = f"Grbl {GRBL_VERSION} ['$' for help]\r\n".encode()
ALARM_LOCK_MSG = b"[MSG:'$H'|'$X' to unlock]\r\n"

_WORD_RE = re.compile(r"([A-Z])\s*([-+]?[0-9]*\.?[0-9]+)")
_SETTING_RE = re.compile(r"^\$([^=]+)=(.*)$")


def _fmt(value):
    return f"{value:.3f}"


class _GcodeError(Exception):
    def __init__(self, code):
        super().__init__(f"error:{code}")
        self.code = code


class _Block:
    """One planner entry: motion, dwell, homing or probe motion."""

    def __init__(
        self,
        start,
        target,
        feed,
        base_seconds,
        is_jog=False,
        is_dwell=False,
        is_home=False,
        probe_touch=None,
    ):
        self.start = start
        self.target = target
        self.feed = feed
        self.base_seconds = base_seconds
        self.is_jog = is_jog
        self.is_dwell = is_dwell
        self.is_home = is_home
        self.probe_touch: list[float] | None = probe_touch
        self.is_probe = False


class GrblEmulator:
    """GRBL 1.1h emulator driving a MockTransport.

    Runs as an asyncio task: consumes the bytes the session writes,
    executes them with firmware-faithful semantics and pushes
    response bytes back.  Tests interact through the attributes:

    - ``settings``: the $-setting store (mutable at runtime).
    - ``speed_factor``: motion time divisor (raise to speed up).
    - ``silent``: when True, the device ignores everything (dead).
    - ``probe_touch``: machine-space (x, y, z) contact point for
      G38.2; None means the probe never triggers.
    - ``mpos``: current machine position in mm.
    - ``rx_overrun``: True when the host overflowed the RX buffer
      (a protocol violation by the host side).
    - ``home_ack_delay``: seconds to hold back the ``ok`` for ``$H``,
      emulating firmware that acknowledges only after the cycle.
    """

    TICK = 0.002

    def __init__(
        self,
        mock,
        speed_factor=2000.0,
        fragment_output=False,
        home_ack_delay=0.0,
    ):
        self.mock = mock
        self.speed_factor = speed_factor
        self.fragment_output = fragment_output
        self.home_ack_delay = home_ack_delay
        self.silent = False
        self.rx_overrun = False
        self.tx_log = bytearray()

        self.settings = dict(DEFAULT_SETTINGS)
        self.wcs = {f"G5{n}": [0.0, 0.0, 0.0] for n in range(4, 10)}
        self.active_wcs = "G54"
        self.mpos: list[float] = [0.0, 0.0, 0.0]
        self._planned_pos: list[float] = [0.0, 0.0, 0.0]
        self.feed = 0.0
        self.spindle = 0
        self.probe_touch = None
        self.last_prb = (0.0, 0.0, 0.0, 0)

        self._state = "Idle"
        self._hold_requested = False
        self._modal = {
            "motion": None,
            "distance": "G90",
            "units": "G21",
            "plane": "G17",
            "feed_mode": "G94",
            "spindle": "M5",
            "coolant": "M9",
        }
        self._planner = []
        self._current_block = None
        self._block_elapsed = 0.0
        self._line = []
        self._line_overflow = False
        self._pending_lines = []
        self._rx_used = 0
        self._report_count = 0
        self._wco_dirty = True
        self._welcomed = False
        self._last_tick = None
        self._out_rng = random.Random(1234)

    # ------------------------------------------------------------ state

    @property
    def state(self):
        if self._state == "Hold":
            return "Hold:0"
        return self._state

    def _wco(self):
        return self.wcs[self.active_wcs]

    def _report_units(self):
        return self.settings["13"] != 0

    def _to_report(self, value_mm):
        if self._report_units():
            return value_mm / 25.4
        return value_mm

    def _set_alarm(self, code, message=None):
        self._state = "Alarm"
        self._planner.clear()
        self._current_block = None
        self._pending_lines.clear()
        self._line = []
        self._line_overflow = False
        self._rx_used = 0
        self._hold_requested = False
        self._planned_pos = list(self.mpos)
        self._out(f"ALARM:{code}\r\n".encode())
        if message:
            self._out(message)

    # ---------------------------------------------------------- runtime

    async def run(self):
        self._last_tick = self._now()
        while True:
            for chunk in self.mock.take_new_sent():
                self._feed_bytes(chunk)
            self._tick()
            await asyncio.sleep(self.TICK)

    @staticmethod
    def _now():
        return asyncio.get_running_loop().time()

    def _out(self, data: bytes):
        self.tx_log.extend(data)
        if self.silent:
            return
        if self.fragment_output:
            pos = 0
            while pos < len(data):
                size = self._out_rng.randint(1, 12)
                self.mock.push(data[pos : pos + size])
                pos += size
        else:
            self.mock.push(data)

    # ------------------------------------------------- serial byte feed

    def _feed_bytes(self, data: bytes):
        for byte in data:
            self._feed_byte(byte)

    def _feed_byte(self, byte: int):
        if self.silent:
            return
        # Opening the serial port toggles DTR and resets the
        # controller: the first byte arrives right as the firmware
        # (re)boots and prints its banner.
        if not self._welcomed:
            self._welcomed = True
            self._out(WELCOME)
        # Realtime commands are intercepted before the line buffer.
        if byte == 0x18:
            self._soft_reset()
            return
        char = chr(byte)
        if char == "?":
            self._status_report()
            return
        if char == "~":
            self._cycle_resume()
            return
        if char == "!":
            self._feed_hold()
            return
        if 0x84 <= byte <= 0x8B:  # overrides: accepted, no-op here
            return
        if byte in (0x0A, 0x0D):
            line = "".join(self._line)
            self._line = []
            self._rx_used -= len(line)
            if self._line_overflow:
                self._line_overflow = False
                self._out(b"error:1\r\n")
                return
            if line:
                self._pending_lines.append(line)
                self._rx_used += len(line) + 1
            return
        if len(self._line) >= LINE_BUFFER_SIZE - 1:
            # Real Grbl discards the rest of an oversized line and
            # reports line overflow on the newline.
            self._line_overflow = True
            return
        self._line.append(char)
        self._rx_used += 1
        if self._rx_used > RX_BUFFER_SIZE:
            self.rx_overrun = True

    # ---------------------------------------------------------- realtime

    def _soft_reset(self):
        was_moving = (
            self._current_block is not None
            or self._planner
            or self._state in ("Run", "Jog", "Home", "Hold")
        )
        self._planner.clear()
        self._current_block = None
        self._pending_lines.clear()
        self._line = []
        self._line_overflow = False
        self._rx_used = 0
        self._hold_requested = False
        self._planned_pos = list(self.mpos)
        self._out(WELCOME)
        if was_moving or self._state == "Alarm":
            self._state = "Alarm"
            self._out(ALARM_LOCK_MSG)
        else:
            self._state = "Idle"

    def _feed_hold(self):
        if self._current_block is not None:
            self._hold_requested = True
            self._state = "Hold"
            if self._current_block.is_jog:
                # Real Grbl cancels a jog on feed hold.
                self._current_block = None
                self._planner.clear()
                self._planned_pos = list(self.mpos)

    def _cycle_resume(self):
        if self._state == "Hold":
            self._hold_requested = False
            if self._current_block is not None or self._planner:
                self._state = "Run"
            else:
                self._state = "Idle"

    def _status_report(self):
        self._report_count += 1
        fields = []
        if self.settings["10"] & 1:
            fields.append(
                "MPos:" + ",".join(_fmt(self._to_report(v)) for v in self.mpos)
            )
        else:
            wco = self._wco()
            fields.append(
                "WPos:"
                + ",".join(
                    _fmt(self._to_report(self.mpos[i] - wco[i]))
                    for i in range(3)
                )
            )
        if self.settings["10"] & 2:
            rx_avail = RX_BUFFER_SIZE - max(self._rx_used, 0)
            fields.append(
                f"Bf:{PLANNER_BLOCKS - len(self._planner)},{rx_avail}"
            )
        block = self._current_block
        if block is not None and not (block.is_dwell or block.is_home):
            fields.append(
                f"FS:{int(round(self.feed))},{int(round(self.spindle))}"
            )
        if self._wco_dirty or self._report_count % 10 == 1:
            wco = [self._to_report(v) for v in self._wco()]
            fields.append("WCO:" + ",".join(_fmt(v) for v in wco))
            self._wco_dirty = False
        self._out(f"<{self.state}|{'|'.join(fields)}>\r\n".encode())

    # ------------------------------------------------------------- tick

    def _tick(self):
        now = self._now()
        elapsed = now - (
            self._last_tick if self._last_tick is not None else now
        )
        self._last_tick = now
        if elapsed < 0:
            elapsed = 0.0

        # Parse and acknowledge queued lines, exactly like Grbl's
        # protocol loop does while motion is running.
        self._drain_pending_lines()

        if (
            self._current_block is None
            and not self._hold_requested
            and self._state != "Alarm"
        ):
            if self._planner:
                self._current_block = self._planner.pop(0)
                self._block_elapsed = 0.0
                block = self._current_block
                if block.feed and not (block.is_dwell or block.is_home):
                    self.feed = block.feed
                self._state = (
                    "Jog"
                    if block.is_jog
                    else "Home"
                    if block.is_home
                    else "Run"
                )
            elif self._state in ("Run", "Jog", "Home"):
                self._state = "Idle"

        if self._current_block is None or self._hold_requested:
            return

        self._block_elapsed += elapsed
        block = self._current_block
        if self._block_elapsed >= self._block_duration(block):
            self.mpos = list(block.target)
            self._current_block = None
            self._finish_block(block)
        else:
            frac = self._block_elapsed / self._block_duration(block)
            for i in range(3):
                self.mpos[i] = (
                    block.start[i] + (block.target[i] - block.start[i]) * frac
                )

    def _finish_block(self, block):
        if block.is_home:
            self._state = "Idle"
            return
        if block.probe_touch is not None:
            self.mpos = list(block.probe_touch)
            self._report_probe(success=True)
            self._ack(b"ok\r\n")
        elif block.is_probe:
            self._report_probe(success=False)
            self._ack(b"ok\r\n")
            self._set_alarm(
                4,
                b"[MSG:Probe fail - Probe did not contact within travel]\r\n",
            )
        elif block.is_dwell:
            return

    def _report_probe(self, success):
        prb = [self._to_report(v) for v in self.mpos]
        flag = 1 if success else 0
        self.last_prb = (*self.mpos, flag)
        self._out(
            (
                "[PRB:" + ",".join(_fmt(v) for v in prb) + f":{flag}]\r\n"
            ).encode()
        )

    def _drain_pending_lines(self):
        """Execute queued lines while the planner accepts them.

        Like real Grbl, a line that produces motion waits when the
        planner is full — and everything behind it waits too, which
        is exactly what the host's character-counting flow control
        relies on.
        """
        while self._pending_lines:
            line = self._pending_lines[0]
            produces_motion, execute = self._plan_line(line)
            if produces_motion and len(self._planner) >= PLANNER_BLOCKS:
                break
            self._pending_lines.pop(0)
            self._rx_used -= len(line) + 1
            execute()
            if self._state == "Alarm":
                break

    # -------------------------------------------------------- line exec

    def _plan_line(self, raw: str):
        """Pre-analyze a line: does it consume planner space?"""

        def produces():
            stripped = raw.strip().upper()
            if stripped.startswith("$J="):
                return True
            for letter, value, _raw in self._parse_words(raw):
                if letter == "G" and value in (0, 1, 2, 3, 4, 38.2):
                    return True
            return False

        return produces(), lambda: self._execute_line(raw)

    @staticmethod
    def _parse_words(raw: str):
        line = raw
        # Strip comments like real Grbl.
        line = re.sub(r"\([^)]*\)", "", line)
        if ";" in line:
            line = line[: line.index(";")]
        line = line.strip().upper()
        words = []
        for match in _WORD_RE.finditer(line):
            words.append(
                (match.group(1), float(match.group(2)), match.group(2))
            )
        return words

    def _execute_line(self, raw: str):
        words = self._parse_words(raw)
        stripped = raw.strip().upper()
        if not words and not stripped:
            self._ack(b"ok\r\n")
            return
        if stripped.startswith("$J="):
            self._exec_jog(words)
            return
        if stripped.startswith("$"):
            self._exec_system(stripped)
            return
        if self._state == "Alarm":
            self._ack(b"error:9\r\n")
            return
        try:
            self._exec_gcode(words)
        except _GcodeError as exc:
            self._ack(f"error:{exc.code}\r\n".encode())

    def _ack(self, data: bytes):
        self._out(data)

    # ------------------------------------------------------ system cmds

    def _exec_system(self, stripped: str):
        if stripped == "$$":
            for key in sorted(self.settings, key=int):
                value = self.settings[key]
                if isinstance(value, float):
                    self._out(f"${key}={value:.3f}\r\n".encode())
                else:
                    self._out(f"${key}={value}\r\n".encode())
            self._ack(b"ok\r\n")
            return
        if stripped == "$I":
            self._out(f"[VER:{GRBL_VERSION}.EMULATOR:]\r\n".encode())
            self._out(
                f"[OPT:V,{PLANNER_BLOCKS},{RX_BUFFER_SIZE}]\r\n".encode()
            )
            self._ack(b"ok\r\n")
            return
        if stripped == "$G":
            modal = self._modal
            self._out(
                "[{} {} {} {} {} {} {} T0 F{:.3f} S{}]\r\n".format(
                    self.active_wcs,
                    modal["plane"],
                    modal["units"],
                    modal["distance"],
                    modal["feed_mode"],
                    modal["spindle"],
                    modal["coolant"],
                    self.feed,
                    self.spindle,
                ).encode()
            )
            self._ack(b"ok\r\n")
            return
        if stripped == "$#":
            for slot in ("G54", "G55", "G56", "G57", "G58", "G59"):
                values = self.wcs[slot]
                self._out(
                    "[{}:{}]\r\n".format(
                        slot,
                        ",".join(_fmt(self._to_report(v)) for v in values),
                    ).encode()
                )
            self._out(b"[G28:0.000,0.000,0.000]\r\n")
            self._out(b"[G30:0.000,0.000,0.000]\r\n")
            self._out(b"[G92:0.000,0.000,0.000]\r\n")
            self._out(b"[TLO:0.000]\r\n")
            prb = self.last_prb
            self._out(
                "[PRB:{},{},{}:{}]\r\n".format(
                    *(_fmt(self._to_report(v)) for v in prb[:3]),
                    prb[3],
                ).encode()
            )
            self._ack(b"ok\r\n")
            return
        if stripped == "$X":
            was_alarm = self._state == "Alarm"
            self._state = "Idle"
            if was_alarm:
                self._out(b"[MSG:Caution: Unlocked]\r\n")
            self._ack(b"ok\r\n")
            return
        if stripped == "$H" or stripped.startswith("$H "):
            if self.settings["22"] == 0:
                self._ack(b"error:5\r\n")
                return
            self._planner.append(
                _Block(
                    list(self._planned_pos),
                    [0.0, 0.0, 0.0],
                    self.settings["25"],
                    self._motion_duration(
                        self._planned_pos, [0.0, 0.0, 0.0], self.settings["25"]
                    ),
                    is_home=True,
                )
            )
            self._planned_pos = [0.0, 0.0, 0.0]
            if self.home_ack_delay > 0:
                asyncio.get_running_loop().call_later(
                    self.home_ack_delay, self._ack, b"ok\r\n"
                )
            else:
                self._ack(b"ok\r\n")
            return
        match = _SETTING_RE.match(stripped)
        if match:
            key, raw_value = match.groups()
            try:
                value = (
                    float(raw_value) if "." in raw_value else int(raw_value)
                )
            except ValueError:
                self._ack(b"error:5\r\n")
                return
            if key not in self.settings:
                self._ack(b"error:20\r\n")
                return
            self.settings[key] = value
            self._ack(b"ok\r\n")
            return
        self._ack(b"error:20\r\n")

    def _exec_jog(self, words):
        if self._state == "Alarm":
            self._ack(b"error:9\r\n")
            return
        try:
            target, feed, has_axis = self._resolve_motion(
                words, default_motion="G1"
            )
        except _GcodeError as exc:
            self._ack(f"error:{exc.code}\r\n".encode())
            return
        if target is None or not has_axis:
            self._ack(b"error:16\r\n")
            return
        self._planner.append(
            _Block(
                list(self._planned_pos),
                target,
                feed,
                self._motion_duration(self._planned_pos, target, feed),
                is_jog=True,
            )
        )
        self._planned_pos = list(target)
        self._ack(b"ok\r\n")

    # ----------------------------------------------------------- gcode

    def _resolve_motion(self, words, default_motion=None):
        """Compute the machine-space target and feed for a motion.

        ``default_motion`` supplies a modal motion for commands that
        omit one (jog commands default to G1, like real Grbl).
        """
        distance = self._modal["distance"]
        scale = 25.4 if self._modal["units"] == "G20" else 1.0
        use_machine = False
        motion = self._modal["motion"]
        feed = self.feed
        offset = self._wco()
        axis_words = {}
        for letter, value, _raw in words:
            if letter == "G":
                if value in (0, 1, 2, 3):
                    motion = f"G{int(value)}"
                elif value == 38.2:
                    motion = "G38.2"
                elif value == 53:
                    use_machine = True
                elif value == 90:
                    distance = "G90"
                elif value == 91:
                    distance = "G91"
                elif value == 20:
                    scale = 25.4
                elif value == 21:
                    scale = 1.0
            elif letter == "F":
                feed = value * scale
            elif letter in "XYZ":
                axis_words[letter] = value * scale

        if motion is None:
            motion = default_motion
        if motion is None:
            return None, feed, False

        # Targets resolve against the position the planner will have
        # reached when this block runs — not the live position, which
        # lags behind while motion is in flight.
        base = self._planned_pos
        target = []
        for i, axis in enumerate("XYZ"):
            if axis not in axis_words:
                target.append(base[i])
                continue
            word = axis_words[axis]
            if motion == "G38.2" or distance == "G91":
                target.append(base[i] + word)
            else:
                target.append(word + (0.0 if use_machine else offset[i]))
        return target, feed, bool(axis_words)

    def _exec_gcode(self, words):
        motion_pending = None
        dwell = None
        wcs_set = None
        l_value = None
        for letter, value, _raw in words:
            if letter == "G":
                if value in (0, 1, 2, 3, 38.2):
                    motion_pending = value
                elif value == 4:
                    dwell = next((v for w, v, _ in words if w == "P"), 0.0)
                elif value == 10:
                    l_value = next((v for w, v, _ in words if w == "L"), None)
                    wcs_set = next((v for w, v, _ in words if w == "P"), None)
                elif value in (17, 18, 19):
                    self._modal["plane"] = f"G{int(value)}"
                elif value in (20, 21):
                    self._modal["units"] = f"G{int(value)}"
                elif value in (90, 91):
                    self._modal["distance"] = f"G{int(value)}"
                elif value in (93, 94):
                    self._modal["feed_mode"] = f"G{int(value)}"
                elif value in (54, 55, 56, 57, 58, 59):
                    self.active_wcs = f"G{int(value)}"
                    self._wco_dirty = True
                elif value in (28, 30, 92, 53):
                    pass
                else:
                    raise _GcodeError(20)
            elif letter == "M":
                if value in (3, 4, 5):
                    self._modal["spindle"] = f"M{int(value)}"
                    if value == 5:
                        self.spindle = 0
                    else:
                        self.spindle = self._pending_spindle(words)
                elif value in (8, 9):
                    self._modal["coolant"] = f"M{int(value)}"
                elif value in (0, 1, 2):
                    pass
                else:
                    raise _GcodeError(20)
            elif letter == "F":
                self.feed = value * (
                    25.4 if self._modal["units"] == "G20" else 1.0
                )
            elif letter == "S":
                self.spindle = int(value)

        if wcs_set is not None and l_value == 2:
            slot = f"G{53 + int(wcs_set)}"
            values = [0.0, 0.0, 0.0]
            idx = {"X": 0, "Y": 1, "Z": 2}
            scale = 25.4 if self._modal["units"] == "G20" else 1.0
            for letter, value, _raw in words:
                if letter in idx:
                    values[idx[letter]] = value * scale
            self.wcs[slot] = values
            self._wco_dirty = True

        if dwell is not None:
            self._planner.append(
                _Block(
                    list(self.mpos),
                    list(self.mpos),
                    self.feed,
                    dwell,
                    is_dwell=True,
                )
            )

        if motion_pending is not None:
            if not self._exec_motion(motion_pending, words):
                # The line's ack is deferred (probe motion) or
                # suppressed (alarm): no ok here.
                return
        self._ack(b"ok\r\n")

    @staticmethod
    def _pending_spindle(words):
        return int(next((v for w, v, _ in words if w == "S"), 0))

    def _exec_motion(self, motion_value, words):
        """Queue a motion block.

        Returns True when the caller should ack the line, False when
        the ack is deferred to block completion (probing) or
        suppressed (soft-limit alarm), matching real Grbl.
        """
        target, feed, has_axis = self._resolve_motion(words)
        if motion_value == 38.2:
            if not has_axis:
                raise _GcodeError(33)
            self._exec_probe(target, feed)
            return False
        if target is None or not has_axis:
            # Modal-only line like "G1 F600": just updates state.
            if (
                motion_value == 1
                and feed <= 0
                and (self._modal["motion"] == "G1")
            ):
                raise _GcodeError(22)
            self._modal["motion"] = f"G{int(motion_value)}"
            return True
        if motion_value == 1 and feed <= 0:
            raise _GcodeError(22)
        self._check_soft_limits(target)
        if self._state == "Alarm":
            return False
        self._modal["motion"] = f"G{int(motion_value)}"
        if motion_value == 0:
            rapid = max(self.settings["110"], self.settings["111"], 1.0)
            feed = rapid
        elif motion_value in (2, 3):
            if not any(w in ("I", "J", "R") for w, _v, _r in words):
                raise _GcodeError(35)
        self._planner.append(
            _Block(
                list(self._planned_pos),
                target,
                feed,
                self._motion_duration(self._planned_pos, target, feed),
            )
        )
        self._planned_pos = list(target)
        return True

    def _exec_probe(self, target, feed):
        self._check_soft_limits(target)
        if self._state == "Alarm":
            return
        touch = None
        if self.probe_touch is not None:
            for i in range(3):
                lo = min(self.mpos[i], target[i])
                hi = max(self.mpos[i], target[i])
                contact = self.probe_touch[i]
                if lo - 1e-9 <= contact <= hi + 1e-9:
                    touch = list(self.probe_touch)
                    break
        end = target if touch is None else touch
        block = _Block(
            list(self._planned_pos),
            end,
            feed,
            self._motion_duration(self._planned_pos, end, feed),
            probe_touch=touch,
        )
        block.is_probe = True
        self._planner.append(block)
        self._planned_pos = list(end)

    def _check_soft_limits(self, target):
        if self.settings["20"] == 0 or self.settings["22"] == 0:
            return
        for i, travel_key in enumerate(("130", "131", "132")):
            travel = self.settings[travel_key]
            if target[i] < -1e-6 or target[i] > travel + 1e-6:
                # Grbl always acks a processed line: the move is
                # rejected with an error status after the alarm is
                # raised (error:5 = travel violation).
                self._set_alarm(2)
                self._ack(b"error:5\r\n")
                return

    def _motion_duration(self, start, target, feed):
        """Un-scaled block duration at speed_factor 1 (seconds)."""
        dist = math.dist(start, target)
        if dist == 0:
            return 0.0
        feed = max(feed, 1.0)
        return dist / (feed / 60.0)

    def _block_duration(self, block):
        """Current wall-clock duration of *block* under the live
        speed factor (so tests can accelerate mid-job)."""
        return max(block.base_seconds / self.speed_factor, 0.001)
