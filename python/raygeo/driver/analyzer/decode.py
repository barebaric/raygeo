"""Decoders turn captured frames into readable listings.

A decoder names a protocol ("grbl", "ruida") and annotates one
frame at a time with a short human-readable summary.  The registry
lets new protocols plug in without touching the renderer; captures
from unknown protocols decode with the raw fallback (payload shown
as-is) or as hex dumps.
"""

from __future__ import annotations

import re

from raygeo.driver.analyzer.capture import RX, TX, Capture, Frame
from raygeo.driver.grbl import parser as grbl_parser

_SETTING_LINE = re.compile(r"^\$\d+ = ")

#: Realtime GRBL bytes: executed on receipt, never acknowledged.
_REALTIME_TX = {
    "?": "status report request",
    "~": "cycle start",
    "!": "feed hold",
    "\x18": "soft reset",
}

#: Recognized interactive ``$`` commands.
_INTERACTIVE_TX = {
    "$I": "build info request",
    "$$": "settings request",
    "$G": "parser state request",
    "$#": "WCS offsets request",
    "$H": "home all axes",
    "$X": "clear alarm",
}


class Decoder:
    """Base decoder: annotates frames of one protocol."""

    name = "raw"

    def reset(self) -> None:
        """Forget decoder state between captures."""

    def decode_frame(self, frame: Frame) -> str:
        """Return a one-line summary for *frame* ("" for none)."""
        return ""


class RawDecoder(Decoder):
    """No interpretation; the listing shows the raw payload."""

    name = "raw"


class HexDecoder(Decoder):
    """Every frame is summarized as a hex dump (binary protocols)."""

    name = "hex"

    def decode_frame(self, frame: Frame) -> str:
        data = frame.payload()
        if not data:
            return ""
        return " ".join(f"{byte:02x}" for byte in data[:8]) + (
            " ..." if len(data) > 8 else ""
        )


class GrblDecoder(Decoder):
    """Annotates GRBL traffic using the raydriver response parsers."""

    name = "grbl"

    def decode_frame(self, frame: Frame) -> str:
        if frame.text is None:
            return ""
        line = frame.text.strip()
        if not line:
            return ""
        if frame.direction == TX:
            return self._decode_tx(line)
        return self._decode_rx(line)

    def _decode_tx(self, line: str) -> str:
        if line in _REALTIME_TX:
            return _REALTIME_TX[line]
        if line in _INTERACTIVE_TX:
            return _INTERACTIVE_TX[line]
        if line.startswith("$J="):
            return "jog"
        if line.startswith("$"):
            return "setting command"
        return ""

    def _decode_rx(self, line: str) -> str:
        if line == "ok":
            return "ack"
        if line.startswith("error:"):
            error = grbl_parser.error_code_to_device_error(
                line[len("error:") :].strip()
            )
            return f"error: {error.title}"
        if line.startswith("ALARM:"):
            alarm = grbl_parser.alarm_code_to_device_error(
                line[len("ALARM:") :].strip()
            )
            return f"alarm: {alarm.title}"
        if line.startswith("[MSG:"):
            parsed = grbl_parser.parse_msg(line)
            if parsed is not None:
                key, value = parsed
                return f"message {key}={value}"
        elif line.startswith("[VER:"):
            parsed = grbl_parser.parse_ver(line)
            if parsed is not None:
                v, build = parsed
                return f"version {v}" + (f" ({build})" if build else "")
        elif line.startswith("[OPT:"):
            flags = grbl_parser.parse_opt_flags(line)
            buffer = grbl_parser.parse_opt_info(line)
            parts = []
            if flags is not None:
                parts.append(f"options {flags}")
            if buffer is not None:
                parts.append(f"rx buffer {buffer}")
            if parts:
                return ", ".join(parts)
        elif line.startswith("[PRB:"):
            parsed = grbl_parser.parse_probe_line(line)
            if parsed is not None:
                (x, y, z), found = parsed
                outcome = "found" if found else "miss"
                return f"probe ({outcome}) at {x:g},{y:g},{z:g}"
        elif line.startswith("[G5") or line.startswith("[G59"):
            parsed = grbl_parser.parse_wcs_line(line)
            if parsed is not None:
                name, (x, y, z) = parsed
                return f"WCS {name} = {x:g},{y:g},{z:g}"
        elif line.startswith("<") and line.endswith(">"):
            state = grbl_parser.parse_state(line)
            positions = ",".join(
                f"{value:.3f}" if value is not None else "-"
                for value in state.machine_pos
            )
            summary = f"{state.status.name} MPos:{positions}"
            if state.feed_rate is not None:
                summary += f" F{state.feed_rate:g}"
            return summary
        if _SETTING_LINE.match(line):
            key, _, value = line.partition(" = ")
            return f"setting {key} = {value}"
        return ""


# -- Ruida -----------------------------------------------------------
#
# Ruida controllers speak a binary, swizzled packet protocol (see
# ``src/ruida/codec.rs``).  A frame is one wire chunk: an
# acknowledge byte, a command packet, or a batch of GET_SETTING
# replies.  The decoder finds the controller's swizzle key, undoes
# it, and annotates the payload.  Live-session commands are named;
# other commands fall back to a header plus payload length.

_RUIDA_ACK = 0xCC
_RUIDA_ERR = 0xCD
_RUIDA_ENQ = 0xCE
_RUIDA_NAK = 0xCF
_RUIDA_CONTROLS = {
    _RUIDA_ACK: "ack",
    _RUIDA_ERR: "rejected",
    _RUIDA_ENQ: "enquiry (keep-alive)",
    _RUIDA_NAK: "checksum mismatch",
}
_RUIDA_DEFAULT_MAGIC = 0x88
_RUIDA_RELATIVE = {
    0: "machine",
    1: "anchor",
    2: "current",
    3: "set point",
}
_RUIDA_ADDRESSES = {
    0x0026: "bed width",
    0x0036: "bed height",
    0x030F: "machine features",
    0x0400: "machine status",
    0x0421: "X position",
    0x0431: "Y position",
    0x0441: "Z position",
    0x0451: "U position",
    0x057E: "card id",
    0x057F: "mainboard version",
}
_RUIDA_POSITION_ADDRESSES = frozenset({0x0421, 0x0431, 0x0441, 0x0451})
_RUIDA_BED_SIZE_ADDRESSES = frozenset({0x0026, 0x0036})
#: header bytes -> (annotation, total command length)
_RUIDA_COMMANDS = {
    b"\xc9\x02": ("laser speed", 7),
    b"\xc9\x03": ("axis speed", 7),
    b"\xca\x01\x12": ("air assist off", 3),
    b"\xca\x01\x13": ("air assist on", 3),
    b"\xd8\x00": ("start job", 2),
    b"\xd8\x01": ("stop job", 2),
    b"\xd8\x02": ("pause job", 2),
    b"\xd8\x03": ("resume job", 2),
    b"\xd8\x2a": ("home XY", 2),
    b"\xd8\x2c": ("home Z", 2),
    b"\xd8\x2d": ("home U", 2),
    b"\xd8\x2e": ("focus Z", 2),
    b"\xd9\x00": ("jog X", 8),
    b"\xd9\x01": ("jog Y", 8),
    b"\xd9\x02": ("jog Z", 8),
    b"\xd9\x03": ("jog U", 8),
    b"\xd9\x10": ("jog XY", 13),
    b"\xd9\x30": ("jog XYU", 18),
    b"\xe5\x05": ("end job", 7),
}
_RUIDA_SINGLE_COMMANDS = {
    0xD7: "end of file",
}


def _ruida_unswizzle_byte(byte: int, magic: int) -> int:
    byte = (byte - 1) & 0xFF
    byte ^= magic
    byte ^= byte >> 7
    byte ^= (byte << 7) & 0xFF
    byte ^= byte >> 7
    return byte


def _ruida_unswizzle(data: bytes, magic: int) -> bytes:
    return bytes(_ruida_unswizzle_byte(byte, magic) for byte in data)


def _ruida_uint(data: bytes) -> int:
    value = 0
    for byte in data:
        value = (value << 7) | (byte & 0x7F)
    return value


def _ruida_int(data: bytes) -> int:
    """Signed 7-bit packing, including the LightBurn workaround
    implemented by ``codec::to_int``."""
    if not data:
        return 0
    first = data[0]
    if len(data) == 5 and not first & 0x40 and first & 0x08:
        first |= 0x70
    value = 0
    for index, byte in enumerate(data):
        part = byte & 0x7F
        if index == 0:
            part = first & 0x3F
        value = (value << 7) | part
    if first & 0x40:
        mask = (1 << (len(data) * 7 - 1)) - 1
        value = -((~value & mask) + 1)
    return value


def _ruida_score(raw: bytes) -> int:
    """How much *raw* (already unswizzled) looks like Ruida traffic.

    Strong evidence comes from a setting/reply header or a known
    command header.  A leading ACK only counts as weak evidence on
    its own: any byte maps to an ACK under some swizzle key, so a
    lone control byte cannot identify the controller's key.
    """
    if not raw:
        return 0
    index = 0
    while index < len(raw) and raw[index] == _RUIDA_ACK:
        index += 1
    if index == len(raw):
        return 4
    score = index
    raw = raw[index:]
    if raw[0] in _RUIDA_CONTROLS:
        return score + 1
    if raw[0] == 0xDA and len(raw) >= 2 and raw[1] in (0x00, 0x01):
        return score + 5
    header = raw[:3]
    if header in _RUIDA_COMMANDS:
        return score + 5
    header = raw[:2]
    if header in _RUIDA_COMMANDS:
        return score + 4
    if raw[0] in _RUIDA_SINGLE_COMMANDS:
        return score + 3
    if raw[0] & 0x80:
        return score + 1
    return score


def _ruida_magic(data: bytes) -> tuple[int | None, int]:
    """The swizzle key that makes *data* plausible, with its score.

    The default key wins whenever it produces anything readable.
    Other keys are only trusted on strong evidence (a reply or a
    three-byte command header): any byte maps to an ACK under some
    key, so controls alone cannot identify one.
    """
    sample = data[:3]
    default_score = _ruida_score(
        _ruida_unswizzle(sample, _RUIDA_DEFAULT_MAGIC)
    )
    if default_score >= 2:
        return _RUIDA_DEFAULT_MAGIC, default_score
    best_magic = None
    best_score = 0
    for magic in range(256):
        if magic == _RUIDA_DEFAULT_MAGIC:
            continue
        score = _ruida_score(_ruida_unswizzle(sample, magic))
        if score > best_score:
            best_magic, best_score = magic, score
            if score >= 5:
                break
    if best_score >= 5:
        return best_magic, best_score
    if default_score:
        return _RUIDA_DEFAULT_MAGIC, default_score
    return None, 0


def _ruida_payloads(wire: bytes):
    """Candidate payloads: the chunk itself, and the same chunk
    without a valid UDP checksum prefix."""
    yield wire
    if len(wire) > 2:
        checksum = (wire[0] << 8) | wire[1]
        if checksum == sum(wire[2:]) & 0xFFFF:
            yield wire[2:]


def _ruida_address(address: int) -> str:
    name = _RUIDA_ADDRESSES.get(address)
    if name is None:
        return f"0x{address:04x}"
    return f"0x{address:04x} ({name})"


def _ruida_value(address: int, data: bytes) -> str | None:
    if not data:
        return None
    if address == 0x0400:
        status = _ruida_uint(data)
        states = []
        if status & 0x00000001:
            states.append("running")
        if status & 0x00000002:
            states.append("paused")
        if status & 0x01000000:
            states.append("moving")
        return ", ".join(states) if states else "idle"
    if address in _RUIDA_POSITION_ADDRESSES:
        return f"{_ruida_int(data) / 1000:g} mm"
    if address in _RUIDA_BED_SIZE_ADDRESSES:
        return f"{_ruida_int(data) / 1000:g} mm"
    if address == 0x057E:
        return f"0x{_ruida_uint(data):08x}"
    if address == 0x057F:
        return data.split(b"\x00", 1)[0].decode("ascii", errors="replace")
    if address == 0x030F:
        return f"0x{_ruida_uint(data):x}"
    return str(_ruida_uint(data))


def _ruida_reply(address: int, data: bytes) -> str:
    summary = f"reply {_ruida_address(address)}"
    value = _ruida_value(address, data)
    if value is not None:
        summary += f" = {value}"
    return summary


def _ruida_setting(address: int, data: bytes) -> str:
    summary = f"set {_ruida_address(address)}"
    if len(data) >= 10:
        first = _ruida_uint(data[:5])
        second = _ruida_uint(data[5:10])
        summary += f" = {first}, {second}"
    return summary


def _ruida_jog(name: str, packet: bytes) -> str:
    if len(packet) < 3:
        return name
    axes = name.split(" ", 1)[1]
    frame = _RUIDA_RELATIVE.get(packet[2], str(packet[2]))
    parts = []
    offset = 3
    for axis in axes:
        if offset + 5 > len(packet):
            break
        value = _ruida_int(packet[offset : offset + 5]) / 1000.0
        parts.append(f"{axis}={value:g}")
        offset += 5
    if not parts:
        return f"{name} ({frame})"
    return f"jog {' '.join(parts)} mm ({frame})"


def _ruida_command(name: str, packet: bytes) -> str:
    if name.startswith("jog "):
        return _ruida_jog(name, packet)
    if name.endswith("speed") and len(packet) >= 7:
        value = _ruida_int(packet[2:7]) / 1000.0
        return f"{name} {value:g} mm/s"
    return name


def _ruida_describe(raw: bytes, direction: str) -> list[str]:
    summaries = []
    index = 0
    while index < len(raw):
        byte = raw[index]
        if byte in _RUIDA_CONTROLS:
            summaries.append(_RUIDA_CONTROLS[byte])
            index += 1
            continue
        if byte == 0xDA and index + 4 <= len(raw):
            opcode = raw[index + 1]
            address = (raw[index + 2] << 8) | raw[index + 3]
            if opcode == 0x00:
                summaries.append(f"get {_ruida_address(address)}")
                index += 4
                continue
            if opcode == 0x01:
                end = index + 4
                while end < len(raw) and not raw[end] & 0x80:
                    end += 1
                data = raw[index + 4 : end]
                if direction == RX:
                    summaries.append(_ruida_reply(address, data))
                else:
                    summaries.append(_ruida_setting(address, data))
                index = max(end, index + 4)
                continue
            summaries.append(f"setting command 0x{opcode:02x}")
            index += 2
            continue
        header = raw[index : index + 3]
        if header in _RUIDA_COMMANDS:
            name, length = _RUIDA_COMMANDS[header]
            packet = raw[index : index + length]
            summaries.append(_ruida_command(name, packet))
            index += length
            continue
        header = raw[index : index + 2]
        if header in _RUIDA_COMMANDS:
            name, length = _RUIDA_COMMANDS[header]
            packet = raw[index : index + length]
            summaries.append(_ruida_command(name, packet))
            index += length
            continue
        if byte in _RUIDA_SINGLE_COMMANDS:
            summaries.append(_RUIDA_SINGLE_COMMANDS[byte])
            index += 1
            continue
        end = index + 1
        while end < len(raw) and not raw[end] & 0x80:
            end += 1
        summaries.append(f"command 0x{byte:02x} ({end - index} bytes)")
        index = end
    return summaries


class RuidaDecoder(Decoder):
    """Annotates the binary Ruida packet protocol."""

    name = "ruida"

    def __init__(self, magic: int | None = None):
        self._forced_magic = magic
        self.magic = magic

    def reset(self) -> None:
        self.magic = self._forced_magic

    def decode_frame(self, frame: Frame) -> str:
        wire = frame.payload()
        if not wire:
            return ""
        best = None
        for payload in _ruida_payloads(wire):
            if self.magic is None:
                magic, score = _ruida_magic(payload)
            else:
                magic = self.magic
                score = _ruida_score(_ruida_unswizzle(payload[:3], magic))
            if magic is None:
                continue
            if best is None or score > best[0]:
                best = (score, magic, payload)
        if best is None:
            return ""
        score, magic, payload = best
        if self.magic is None and score >= 3 and magic != _RUIDA_DEFAULT_MAGIC:
            # Remember a non-default controller key so the ACK-only
            # frames that follow decode too.
            self.magic = magic
        raw = _ruida_unswizzle(payload, magic)
        if _ruida_score(raw) == 0:
            return ""
        return "; ".join(_ruida_describe(raw, frame.direction))


_DECODERS: dict[str, Decoder] = {}


def register_decoder(decoder: Decoder) -> None:
    """Make *decoder* available as :func:`get_decoder` target."""
    _DECODERS[decoder.name] = decoder


register_decoder(RawDecoder())
register_decoder(HexDecoder())
register_decoder(GrblDecoder())
register_decoder(RuidaDecoder())


def decoder_names() -> list[str]:
    """The registered decoder names."""
    return sorted(_DECODERS)


def get_decoder(name: str) -> Decoder:
    """The decoder called *name* (unknown names decode as raw)."""
    return _DECODERS.get(name, _DECODERS[RawDecoder.name])


def _display_payload(frame: Frame, width: int) -> str:
    if frame.note is not None:
        return ""
    if frame.text is not None:
        text = frame.text
        for byte in range(0, 32):
            text = text.replace(chr(byte), f"\\x{byte:02x}")
    else:
        data = frame.payload()
        text = " ".join(f"{byte:02x}" for byte in data[:6])
        if len(data) > 6:
            text += " ..."
    if len(text) > width:
        text = text[: width - 1] + "…"
    return text


def format_frame(
    index: int, frame: Frame, decoder: Decoder, *, with_times=True
) -> str:
    """One listing row for *frame* (see :func:`render_listing`)."""
    number = f"{index:>5}"
    when = f"{frame.offset:>9.3f}" if with_times else ""
    lead = f"{number}  {when}  " if with_times else f"{number}  "
    if frame.note is not None:
        row = f"{lead}{'':>3}  {'':<26}  {frame.note}"
    else:
        direction = "->" if frame.direction == TX else "<-"
        payload = _display_payload(frame, 26)
        summary = decoder.decode_frame(frame)
        row = f"{lead}{direction:>3}  {payload:<26}  {summary}"
    return row.rstrip()


def render_listing(
    capture: Capture, decoder: Decoder | None = None, *, with_times=True
) -> str:
    """Render a capture as a fixed-column listing, one line per
    frame.  ``with_times=False`` drops the timestamp column so
    listings from different runs compare equal (golden tests)."""
    decoder = decoder or get_decoder(capture.protocol)
    decoder.reset()
    prefix = f"{'#':>5}  {'t':>9}  " if with_times else f"{'#':>5}  "
    header = f"{prefix}{'dir':>3}  {'payload':<26}  decode".rstrip()
    lines = [header]
    for index, frame in enumerate(capture.frames):
        lines.append(
            format_frame(index, frame, decoder, with_times=with_times)
        )
    return "\n".join(lines) + "\n"
