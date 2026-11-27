"""Tests for raygeo.driver.analyzer.proxy and raygeo.driver.analyzer.replay."""

import asyncio

from conftest import server_port

from raygeo.driver.analyzer.capture import Capture, Frame
from raygeo.driver.analyzer.proxy import run_proxy
from raygeo.driver.analyzer.replay import replay_capture


async def start_fake_device(handler):
    server = await asyncio.start_server(handler, "127.0.0.1", 0)
    return server, server_port(server)


async def grblish_device(reader, writer):
    """Answers $I with a version block and echoes ok per line."""
    try:
        while True:
            line = await reader.readline()
            if not line:
                return
            text = line.decode().strip()
            if text == "$I":
                writer.write(b"[VER:1.1h.20190825:]\n[OPT:V,15,128]\nok\n")
            else:
                writer.write(b"ok\n")
            await writer.drain()
    finally:
        writer.close()


async def test_proxy_records_both_directions():
    device, device_port = await start_fake_device(grblish_device)
    proxy = await run_proxy("127.0.0.1", device_port)
    try:
        reader, writer = await asyncio.open_connection("127.0.0.1", proxy.port)
        writer.write(b"$I\n")
        await writer.drain()
        assert await reader.readline() == b"[VER:1.1h.20190825:]\n"
        assert await reader.readline() == b"[OPT:V,15,128]\n"
        assert await reader.readline() == b"ok\n"
        writer.write(b"G1 X10\n")
        await writer.drain()
        assert await reader.readline() == b"ok\n"
        writer.close()
        await writer.wait_closed()
        await asyncio.sleep(0.05)

        capture = proxy.capture
        texts = [
            (frame.direction, frame.text)
            for frame in capture.frames
            if frame.note is None
        ]
        assert ("tx", "$I") in texts
        assert ("rx", "[VER:1.1h.20190825:]") in texts
        assert ("rx", "[OPT:V,15,128]") in texts
        assert ("rx", "ok") in texts
        assert ("tx", "G1 X10") in texts
        notes = [f.note for f in capture.frames if f.note is not None]
        assert any(note.startswith("client connected") for note in notes)
        assert any(note.startswith("client disconnected") for note in notes)
    finally:
        await proxy.stop()
        device.close()
        await device.wait_closed()


async def test_proxy_records_binary_chunks_raw():
    async def echo(reader, writer):
        try:
            while chunk := await reader.read(256):
                writer.write(chunk)
                await writer.drain()
        finally:
            writer.close()

    device, device_port = await start_fake_device(echo)
    proxy = await run_proxy("127.0.0.1", device_port, framing="raw")
    try:
        reader, writer = await asyncio.open_connection("127.0.0.1", proxy.port)
        writer.write(b"\x88\xa8\x00")
        await writer.drain()
        assert await reader.readexactly(3) == b"\x88\xa8\x00"
        writer.close()
        await asyncio.sleep(0.05)
        data_frames = [
            frame for frame in proxy.capture.frames if frame.data is not None
        ]
        assert any(
            (frame.data or b"").startswith(b"\x88") for frame in data_frames
        )
    finally:
        await proxy.stop()
        device.close()
        await device.wait_closed()


async def test_proxy_rejects_unknown_framing():
    try:
        await run_proxy("127.0.0.1", 1, framing="bogus")
    except ValueError as err:
        assert "unknown framing" in str(err)
    else:
        raise AssertionError("expected ValueError")


async def test_replay_through_proxy():
    """End-to-end: replay a capture into a proxy in front of a
    device, at speed 0, and collect the device responses."""
    device, device_port = await start_fake_device(grblish_device)
    proxy = await run_proxy("127.0.0.1", device_port)
    try:
        capture = Capture(protocol="grbl")
        capture.append("tx", text="$I")
        capture.append("rx", text="ok")
        capture.append("tx", text="G1 X10")

        reader, writer = await asyncio.open_connection("127.0.0.1", proxy.port)

        async def send(data):
            writer.write(data)
            await writer.drain()

        written = await replay_capture(capture, send, speed=0)
        assert written == 2
        responses = []
        while len(responses) < 4:
            line = await asyncio.wait_for(reader.readline(), timeout=5.0)
            if not line:
                break
            responses.append(line.decode().strip())
        writer.close()
        assert responses == [
            "[VER:1.1h.20190825:]",
            "[OPT:V,15,128]",
            "ok",
            "ok",
        ]
    finally:
        await proxy.stop()
        device.close()
        await device.wait_closed()


async def test_replay_paces_frames():
    wrote = []

    async def write(data):
        wrote.append((asyncio.get_running_loop().time(), data))

    async def fake_sleep(delay):
        wrote.append(("delay", delay))

    capture = Capture()
    capture.frames = [
        Frame("tx", 0.0, text="$I"),
        Frame("rx", 0.5, text="ok"),
        Frame("tx", 1.0, text="$$"),
        Frame("tx", 1.5, text="$G"),
    ]
    written = await replay_capture(capture, write, speed=2.0, sleep=fake_sleep)
    assert written == 3
    payloads = [entry[1] for entry in wrote if entry[0] != "delay"]
    delays = [entry[1] for entry in wrote if entry[0] == "delay"]
    assert payloads == [b"$I\n", b"$$\n", b"$G\n"]
    # 1.0s gap / speed 2.0 between the first two tx frames, then
    # 0.5s / speed 2.0.
    assert delays == [0.5, 0.25]


async def test_replay_writes_binary_frames_verbatim():
    wrote = []

    async def write(data):
        wrote.append(data)

    capture = Capture()
    capture.frames = [Frame("tx", 0.0, data=b"\x88\xa8\x00")]
    assert await replay_capture(capture, write, speed=0) == 1
    assert wrote == [b"\x88\xa8\x00"]
