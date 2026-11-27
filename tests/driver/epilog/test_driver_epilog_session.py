"""Epilog session tests, driven against an in-process fake LPD
printer.

The fake printer speaks the BSD "receive job" exchange the machine
expects, so the tests assert the exact wire bytes: the five stages,
their single-byte acks, one connection per job and no retry of a
partial transfer.
"""

import asyncio
from contextlib import asynccontextmanager

import pytest
from conftest import EventRecorder

from raygeo.driver.core import DriverEvent
from raygeo.driver.epilog import EpilogDriver


@asynccontextmanager
async def printer(handler):
    """Run *handler(reader, writer)* for every accepted connection."""
    tasks = set()

    async def connection(reader, writer):
        task = asyncio.current_task()
        tasks.add(task)
        try:
            await handler(reader, writer)
        finally:
            writer.close()
            await writer.wait_closed()
            tasks.discard(task)

    server = await asyncio.start_server(connection, "127.0.0.1", 0)
    port = server.sockets[0].getsockname()[1]
    try:
        yield port
    finally:
        server.close()
        await server.wait_closed()
        for task in list(tasks):
            task.cancel()
        await asyncio.gather(*tasks, return_exceptions=True)


def make_session(port, events, **config):
    return EpilogDriver(
        config={"host": "127.0.0.1", "port": port, "timeout": 2.0, **config},
        event_callback=events.callback,
    )


async def test_upload_performs_the_lpd_exchange():
    received = asyncio.get_running_loop().create_future()
    payload = b"PJL-test-payload" + bytes(4096)

    async def receive(reader, writer):
        try:
            # RFC 1179 §5.2: the receive-job operand is the queue name;
            # it is empty unless the print server needs one.
            assert await reader.readline() == b"\x02\n"
            writer.write(b"\x00")
            control_header = await reader.readline()
            assert control_header.startswith(b"\x02")
            size, control_name = control_header[1:].strip().split(b" ")
            assert control_name.startswith(b"cfA")
            writer.write(b"\x00")
            control = await reader.readexactly(int(size))
            assert await reader.readexactly(1) == b"\x00"
            writer.write(b"\x00")
            data_header = await reader.readline()
            assert data_header.startswith(b"\x03")
            size, data_name = data_header[1:].strip().split(b" ")
            assert data_name == b"df" + control_name[2:]
            assert int(size) == len(payload)
            writer.write(b"\x00")
            assert await reader.readexactly(int(size)) == payload
            assert b"l" + data_name + b"\n" in control
            assert b"U" + data_name + b"\n" in control
            assert b"JJob___E\n" in control
            assert b"NJob___E\n" in control
            assert control.startswith(b"Hrayforge\nPrayforge\n")
            writer.write(b"\x00")
            await writer.drain()
            assert await reader.read() == b""
            received.set_result(True)
        except (AssertionError, OSError, ValueError) as exc:
            received.set_exception(exc)

    events = EventRecorder()
    async with printer(receive) as port:
        session = make_session(port, events)
        await session.upload(payload, "Job\r\n\x1bE")
        assert await asyncio.wait_for(received, 2)
        assert "JobFinished" in events.names()
        await session.disconnect()


@pytest.mark.parametrize("rejected_stage", range(5))
async def test_rejection_closes_without_retry(rejected_stage):
    connections = 0
    closed = asyncio.Event()

    async def reject(reader, writer):
        nonlocal connections
        connections += 1
        size = 0
        for stage in range(5):
            if stage in (0, 1, 3):
                line = await reader.readline()
                if stage in (1, 3):
                    size = int(line[1:].split(b" ")[0])
            else:
                await reader.readexactly(size + (1 if stage == 2 else 0))
            writer.write(b"\x01" if stage == rejected_stage else b"\x00")
            await writer.drain()
            if stage == rejected_stage:
                # A rejection of the receive-job command itself leaves
                # nothing created. Later rejections abort the job
                # (RFC 1179 §6.1) before closing.
                expected = b"" if stage == 0 else b"\x01\n"
                assert await reader.read() == expected
                closed.set()
                return

    events = EventRecorder()
    async with printer(reject) as port:
        session = make_session(port, events)
        with pytest.raises(RuntimeError, match="rejected"):
            await session.upload(b"payload", "test")
        await asyncio.wait_for(closed.wait(), 2)
        await session.disconnect()
    assert connections == 1


async def test_missing_ack_times_out_and_closes():
    closed = asyncio.Event()

    async def silent(reader, writer):
        await reader.readline()
        assert await reader.read() == b""
        closed.set()

    events = EventRecorder()
    async with printer(silent) as port:
        session = make_session(port, events, timeout=0.05)
        with pytest.raises(TimeoutError, match="timed out"):
            await session.upload(b"test", "test")
        await asyncio.wait_for(closed.wait(), 2)
        await session.disconnect()


async def test_disconnect_is_not_success():
    async def disconnect(reader, writer):
        await reader.readline()

    events = EventRecorder()
    async with printer(disconnect) as port:
        session = make_session(port, events)
        with pytest.raises(ConnectionError, match="disconnected"):
            await session.upload(b"test", "test")
        await session.disconnect()


async def test_cancel_closes_connection():
    waiting = asyncio.Event()
    closed = asyncio.Event()

    async def silent(reader, writer):
        await reader.readline()
        waiting.set()
        assert await reader.read() == b""
        closed.set()

    events = EventRecorder()
    async with printer(silent) as port:
        session = make_session(port, events)
        task = asyncio.ensure_future(session.upload(b"test", "test"))
        await asyncio.wait_for(waiting.wait(), 2)
        await session.cancel()
        with pytest.raises(Exception):
            await asyncio.wait_for(task, 2)
        await asyncio.wait_for(closed.wait(), 2)
        await session.disconnect()


async def test_probe_sends_nothing():
    received = asyncio.get_running_loop().create_future()

    async def record(reader, writer):
        received.set_result(await reader.read())

    events = EventRecorder()
    async with printer(record) as port:
        session = make_session(port, events)
        await session.probe()
        assert await asyncio.wait_for(received, 2) == b""
        await session.disconnect()


async def test_connect_reports_reachable_machine():
    async def idle(reader, writer):
        await reader.read()

    events = EventRecorder()
    async with printer(idle) as port:
        session = make_session(port, events)
        await session.connect()
        event = await events.wait_for(
            DriverEvent.ConnectionStatusChanged,
            lambda event: event.status == "CONNECTED",
        )
        assert "reachable" in (event.message or "")
        await session.disconnect()


async def test_connect_failure_reports_error():
    # A closed port is not deterministic: on Windows a SYN to a
    # recently closed port is silently dropped, so the connect times
    # out instead of being refused. An unroutable address always
    # fails, and the session maps it to the same error path.
    events = EventRecorder()
    session = make_session(9, events, host="192.0.2.1", timeout=0.5)
    with pytest.raises((ConnectionError, TimeoutError)):
        await session.connect()
    await events.wait_for(
        DriverEvent.ConnectionStatusChanged,
        lambda event: event.status == "ERROR",
    )
    await session.disconnect()


async def test_empty_payload_is_rejected():
    events = EventRecorder()
    async with printer(lambda r, w: r.read()) as port:
        session = make_session(port, events)
        with pytest.raises(RuntimeError, match="empty"):
            await session.upload(b"", "test")
        await session.disconnect()


async def test_gcode_run_is_unsupported():
    events = EventRecorder()
    async with printer(lambda r, w: r.read()) as port:
        session = make_session(port, events)
        with pytest.raises(NotImplementedError):
            await session.run("G1 X0")
        await session.disconnect()


async def test_machine_space_and_single_wcs():
    events = EventRecorder()
    async with printer(lambda r, w: r.read()) as port:
        session = make_session(port, events)
        assert await session.read_parser_state() == "MACHINE"
        assert await session.read_wcs_offsets() == {"MACHINE": (0.0, 0.0, 0.0)}
        await session.disconnect()


def test_driver_info():
    from raygeo.driver import driver_info

    info = driver_info("EpilogDriver")
    assert info.name == "EpilogDriver"
    assert info.connection == "network"
    assert hasattr(EpilogDriver, "upload")
    assert info.capabilities["uses_gcode"] is False


async def test_abort_is_sent_when_the_transfer_fails_mid_job():
    """RFC 1179 §6.1: a client that gives up mid-job sends 0x01 and
    reads its ack, so the daemon drops the partial job."""
    closed = asyncio.Event()
    received = []

    async def reject_data(reader, writer):
        size = 0
        for stage in range(5):
            if stage in (0, 1, 3):
                line = await reader.readline()
                received.append(line)
                if stage in (1, 3):
                    size = int(line[1:].split(b" ")[0])
            else:
                await reader.readexactly(size + (1 if stage == 2 else 0))
            writer.write(b"\x01" if stage == 3 else b"\x00")
            await writer.drain()
            if stage == 3:
                received.append(await reader.read())
                closed.set()
                return

    events = EventRecorder()
    async with printer(reject_data) as port:
        session = make_session(port, events)
        with pytest.raises(RuntimeError, match="rejected"):
            await session.upload(b"payload-bytes", "job")
        await asyncio.wait_for(closed.wait(), 2)
        await session.disconnect()
    assert received[-1] == b"\x01\n"


async def test_queue_name_is_configurable():
    first_line = asyncio.get_running_loop().create_future()

    async def record(reader, writer):
        first_line.set_result(await reader.readline())

    events = EventRecorder()
    async with printer(record) as port:
        session = make_session(port, events, queue="Engraver")
        task = asyncio.ensure_future(session.upload(b"x", "job"))
        assert await asyncio.wait_for(first_line, 2) == b"\x02Engraver\n"
        task.cancel()
        await asyncio.gather(task, return_exceptions=True)
        await session.disconnect()


async def test_blank_queue_is_sent_empty():
    first_line = asyncio.get_running_loop().create_future()

    async def record(reader, writer):
        first_line.set_result(await reader.readline())

    events = EventRecorder()
    async with printer(record) as port:
        session = make_session(port, events, queue="   ")
        task = asyncio.ensure_future(session.upload(b"x", "job"))
        assert await asyncio.wait_for(first_line, 2) == b"\x02\n"
        task.cancel()
        await asyncio.gather(task, return_exceptions=True)
        await session.disconnect()


async def test_concurrent_job_numbers_do_not_collide():
    """Two jobs started in the same instant must get distinct file
    names, or the machine merges them."""
    names = []
    seen = asyncio.Event()

    async def record(reader, writer):
        await reader.readline()
        writer.write(b"\x00")
        header = await reader.readline()
        names.append(header)
        if len(names) >= 2:
            seen.set()

    events = EventRecorder()
    async with printer(record) as port:
        first = make_session(port, events)
        second = make_session(port, events)
        task_a = asyncio.ensure_future(first.upload(b"a", "job"))
        task_b = asyncio.ensure_future(second.upload(b"b", "job"))
        await asyncio.wait_for(seen.wait(), 2)
        task_a.cancel()
        task_b.cancel()
        await asyncio.gather(task_a, task_b, return_exceptions=True)
        await first.disconnect()
        await second.disconnect()
    assert names[0].split(b" ")[1] != names[1].split(b" ")[1]
