import asyncio
import ssl
from pathlib import Path

import pytest

from raygeo.driver.xtool.m2 import XtoolM2Driver
from raygeo.driver.xtool.m2.emulator import M2Emulator


def test_session_exposes_m2_resource_uri_and_state():
    session = XtoolM2Driver(config={"host": "192.0.2.20"})

    assert session.resource_uri == "wss://192.0.2.20:28900/websocket"
    assert session.state.status.name == "UNKNOWN"
    assert session.is_job_running is False


@pytest.mark.asyncio
async def test_session_has_control_operations_but_no_gcode_streamer():
    session = XtoolM2Driver(config={"host": "192.0.2.20"})

    assert callable(session.start)
    assert callable(session.set_hold)
    assert callable(session.cancel)
    # The uniform surface exposes job streaming; the M2 protocol has
    # no G-code upload, so it raises rather than being absent.
    with pytest.raises(NotImplementedError):
        await session.run("G1 X1")


@pytest.mark.asyncio
async def test_operations_fail_cleanly_before_connecting():
    session = XtoolM2Driver(config={"host": "192.0.2.20"})

    with pytest.raises(ConnectionError, match="not connected"):
        await session.home()


@pytest.mark.asyncio
async def test_session_connects_and_controls_m2_jobs():
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    certs = Path(__file__).parents[1] / "certs"
    context.load_cert_chain(
        certs / "analyzer-test-cert.pem",
        certs / "analyzer-test-key.pem",
    )
    emulator = M2Emulator(context)
    await emulator.start()
    session = XtoolM2Driver(
        config={
            "host": "127.0.0.1",
            "port": emulator.port,
            "poll_interval": 5.0,
            "tls_allow_invalid_certs": True,
        }
    )
    try:
        await session.connect()
        await wait_until(lambda: session.state.status.name == "IDLE")
        await wait_until(lambda: session.state.machine_pos[0] == 1.0)

        await session.home()
        await session.move_to(18000.0, 10.0, 20.0)
        await session.jog(18000.0, [("x", 1.0), ("y", -1.0)])
        await session.start()
        await wait_until(lambda: session.state.status.name == "RUN")
        await session.set_hold(True)
        await wait_until(lambda: session.state.status.name == "HOLD")
        await session.set_hold(False)
        await wait_until(lambda: session.state.status.name == "RUN")
        await session.cancel()
        await wait_until(lambda: session.state.status.name == "IDLE")

        actions = [
            request["params"]["action"]
            for request in emulator.requests
            if request["url"] == "/v1/project/device/control"
        ]
        assert actions == ["START", "PAUSE", "RESUME", "CANCEL"]
        urls = [request["url"] for request in emulator.requests]
        assert "/v1/user/parity" in urls
        assert "/v1/project/control/home" in urls
        assert "/v1/project/control/absolute-move" in urls
        assert "/v1/project/control/relative-move" in urls
    finally:
        await session.disconnect()
        await emulator.close()


async def wait_until(predicate, timeout=3.0):
    loop = asyncio.get_running_loop()
    deadline = loop.time() + timeout
    while loop.time() < deadline:
        if predicate():
            return
        await asyncio.sleep(0.01)
    raise AssertionError("condition was not met")


@pytest.mark.asyncio
async def test_unit_system_is_metric():
    session = XtoolM2Driver(config={"host": "192.0.2.20"})
    assert await session.detect_unit_system() == "metric"


@pytest.mark.asyncio
async def test_local_reads_and_alarm_need_no_connection():
    session = XtoolM2Driver(config={"host": "192.0.2.20"})
    await session.clear_alarm()
    assert await session.read_settings() == []
    assert await session.read_wcs_offsets() == {}


@pytest.mark.asyncio
async def test_single_axis_home_and_z_moves_are_rejected():
    session = XtoolM2Driver(config={"host": "192.0.2.20"})
    with pytest.raises(NotImplementedError):
        await session.home(axes=["x"])
    with pytest.raises(NotImplementedError):
        await session.move_to(1000, 1.0, 2.0, z=0.0)
