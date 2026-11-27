"""Paced replay of captured host-to-device frames.

Replay sends every ``tx`` frame of a capture to a live connection
with the recorded pacing, so a controller sees (a sped-up version
of) the original conversation.  This drives characterization: edit
a capture's JSONL, replay it against real hardware, and capture
the responses.

Frames are written through an async ``write`` callable, which
keeps this module independent of the transport: today that is a
TCP connection (see the CLI's ``analyzer replay`` command); once
raw transports are exposed to Python it works for serial too.
"""

from __future__ import annotations

import asyncio

from raygeo.driver.analyzer.capture import TX, Capture, Frame


async def replay_frames(
    frames: list[Frame],
    write,
    *,
    speed: float = 1.0,
    sleep=asyncio.sleep,
) -> int:
    """Send every ``tx`` frame via ``await write(bytes)``.

    The delay between frames follows the recorded offsets divided
    by *speed* (``0`` sends as fast as the connection accepts).
    Returns the number of frames written.
    """
    last_offset = None
    written = 0
    for frame in frames:
        if frame.direction != TX or frame.note is not None:
            continue
        if last_offset is not None and speed > 0:
            delay = (frame.offset - last_offset) / speed
            if delay > 0:
                await sleep(delay)
        last_offset = frame.offset
        payload = frame.payload()
        if frame.text is not None and not payload.endswith(b"\n"):
            # Lines were recorded newline-free (RawTx/RawRx strip
            # them); the wire format needs them back.
            payload += b"\n"
        await write(payload)
        written += 1
    return written


async def replay_capture(
    capture: Capture, write, *, speed: float = 1.0, sleep=asyncio.sleep
) -> int:
    """:func:`replay_frames` over a capture's frames."""
    return await replay_frames(capture.frames, write, speed=speed, sleep=sleep)
