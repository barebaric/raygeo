import asyncio
import os
import sys
import threading

try:
    import tty
except ImportError:  # Windows has no termios/tty
    tty = None

import pytest

from raygeo.driver.core.discovery import mdns_channel, serial_channel
from raygeo.driver.core.discovery.engine import find_all_devices
from raygeo.driver.core.discovery.mdns_channel import find_network_devices
from raygeo.driver.core.discovery.mdns_scan import MDNSService
from raygeo.driver.core.discovery.ports import SerialPortInfo
from raygeo.driver.core.discovery.serial_channel import (
    GENERIC_TOKENS,
    build_identity,
    normalize_tokens,
)
from raygeo.driver.core.discovery.spec import (
    DiscoverySpec,
    MdnsRecognizer,
    SerialRecognizer,
)

requires_pty = pytest.mark.skipif(
    not hasattr(os, "openpty") or tty is None or sys.platform == "darwin",
    reason="requires a POSIX pty; macOS ptys do not deliver master "
    "writes to a serialport reader on CI runners",
)


@pytest.fixture(autouse=True)
def _isolate_network_channels(monkeypatch):
    """Isolate from real network discovery.

    find_all_devices also browses for raydriver daemons and scans
    the local subnets; without this, daemons or controllers on the
    test machine's network would leak into the results.
    """
    from raygeo.driver.core.discovery import engine

    async def _none(*args, **kwargs):
        return []

    monkeypatch.setattr(engine, "find_raygeo_devices", _none)
    monkeypatch.setattr(engine, "find_subnet_devices", _none)


def _grbl_matcher(data: bytes) -> bool:
    # Mirrors the real is_grbl_output(): banner or realtime status
    # report. A bare "ok" ack must NOT match (Marlin acks alike).
    return b"Grbl" in data or b"<Idle" in data


def _grbl_name(data: bytes) -> str | None:
    # Mirrors the real extract_device_name_from_output(): the first
    # informative line, skipping bare "ok" acks.
    for raw in data.decode("ascii", errors="replace").splitlines():
        line = raw.strip()
        if line and line != "ok":
            return line[:80]
    return None


def _marlin_matcher(data: bytes) -> bool:
    return b"start" in data or b"echo:" in data


def _marlin_name(data: bytes) -> str | None:
    # Mirrors the real extract_marlin_banner_from_output(): the first
    # boot message line, skipping "ok" acks.
    for raw in data.decode("ascii", errors="replace").splitlines():
        line = raw.strip()
        if not line or line == "ok":
            continue
        if line.startswith(("start", "Marlin")) or "echo:" in line:
            return line[:80]
    return None


class FakeGrblDriver:
    DISCOVERY = DiscoverySpec(
        serial=SerialRecognizer(
            label=lambda: "GRBL device",
            matches=_grbl_matcher,
            name=_grbl_name,
            firmware="grbl",
        )
    )


class FakeMarlinDriver:
    DISCOVERY = DiscoverySpec(
        serial=SerialRecognizer(
            label=lambda: "Marlin device",
            matches=_marlin_matcher,
            name=_marlin_name,
            firmware="marlin",
        )
    )


class FakeOctoPrintDriver:
    DISCOVERY = DiscoverySpec(
        mdns=MdnsRecognizer(services=("_octoprint._tcp.local.",))
    )
    label = "OctoPrint"


class FakeOctoPrintWithTxtDriver:
    DISCOVERY = DiscoverySpec(
        mdns=MdnsRecognizer(
            services=("_octoprint._tcp.local.",),
            txt_map={"path": "path"},
        )
    )
    label = "OctoPrint"


class PtyDevice:
    """A pty-backed fake serial device: the capture probe runs in
    Rust, so the device side is a real port. *banner* is written
    before the scan opens the port; *responder* answers the nudges."""

    def __init__(self, banner: bytes = b"", responder=None):
        self.master, self.slave = os.openpty()
        # Raw mode: no echo of the scanner's nudges and no CR/NL
        # translation, so captured bytes match what the device sent.
        if tty is not None:
            tty.setraw(self.slave)
        self.port = os.ttyname(self.slave)
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


def _grbl_nudge_responder(data: bytes) -> bytes:
    """A bannerless GRBL device: ``ok`` for a newline, a status
    report for ``?``."""
    return {
        b"\n": b"ok\r\n",
        b"?": b"<Idle|MPos:0.0,0.0,0.0>\r\n",
    }.get(data, b"ok\r\n")


@pytest.fixture(autouse=True)
def no_mdns(monkeypatch):
    """Tests opt into mDNS explicitly; by default no browse happens
    (find_all_devices also scans the network)."""

    async def _no_browse(service_types):
        return []

    monkeypatch.setattr(mdns_channel, "scan_mdns_services", _no_browse)


# ----- serial channel ---------------------------------------------------


def test_normalize_tokens():
    tokens = normalize_tokens("Ortur Laser_Master", "CH340", None)
    # Generic chip names stay in the token set; they are filtered at
    # match time via GENERIC_TOKENS.
    assert tokens == {"ortur", "laser", "master", "ch340"}
    assert normalize_tokens(None, "") == frozenset()
    assert "ch340" in GENERIC_TOKENS


def test_build_identity():
    info = SerialPortInfo("/dev/ttyUSB0", "USB Serial", vid=1, pid=2)
    identity = build_identity("grbl", info)
    assert identity.firmware == "grbl"
    assert identity.banner is None
    assert identity.usb_vid == 1
    assert identity.usb_pid == 2
    assert "usb" in identity.tokens
    assert "serial" in identity.tokens


def test_build_identity_uses_device_name_as_banner():
    info = SerialPortInfo("/dev/ttyUSB0", "USB Serial", vid=1, pid=2)
    identity = build_identity("grbl", info, "Grbl 1.1f")
    assert identity.banner == "Grbl 1.1f"
    assert "grbl" in identity.tokens


@pytest.mark.asyncio
@requires_pty
async def test_one_scan_serves_all_drivers():
    """Adding discovery-capable drivers never multiplies port opens:
    both recognizers evaluate a single scan."""
    grbl_port = PtyDevice(banner=b"Grbl 1.1f ['$' for help]\r\n")
    marlin_port = PtyDevice(banner=b"start\r\necho:Marlin initialized\r\n")
    try:
        devices = await find_all_devices(
            [FakeGrblDriver, FakeMarlinDriver],
            ports=[grbl_port.port, marlin_port.port],
        )
    finally:
        grbl_port.close()
        marlin_port.close()
    by_driver = {d.driver_name: d for d in devices}
    assert set(by_driver) == {"FakeGrblDriver", "FakeMarlinDriver"}

    grbl = by_driver["FakeGrblDriver"]
    assert grbl.params == {"port": grbl_port.port, "baudrate": 115200}
    assert grbl.label == "GRBL device"
    assert grbl.detail == f"{grbl_port.port} at 115200 baud"
    assert grbl.identity.firmware == "grbl"
    assert grbl.identity.banner == "Grbl 1.1f ['$' for help]"

    marlin = by_driver["FakeMarlinDriver"]
    assert marlin.params["port"] == marlin_port.port
    assert marlin.identity.firmware == "marlin"
    assert marlin.identity.banner == "start"


@pytest.mark.asyncio
@requires_pty
async def test_bannerless_grbl_is_recognized():
    """A GRBL device that never emits a boot banner (no DTR reset)
    still identifies itself through its nudge responses alone."""
    port = PtyDevice(responder=_grbl_nudge_responder)
    try:
        devices = await find_all_devices(
            [FakeGrblDriver, FakeMarlinDriver], ports=[port.port]
        )
    finally:
        port.close()
    assert [d.driver_name for d in devices] == ["FakeGrblDriver"]
    # The bare "ok" ack is skipped in favour of the status report,
    # which is a more informative banner for a bannerless device.
    assert devices[0].identity.banner == "<Idle|MPos:0.0,0.0,0.0>"


# Real-world capture from a Sculpfun iCube: a Grbl fork whose boot
# output never mentions Grbl — only $I-style build-info lines.
SCULPFUN_ICUBE_BANNER = (
    b"[VER:1.0.15,20240923:]\r\n"
    b"[OPT:VMP,31,511]\r\n"
    b"[MSG:mechine:Sculpfun iCube]\r\n"
    b"[MSG:Mode=BT]\r\n"
    b"[BT_VER:8.1.2,FSC-BT836B]\r\n"
    b"Connection status: CONNECTED \r\n"
)


@pytest.mark.asyncio
@requires_pty
async def test_unrecognized_output_is_not_reported():
    port = PtyDevice(banner=b"SomeOtherFW v2\r\n")
    try:
        devices = await find_all_devices(
            [FakeGrblDriver, FakeMarlinDriver], ports=[port.port]
        )
    finally:
        port.close()
    assert devices == []


@pytest.mark.asyncio
async def test_drivers_without_recognizer_are_ignored():
    class UndiscoverableDriver:
        DISCOVERY = None

    # No recognizer at all means no scan is even attempted, so a
    # nonexistent port is never opened.
    devices = await find_all_devices(
        [UndiscoverableDriver], ports=["/dev/raydriver-does-not-exist"]
    )
    assert devices == []


@pytest.mark.asyncio
@requires_pty
async def test_broken_recognizer_does_not_break_discovery():
    port = PtyDevice(banner=b"Grbl 1.1f\r\n")

    def boom(data: bytes) -> bool:
        raise OSError("boom")

    class BrokenDriver:
        DISCOVERY = DiscoverySpec(
            serial=SerialRecognizer(
                label=lambda: "Broken",
                matches=boom,
            )
        )

    try:
        devices = await find_all_devices(
            [BrokenDriver, FakeGrblDriver], ports=[port.port]
        )
    finally:
        port.close()
    assert [d.driver_name for d in devices] == ["FakeGrblDriver"]


@pytest.mark.asyncio
@requires_pty
async def test_silent_ports_do_not_hide_responsive_ones(monkeypatch):
    """Silent adapters used to burn the whole scan budget, and hitting
    that budget discarded every observation. Now ports that answered
    before the deadline are still reported, so a responsive machine
    later in sort order is found alongside dumb adapters."""
    silent1 = PtyDevice()
    silent2 = PtyDevice()
    talking = PtyDevice(banner=b"Grbl 1.1f\r\n")
    monkeypatch.setattr(serial_channel, "SERIAL_SCAN_TIMEOUT", 2.0)

    try:
        devices = await find_all_devices(
            [FakeGrblDriver],
            ports=[silent1.port, talking.port, silent2.port],
        )
    finally:
        silent1.close()
        silent2.close()
        talking.close()
    assert [d.params["port"] for d in devices] == [talking.port]


@pytest.mark.asyncio
async def test_scan_failure_returns_empty(monkeypatch):
    async def broken_scan(**kwargs):
        raise OSError("boom")

    monkeypatch.setattr(serial_channel, "scan_serial_ports", broken_scan)
    devices = await find_all_devices([FakeGrblDriver], ports=["/dev/x"])
    assert devices == []


# ----- network channel --------------------------------------------------


def _octoprint_service(**overrides):
    defaults = {
        "service_type": "_octoprint._tcp",
        "name": "OctoPrint on octopi",
        "host": "192.168.1.42",
        "port": 80,
        "server": "octopi.local",
        "txt": {},
    }
    defaults.update(overrides)
    return MDNSService(**defaults)


@pytest.mark.asyncio
async def test_network_devices_from_mdns(monkeypatch):
    async def fake_scan(service_types):
        assert service_types == {"_octoprint._tcp"}
        return [_octoprint_service()]

    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    devices = await find_network_devices([FakeOctoPrintDriver])
    assert len(devices) == 1
    device = devices[0]
    assert device.driver_name == "FakeOctoPrintDriver"
    assert device.params == {"host": "192.168.1.42", "port": 80}
    assert device.label == "OctoPrint"
    assert device.detail == "192.168.1.42:80 (octopi.local)"
    assert device.key == "FakeOctoPrintDriver:192.168.1.42:80"
    assert device.identity.banner == "OctoPrint on octopi"
    assert "octoprint" in device.identity.tokens
    assert "octopi" in device.identity.tokens


@pytest.mark.asyncio
async def test_mdns_txt_forwarded_into_params(monkeypatch):
    """A TXT key declared in the recognizer's txt_map is forwarded
    into the discovered device's params under the mapped arg key."""
    service = _octoprint_service(
        txt={"path": "/octoprint", "version": "1.10.3", "api": "0.1"}
    )

    async def fake_scan(service_types):
        return [service]

    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    devices = await find_network_devices([FakeOctoPrintWithTxtDriver])
    assert len(devices) == 1
    device = devices[0]
    assert device.params == {
        "host": "192.168.1.42",
        "port": 80,
        "path": "/octoprint",
    }


@pytest.mark.asyncio
async def test_mdns_txt_empty_value_not_forwarded(monkeypatch):
    """A TXT key whose value is empty is not forwarded, so the
    driver's own default applies."""
    service = _octoprint_service(txt={"path": ""})

    async def fake_scan(service_types):
        return [service]

    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    devices = await find_network_devices([FakeOctoPrintWithTxtDriver])
    assert len(devices) == 1
    assert devices[0].params == {"host": "192.168.1.42", "port": 80}


@pytest.mark.asyncio
async def test_mdns_txt_vendor_model_enrich_identity(monkeypatch):
    """vendor and model TXT keys strengthen the identity token set
    used for profile matching; version becomes the banner when the
    service announced no instance name."""
    service = _octoprint_service(
        name="",
        server="octopi.local",
        txt={
            "vendor": "Prusa Research",
            "model": "MK4",
            "version": "OctoPrint 1.10.3",
        },
    )

    async def fake_scan(service_types):
        return [service]

    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    devices = await find_network_devices([FakeOctoPrintWithTxtDriver])
    assert len(devices) == 1
    device = devices[0]
    # version is the banner because no instance name was announced
    assert device.identity.banner == "OctoPrint 1.10.3"
    assert "prusa" in device.identity.tokens
    assert "mk4" in device.identity.tokens


@pytest.mark.asyncio
async def test_mdns_txt_instance_name_preferred_as_banner(monkeypatch):
    """When the service announces an instance name it is the banner,
    even if a version TXT is also present."""
    service = _octoprint_service(
        name="OctoPrint on octopi",
        txt={"version": "1.10.3"},
    )

    async def fake_scan(service_types):
        return [service]

    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    devices = await find_network_devices([FakeOctoPrintWithTxtDriver])
    assert devices[0].identity.banner == "OctoPrint on octopi"


@pytest.mark.asyncio
async def test_mdns_declaration_without_local_suffix(monkeypatch):
    """Service type declarations and browse results are normalized
    on both sides, so "_octoprint._tcp" finds "_octoprint._tcp.local."
    announcements."""

    class BareDeclarationDriver:
        DISCOVERY = DiscoverySpec(
            mdns=MdnsRecognizer(services=("_octoprint._tcp",))
        )
        label = "Bare"

    async def fake_scan(service_types):
        assert service_types == {"_octoprint._tcp"}
        return [_octoprint_service()]

    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    devices = await find_network_devices([BareDeclarationDriver])
    assert [d.driver_name for d in devices] == ["BareDeclarationDriver"]


@pytest.mark.asyncio
@requires_pty
async def test_find_all_devices_merges_serial_and_network(monkeypatch):
    port = PtyDevice(banner=b"Grbl 1.1f\r\n")

    async def fake_mdns(service_types):
        return [_octoprint_service()]

    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_mdns)
    try:
        devices = await find_all_devices(
            [FakeGrblDriver, FakeOctoPrintDriver], ports=[port.port]
        )
    finally:
        port.close()
    assert {d.driver_name for d in devices} == {
        "FakeGrblDriver",
        "FakeOctoPrintDriver",
    }


@pytest.mark.asyncio
@requires_pty
async def test_no_mdns_declarations_skips_network_scan(monkeypatch):
    calls = []

    async def fake_mdns(service_types):
        calls.append(service_types)
        return []

    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_mdns)
    port = PtyDevice(banner=b"Grbl 1.1f\r\n")
    try:
        devices = await find_all_devices(
            [FakeGrblDriver, FakeMarlinDriver], ports=[port.port]
        )
    finally:
        port.close()
    assert [d.driver_name for d in devices] == ["FakeGrblDriver"]
    assert calls == []


@pytest.mark.asyncio
async def test_network_scan_timeout_returns_empty(monkeypatch):
    async def slow_scan(service_types):
        await asyncio.sleep(1.0)
        return []

    monkeypatch.setattr(mdns_channel, "scan_mdns_services", slow_scan)
    monkeypatch.setattr(mdns_channel, "NETWORK_SCAN_TIMEOUT", 0.05)
    assert await find_network_devices([FakeOctoPrintDriver]) == []


@pytest.mark.asyncio
async def test_network_scan_failure_returns_empty(monkeypatch):
    async def broken_scan(service_types):
        raise OSError("boom")

    monkeypatch.setattr(mdns_channel, "scan_mdns_services", broken_scan)
    assert await find_network_devices([FakeOctoPrintDriver]) == []


@pytest.mark.asyncio
async def test_network_devices_real_browse(monkeypatch):
    """End to end without a scanner fake: the channel hands the
    declared types to the real Rust browse as a set."""
    zeroconf = pytest.importorskip("zeroconf")
    from zeroconf.asyncio import AsyncZeroconf

    from raygeo.driver.core.discovery import mdns_scan

    monkeypatch.setattr(
        mdns_channel, "scan_mdns_services", mdns_scan.scan_mdns_services
    )
    service_type = "_octoprint._tcp.local."
    info = zeroconf.ServiceInfo(
        service_type,
        f"OctoPrint on octopi.{service_type}",
        port=80,
        addresses=[b"\xc0\xa8\x01\x2a"],  # 192.168.1.42
        server="octopi.local.",
    )
    aiozc = AsyncZeroconf()
    await aiozc.async_register_service(info)
    try:
        devices = await find_network_devices([FakeOctoPrintDriver])
    finally:
        await aiozc.async_unregister_service(info)
        await aiozc.async_close()
    assert len(devices) == 1
    assert devices[0].params == {"host": "192.168.1.42", "port": 80}


# ----- fingerprint pass -------------------------------------------------


def _fingerprint_driver(fingerprint, name="FingerprintDriver"):
    """A fresh driver class claiming _esp3d._tcp by declaration and
    _http._tcp candidates only via its fingerprint probe."""

    class FingerprintDriver:
        DISCOVERY = DiscoverySpec(
            mdns=MdnsRecognizer(
                services=("_esp3d._tcp.local.",),
                fingerprint=fingerprint,
            )
        )
        label = "FluidNC / ESP3D (Web API)"

    FingerprintDriver.__name__ = name
    return FingerprintDriver


def _http_service(host="192.168.1.70", port=80, **txt):
    return MDNSService(
        service_type="_http._tcp",
        name="FluidNC",
        host=host,
        port=port,
        server="fluidnc.local",
        txt=dict(txt),
    )


@pytest.mark.asyncio
async def test_http_candidate_fingerprinted_and_claimed(monkeypatch):
    """A FluidNC announcing only generic _http._tcp is confirmed by
    the fingerprint probe and claimed by the driver."""

    async def ok_fingerprint(host, port, timeout=1.5):
        return _http_service()

    async def fake_scan(service_types):
        assert service_types == {"_esp3d._tcp", "_http._tcp"}
        return [_http_service()]

    driver = _fingerprint_driver(ok_fingerprint)
    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    devices = await find_network_devices([driver])
    assert len(devices) == 1
    device = devices[0]
    assert device.driver_name == "FingerprintDriver"
    assert device.params == {"host": "192.168.1.70", "port": 80}
    assert device.label == "FluidNC / ESP3D (Web API)"


@pytest.mark.asyncio
async def test_non_matching_http_candidate_ignored(monkeypatch):
    async def no_match(host, port, timeout=1.5):
        return None

    async def fake_scan(service_types):
        return [_http_service()]

    driver = _fingerprint_driver(no_match)
    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    assert await find_network_devices([driver]) == []


@pytest.mark.asyncio
async def test_declared_service_wins_over_fingerprint(monkeypatch):
    """A host already claimed via its declared service type must not
    be fingerprinted again."""
    calls = []

    async def spy_fingerprint(host, port, timeout=1.5):
        calls.append((host, port))
        return _http_service(host=host)

    async def fake_scan(service_types):
        return [
            MDNSService(
                service_type="_esp3d._tcp",
                name="ESP3D",
                host="192.168.1.60",
                port=80,
                server="esp3d.local",
                txt={},
            ),
            _http_service(host="192.168.1.60"),
        ]

    driver = _fingerprint_driver(spy_fingerprint)
    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    devices = await find_network_devices([driver])
    assert len(devices) == 1
    assert devices[0].params["host"] == "192.168.1.60"
    assert devices[0].identity.banner == "ESP3D"
    assert calls == []


@pytest.mark.asyncio
async def test_failing_fingerprint_is_isolated(monkeypatch):
    """One candidate failing (or refusing) to fingerprint does not
    prevent other candidates from being claimed."""

    async def match_only_second(host, port, timeout=1.5):
        if host == "192.168.1.71":
            return _http_service(host=host)
        return None

    async def fake_scan(service_types):
        return [_http_service(), _http_service(host="192.168.1.71")]

    driver = _fingerprint_driver(match_only_second)
    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    devices = await find_network_devices([driver])
    assert [d.params["host"] for d in devices] == ["192.168.1.71"]


@pytest.mark.asyncio
async def test_first_declared_driver_claims_host(monkeypatch):
    """When several fingerprinting drivers match the same host, the
    first one in declaration order claims it; every host yields at
    most one device."""

    async def always_match(host, port, timeout=1.5):
        return _http_service(host=host)

    first = _fingerprint_driver(always_match, name="FirstDriver")
    second = _fingerprint_driver(always_match, name="SecondDriver")

    async def fake_scan(service_types):
        return [_http_service(), _http_service(host="192.168.1.71")]

    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    devices = await find_network_devices([first, second])
    assert [d.params["host"] for d in devices] == [
        "192.168.1.70",
        "192.168.1.71",
    ]
    assert all(d.driver_name == "FirstDriver" for d in devices)


@pytest.mark.asyncio
async def test_no_candidates_skips_fingerprint_pass(monkeypatch):
    """Without _http._tcp announcements no probe runs; declared-service
    browsing works unchanged."""

    async def spy_fingerprint(host, port, timeout=1.5):
        raise AssertionError("must not be called")

    async def fake_scan(service_types):
        assert service_types == {"_esp3d._tcp"}
        return []

    driver = _fingerprint_driver(spy_fingerprint)
    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    assert await find_network_devices([driver]) == []


def _telnet_fingerprint_driver(fingerprint, name="TelnetFingerprintDriver"):
    """A driver that declares no service types and offers only
    _telnet._tcp candidates to its fingerprint probe."""

    class TelnetFingerprintDriver:
        DISCOVERY = DiscoverySpec(
            mdns=MdnsRecognizer(
                fingerprint=fingerprint,
                fingerprint_services=("_telnet._tcp.local.",),
            )
        )
        label = "grblHAL (Telnet)"

    TelnetFingerprintDriver.__name__ = name
    return TelnetFingerprintDriver


def _telnet_service(host="192.168.1.80", port=23):
    return MDNSService(
        service_type="_telnet._tcp",
        name="grblHAL",
        host=host,
        port=port,
        server="grblhal.local",
        txt={},
    )


def test_fingerprint_services_default_to_http():
    async def fingerprint(host, port):
        return False

    recognizer = MdnsRecognizer(
        services=("_esp3d._tcp.local.",),
        fingerprint=fingerprint,
    )
    assert recognizer.fingerprint_services == ("_http._tcp",)


@pytest.mark.asyncio
async def test_declared_fingerprint_service_is_browsed(monkeypatch):
    """A recognizer that names _telnet._tcp in fingerprint_services
    extends the browse set with that type instead of _http._tcp."""

    async def fingerprint(host, port):
        return None

    seen = []

    async def fake_scan(service_types):
        seen.append(set(service_types))
        return []

    driver = _telnet_fingerprint_driver(fingerprint)
    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    assert await find_network_devices([driver]) == []
    assert seen == [{"_telnet._tcp"}]


@pytest.mark.asyncio
async def test_telnet_candidate_reaches_only_declaring_recognizer(
    monkeypatch,
):
    """A _telnet._tcp candidate is probed only by the declaration that
    named that type, and its SRV host/port become the device params."""

    calls = []

    async def telnet_fingerprint(host, port):
        calls.append(("telnet", host, port))
        return _telnet_service(host=host, port=port)

    async def http_fingerprint(host, port):
        calls.append(("http", host, port))
        return _http_service(host=host, port=port)

    async def fake_scan(service_types):
        return [_telnet_service()]

    telnet_driver = _telnet_fingerprint_driver(telnet_fingerprint)
    http_driver = _fingerprint_driver(http_fingerprint)
    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    devices = await find_network_devices([telnet_driver, http_driver])
    assert len(devices) == 1
    device = devices[0]
    assert device.driver_name == "TelnetFingerprintDriver"
    assert device.params == {"host": "192.168.1.80", "port": 23}
    assert calls == [("telnet", "192.168.1.80", 23)]


@pytest.mark.asyncio
async def test_no_cross_type_fingerprinting(monkeypatch):
    """Each candidate is probed by its own type's declarations only;
    an _http._tcp candidate never reaches a _telnet._tcp declaration
    and vice versa."""

    calls = []

    async def telnet_fingerprint(host, port):
        calls.append(("telnet", host))
        return None

    async def http_fingerprint(host, port):
        calls.append(("http", host))
        return None

    async def fake_scan(service_types):
        return [_telnet_service(), _http_service()]

    telnet_driver = _telnet_fingerprint_driver(telnet_fingerprint)
    http_driver = _fingerprint_driver(http_fingerprint)
    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    assert await find_network_devices([telnet_driver, http_driver]) == []
    assert set(calls) == {
        ("telnet", "192.168.1.80"),
        ("http", "192.168.1.70"),
    }


@pytest.mark.asyncio
async def test_non_matching_telnet_fingerprint_ignored(monkeypatch):
    """An unrelated telnet listener is not mislabeled: the candidate
    is only claimed when the fingerprint probe confirms it."""

    async def no_match(host, port):
        return None

    async def fake_scan(service_types):
        return [_telnet_service()]

    driver = _telnet_fingerprint_driver(no_match)
    monkeypatch.setattr(mdns_channel, "scan_mdns_services", fake_scan)
    assert await find_network_devices([driver]) == []
