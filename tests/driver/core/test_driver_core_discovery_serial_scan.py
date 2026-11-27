"""Tests for the serial port scanner.

The capture probe runs in Rust (raygeo.driver.core.discovery), so the
device side is simulated with real ptys: a pre-written banner stands
in for a power-on message, and a responder thread answers the
scanner's nudge characters like a silent device would.
"""

import asyncio
import os
import sys
import threading
import time

try:
    import tty
except ImportError:  # Windows has no termios/tty
    tty = None

import pytest

from raygeo.driver.core.discovery.ports import SerialPortInfo
from raygeo.driver.core.discovery.serial_scan import (
    looks_like_device_output,
    scan_serial_ports,
)

requires_pty = pytest.mark.skipif(
    not hasattr(os, "openpty") or tty is None or sys.platform == "darwin",
    reason="requires a POSIX pty; macOS ptys do not deliver master "
    "writes to a serialport reader on CI runners",
)


class PtyDevice:
    """A pty-backed fake serial device.

    *banner* is written before the scan opens the port, like a
    device that talks on power-up. *responder* maps each chunk the
    scanner writes (the nudges) to the bytes the device answers.
    """

    def __init__(self, banner: bytes = b"", responder=None):
        self.master, self.slave = os.openpty()
        # Raw mode: no echo of the scanner's nudges and no CR/NL
        # translation, so captured bytes match what the device sent.
        if tty is not None:
            tty.setraw(self.slave)
        self.port = os.ttyname(self.slave)
        self.writes: list[bytes] = []
        self._stop = threading.Event()
        self._thread = None
        if banner:
            os.write(self.master, banner)
        if responder is not None:
            self._thread = threading.Thread(
                target=self._serve, args=(responder,), daemon=True
            )
            self._thread.start()

    def _serve(self, responder):
        while not self._stop.is_set():
            try:
                data = os.read(self.master, 256)
            except OSError:
                return
            if not data:
                return
            self.writes.append(data)
            reply = responder(data)
            if reply:
                os.write(self.master, reply)

    def close(self):
        self._stop.set()
        # Closing the slave first wakes a responder blocked on the
        # master; on macOS close(master) blocks while the slave is open.
        os.close(self.slave)
        if self._thread is not None:
            self._thread.join(timeout=2.0)
        os.close(self.master)


def _ok_responder(data: bytes) -> bytes:
    return b"ok\r\n"


def test_looks_like_device_output():
    assert looks_like_device_output(b"Grbl 1.1f ['$' for help]\r\n")
    assert looks_like_device_output(b"start\r\necho:busy: processing")
    assert not looks_like_device_output(b"")
    assert not looks_like_device_output(b"\x00\xff\x0f\xd3\xa1")
    assert not looks_like_device_output(b"\n\n\r\n")


@requires_pty
@pytest.mark.asyncio
async def test_scan_observes_responding_port():
    banner = b"Grbl 1.1f ['$' for help]\r\n"
    device = PtyDevice(banner=banner)
    try:
        observations = await scan_serial_ports(ports=[device.port])
        assert len(observations) == 1
        obs = observations[0]
        assert obs.port == device.port
        assert obs.baud_rate == 115200
        assert obs.data == banner
    finally:
        device.close()


@requires_pty
@pytest.mark.asyncio
async def test_scan_accepts_first_responding_baud():
    """A responding port is accepted at the first baud rate that
    yields device-like output; later rates are not tried."""
    device = PtyDevice(banner=b"Grbl 1.1f\r\n", responder=_ok_responder)
    try:
        observations = await scan_serial_ports(
            ports=[device.port], baud_rates=[115200, 9600]
        )
        assert [(o.port, o.baud_rate) for o in observations] == [
            (device.port, 115200)
        ]
    finally:
        device.close()


@requires_pty
@pytest.mark.asyncio
async def test_scan_silent_port_yields_nothing():
    device = PtyDevice()
    try:
        observations = await scan_serial_ports(
            ports=[device.port], baud_rates=[115200]
        )
        assert observations == []
    finally:
        device.close()


@requires_pty
@pytest.mark.asyncio
async def test_scan_skips_port_with_only_garbage():
    """Framing garbage is not device-like output and is skipped."""
    device = PtyDevice(banner=b"\x00\xff\x0f\xd3\xa1\x7f")
    try:
        observations = await scan_serial_ports(
            ports=[device.port], baud_rates=[115200]
        )
        assert observations == []
    finally:
        device.close()


@requires_pty
@pytest.mark.asyncio
async def test_scan_collects_nudge_responses_for_silent_devices():
    """A device without a power-on banner is provoked into talking:
    every nudge is answered, all responses are captured together
    (a bare ``ok`` ack alone is not enough to identify firmware)."""
    replies = {
        b"\n": b"ok\r\n",
        b"?": b"<Idle|MPos:0.000,0.000,0.000|FS:0,0>\r\n",
    }
    device = PtyDevice(responder=lambda data: replies.get(data, b""))
    try:
        observations = await scan_serial_ports(
            ports=[device.port], baud_rates=[115200]
        )
        assert device.writes == [b"\n", b"?"]
        assert len(observations) == 1
        assert observations[0].data == (
            b"ok\r\n<Idle|MPos:0.000,0.000,0.000|FS:0,0>\r\n"
        )
    finally:
        device.close()


@requires_pty
@pytest.mark.asyncio
async def test_scan_skips_unopenable_port():
    device = PtyDevice(banner=b"Grbl 1.1f\r\n", responder=_ok_responder)
    try:
        observations = await scan_serial_ports(
            ports=["/dev/raydriver-does-not-exist", device.port],
            baud_rates=[115200],
        )
        assert [o.port for o in observations] == [device.port]
    finally:
        device.close()


@requires_pty
@pytest.mark.asyncio
async def test_scan_excludes_requested_ports():
    devices = [
        PtyDevice(banner=b"Grbl 1.1f\r\n", responder=_ok_responder)
        for _ in range(2)
    ]
    try:
        observations = await scan_serial_ports(
            ports=[d.port for d in devices],
            exclude_ports={devices[0].port},
            baud_rates=[115200],
        )
        assert [o.port for o in observations] == [devices[1].port]
    finally:
        for device in devices:
            device.close()


@requires_pty
@pytest.mark.asyncio
async def test_scan_uses_port_metadata(monkeypatch):
    device = PtyDevice(banner=b"Grbl 1.1f\r\n", responder=_ok_responder)
    try:
        port_info = [
            SerialPortInfo(
                device=device.port,
                description="Ortur Laser Master",
                manufacturer="Ortur",
                vid=0x1A86,
                pid=0x7523,
            )
        ]
        monkeypatch.setattr(
            "raygeo.driver.core.discovery.serial_scan.list_port_info",
            lambda: port_info,
        )
        observations = await scan_serial_ports(ports=[device.port])
        info = observations[0].info
        assert info is not None
        assert info.description == "Ortur Laser Master"
        assert info.vid == 0x1A86
        assert info.pid == 0x7523
    finally:
        device.close()


@requires_pty
@pytest.mark.asyncio
async def test_scan_returns_partial_results_on_timeout():
    """When the scan deadline is hit, ports that already answered are
    still reported; probes still in flight are cancelled. The silent
    port is listed first: a sequential scan would still be probing it
    at the deadline and report nothing."""
    silent = PtyDevice()
    talking = PtyDevice(banner=b"Grbl 1.1f\r\n", responder=_ok_responder)
    try:
        start = time.monotonic()
        observations = await scan_serial_ports(
            ports=[silent.port, talking.port],
            baud_rates=[115200],
            timeout=2.0,
        )
        assert time.monotonic() - start < 3.5
        assert [o.port for o in observations] == [talking.port]
    finally:
        silent.close()
        talking.close()


@requires_pty
@pytest.mark.asyncio
async def test_scan_keeps_port_order():
    """Concurrent probes still report observations in the given port
    order."""
    devices = [
        PtyDevice(banner=b"Grbl 1.1f\r\n", responder=_ok_responder)
        for _ in range(3)
    ]
    ports = [devices[i].port for i in (2, 0, 1)]
    try:
        observations = await scan_serial_ports(
            ports=ports, baud_rates=[115200]
        )
        assert [o.port for o in observations] == ports
    finally:
        for device in devices:
            device.close()


@requires_pty
@pytest.mark.asyncio
async def test_concurrent_scans_serialize_on_port():
    """A second scan cannot open a port while another scan is still
    probing it; by then the first scan has consumed the banner and
    the device answers no nudges, so the second scan sees silence."""
    device = PtyDevice(banner=b"Grbl 1.1f\r\n")
    try:
        scan1 = asyncio.ensure_future(
            scan_serial_ports(ports=[device.port], baud_rates=[115200])
        )
        await asyncio.sleep(0.1)
        scan2 = asyncio.ensure_future(
            scan_serial_ports(ports=[device.port], baud_rates=[115200])
        )
        first, second = await asyncio.gather(scan1, scan2)
        assert [o.port for o in first] == [device.port]
        assert second == []
    finally:
        device.close()
