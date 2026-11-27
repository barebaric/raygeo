"""A pty-backed fake serial device: the device side of a serial
link without hardware."""

from __future__ import annotations

import os
import threading

try:  # tty/termios are POSIX-only; the rig degrades gracefully.
    import tty

    _HAVE_TTY = True
except ImportError:  # pragma: no cover - platform-dependent
    tty = None
    _HAVE_TTY = False

__all__ = [
    "PtySerialDevice",
]


class PtySerialDevice:
    """A pty-backed fake serial device.

    The device side is a real port: :attr:`port` is the slave path
    a session's serial transport opens. *banner* is written before
    the client opens the port; *responder* — a callable mapping
    received bytes to a reply (``None`` sends nothing) — runs on a
    thread until :meth:`close`.
    """

    def __init__(self, banner: bytes = b"", responder=None):
        if not _HAVE_TTY:
            raise RuntimeError("PtySerialDevice requires a POSIX host")
        self._master, self._slave = os.openpty()
        # Raw mode: no echo of the client's bytes and no CR/NL
        # translation, so captured bytes match what was sent.
        if tty is not None:
            tty.setraw(self._slave)
        self.port = os.ttyname(self._slave)
        self._stop = threading.Event()
        self._thread = None
        if banner:
            os.write(self._master, banner)
        if responder is not None:
            self._thread = threading.Thread(
                target=self._serve, args=(responder,), daemon=True
            )
            self._thread.start()

    def _serve(self, responder):
        while not self._stop.is_set():
            try:
                data = os.read(self._master, 256)
            except OSError:
                return
            if not data:
                return
            reply = responder(data)
            if reply:
                os.write(self._master, reply)

    def write(self, data: bytes) -> None:
        """Push bytes out to the client, as a device would."""
        os.write(self._master, data)

    def close(self):
        self._stop.set()
        # Closing the slave first wakes a responder blocked on the
        # master; on macOS close(master) blocks while the slave is
        # open.
        os.close(self._slave)
        if self._thread is not None:
            self._thread.join(timeout=2.0)
        os.close(self._master)
