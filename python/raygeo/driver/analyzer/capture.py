"""Recorded protocol traffic: frames, captures and the recorder.

A capture is an ordered, timestamped log of the frames exchanged
between a host and a device.  It is the analyzer's common currency:
the recorder produces captures from live session events, decoders
turn them into readable listings, the replay command re-sends their
host-to-device frames, and golden tests diff them against fixtures.

Captures are stored as JSONL — one metadata line, then one line per
frame — so they stay diff-friendly and editable by hand.  Text
frames (line-based protocols like GRBL) are stored as strings;
binary protocols store base64.
"""

from __future__ import annotations

import base64
import json
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from importlib.metadata import PackageNotFoundError, version
from pathlib import Path

from raygeo.driver.core import DriverEvent

#: Host to device.
TX = "tx"
#: Device to host.
RX = "rx"

CAPTURE_FORMAT_VERSION = 1


def _raygeo_version():
    try:
        return version("raygeo")
    except PackageNotFoundError:
        return None


@dataclass
class Frame:
    """One recorded frame, *offset* seconds into the capture.

    Exactly one of *text* (line-based protocols) and *data* (binary
    protocols) is set.  A frame with neither is a note: a lifecycle
    marker such as a connect or disconnect.
    """

    direction: str
    offset: float
    text: str | None = None
    data: bytes | None = None
    note: str | None = None

    def payload(self) -> bytes:
        """The frame's wire bytes (text frames gain no newline)."""
        if self.text is not None:
            return self.text.encode("utf-8", errors="replace")
        return self.data if self.data is not None else b""

    def to_json(self, index: int) -> dict:
        record = {
            "i": index,
            "t": round(self.offset, 6),
            "d": self.direction,
        }
        if self.text is not None:
            record["s"] = self.text
        if self.data is not None:
            record["b"] = base64.b64encode(self.data).decode("ascii")
        if self.note is not None:
            record["n"] = self.note
        return record

    @classmethod
    def from_json(cls, record: dict) -> Frame:
        try:
            return cls(
                direction=record["d"],
                offset=record["t"],
                text=record.get("s"),
                data=(
                    base64.b64decode(record["b"]) if "b" in record else None
                ),
                note=record.get("n"),
            )
        except (KeyError, ValueError) as err:
            raise ValueError(f"bad frame record: {record!r}") from err


@dataclass
class Capture:
    """A recorded conversation between host and device."""

    protocol: str = "raw"
    source: str = ""
    started_at: str = field(
        default_factory=lambda: datetime.now(timezone.utc).isoformat()
    )
    frames: list[Frame] = field(default_factory=list)
    _start: float = field(
        default_factory=time.monotonic, init=False, repr=False
    )

    def append(
        self,
        direction: str,
        *,
        text: str | None = None,
        data: bytes | None = None,
        note: str | None = None,
    ) -> Frame:
        """Append a frame timestamped *now*."""
        frame = Frame(
            direction,
            time.monotonic() - self._start,
            text=text,
            data=data,
            note=note,
        )
        self.frames.append(frame)
        return frame

    def append_note(self, note: str) -> Frame:
        """Append a lifecycle marker (connect, disconnect, ...)."""
        return self.append("", note=note)

    def save(self, path) -> None:
        """Write the capture as JSONL (metadata line, then frames)."""
        meta = {
            "format": CAPTURE_FORMAT_VERSION,
            "protocol": self.protocol,
            "source": self.source,
            "started_at": self.started_at,
            "raydriver": _raygeo_version(),
        }
        lines = [json.dumps({"meta": meta})]
        lines += [
            json.dumps(frame.to_json(i)) for i, frame in enumerate(self.frames)
        ]
        Path(path).write_text("\n".join(lines) + "\n", encoding="utf-8")

    @classmethod
    def load(cls, path) -> Capture:
        """Load a JSONL capture written by :meth:`save`."""
        lines = Path(path).read_text(encoding="utf-8").splitlines()
        if not lines:
            raise ValueError(f"{path}: empty capture file")
        meta = json.loads(lines[0]).get("meta", {})
        capture = cls(
            protocol=meta.get("protocol", "raw"),
            source=meta.get("source", ""),
            started_at=meta.get("started_at", ""),
        )
        for line in lines[1:]:
            if line.strip():
                capture.frames.append(Frame.from_json(json.loads(line)))
        return capture


class Recorder:
    """Driver event callback that records traffic into a Capture.

    Pass the recorder (it is callable) as the session's
    ``event_callback``; every raw event (text lines or binary
    chunks) becomes a frame.  *forward*, when given, still receives
    every event unchanged, so the recorder can sit in front of an
    existing callback.
    """

    def __init__(self, capture: Capture | None = None, forward=None):
        self.capture = capture if capture is not None else Capture()
        self.forward = forward

    def __call__(self, event: DriverEvent) -> None:
        match event:
            case DriverEvent.RawTx(line=line):
                self.capture.append(TX, text=line)
            case DriverEvent.RawRx(line=line):
                self.capture.append(RX, text=line)
            case DriverEvent.RawTxBytes(data=data):
                self.capture.append(TX, data=bytes(data))
            case DriverEvent.RawRxBytes(data=data):
                self.capture.append(RX, data=bytes(data))
        if self.forward is not None:
            self.forward(event)
