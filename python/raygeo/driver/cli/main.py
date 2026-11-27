"""Command line interface for testing raydriver against devices.

Runs a G-code file against a real GRBL device over serial, or
against the built-in firmware emulator for hardware-free dry runs,
showing live progress while the job executes:

    raydriver run job.gcode --port /dev/ttyUSB0
    raydriver run job.gcode --port /dev/ttyUSB0 --log-file job.log
    raydriver run job.gcode --emulator
    raydriver run job.rds --protocol ruida --host 192.168.1.50
    raydriver run job.rds --protocol ruida --emulator --no-wait
    raydriver status --port /dev/ttyUSB0 --seconds 5

Ruida controllers consume rpascript rather than G-code.  ``run``
sends the whole script and, unless ``--no-wait`` is given, polls
the controller until it reports idle again.

Driver diagnostics (stall detection, liveness, unexpected device
lines) go to stderr by default; ``--log-file`` redirects them to a
dedicated file so they never interleave with the progress display.
Verbosity follows ``RUST_LOG`` (e.g. ``RUST_LOG=debug``).
"""

import argparse
import asyncio
import inspect
import os
import sys
import time
from collections.abc import Awaitable

import raygeo.raygeo as _native
from raygeo.driver.cli.analyzer import (
    add_analyzer_parser,
    add_protocol_arguments,
    open_source_session,
)
from raygeo.driver.cli.daemon import add_daemon_parser
from raygeo.driver.cli.debug import add_debug_parser
from raygeo.driver.core import DriverEvent
from raygeo.driver.core.transport import MockTransport
from raygeo.driver.core.types import DeviceState
from raygeo.driver.grbl import GrblSerialDriver
from raygeo.driver.grbl.parser import strip_gcode_comments
from raygeo.driver.ruida import RuidaDriver


def _fmt_seconds(seconds):
    minutes, secs = divmod(int(seconds), 60)
    return f"{minutes:02d}:{secs:02d}"


class RunView:
    """Single-line live progress renderer for a streaming job.

    On a TTY the display is a ``\\r``-animated single line.  When
    stdout is redirected (no TTY), updates are newline-terminated and
    rate-limited so interleaved log lines stay intact.
    """

    def __init__(self, total_lines, out=None):
        self.total = total_lines
        self.out = out
        self.acked = 0
        self.state: DeviceState | None = None
        self.had_error = False
        self.started = time.monotonic()
        self._last_render = 0.0
        self.finished = False
        self._stream = self._stdout()
        self._tty = bool(getattr(self._stream, "isatty", lambda: False)())
        self._min_interval = 0.05 if self._tty else 1.0

    def _stdout(self):
        return self.out if self.out is not None else sys.stdout

    def progress(self, line_index):
        self.acked = max(self.acked, line_index + 1)
        self.render()

    def on_state(self, state: DeviceState):
        self.state = state
        if state.error is not None:
            self.had_error = True
        self.render()

    def render(self, force=False):
        if self.finished and not force:
            return
        now = time.monotonic()
        if not force and now - self._last_render < self._min_interval:
            return
        self._last_render = now
        elapsed = now - self.started
        frac = self.acked / self.total if self.total else 1.0
        bar_width = 20
        filled = int(bar_width * frac)
        bar = "#" * filled + "-" * (bar_width - filled)
        parts = [f" {self.acked}/{self.total} [{bar}] {frac:6.1%}"]
        if self.state is not None:
            mpos = ",".join(
                f"{v:.3f}" if v is not None else "  -  "
                for v in self.state.machine_pos
            )
            state = self.state.status.name
            feed = (
                f" F{self.state.feed_rate}"
                if self.state.feed_rate is not None
                else ""
            )
            parts.append(f"{state} MPos:{mpos}{feed}")
        if frac > 0:
            eta = elapsed / frac - elapsed
            parts.append(f"eta {_fmt_seconds(eta)}")
        parts.append(f"{_fmt_seconds(elapsed)}")
        line = " | ".join(parts)
        if self._tty:
            print(f"\r{line:<100}", end="", flush=True, file=self._stream)
        else:
            # Newline-terminated so redirected logs and progress
            # lines never share a physical line.
            self._stream.write(line + "\n")
            self._stream.flush()

    def finish(self, success, message=""):
        self.finished = False
        self.render(force=True)
        self.finished = True
        elapsed = time.monotonic() - self.started
        print(file=self._stdout())
        status = "done" if success else f"ABORTED — {message}"
        print(
            f" {status}: {self.acked}/{self.total} lines acknowledged "
            f"in {_fmt_seconds(elapsed)}",
            file=self._stdout(),
        )


def _open_session(args, on_event):
    poll = not getattr(args, "no_poll", False)
    config = {"poll_status_while_running": poll}
    if args.emulator:
        from raygeo.driver.grbl.emulator import GrblEmulator

        mock = MockTransport()
        emulator = GrblEmulator(mock, speed_factor=args.speed_factor)
        session = GrblSerialDriver.with_transport(
            mock,
            config=config,
            dialect={},
            event_callback=on_event,
        )
        return session, asyncio.ensure_future(emulator.run())
    return (
        GrblSerialDriver(
            config={
                "port": args.port,
                "baudrate": args.baudrate,
                **config,
            },
            dialect={},
            event_callback=on_event,
        ),
        None,
    )


#: How long to wait for the controller to report RUN before
#: concluding that a script did not start a job.
_RUIDA_START_GRACE = 3.0


async def _wait_for_ruida_idle(session, timeout: float) -> bool:
    """Wait until a sent Ruida job has finished.

    Ruida has no job-complete event, so poll the machine status:
    wait for RUN/HOLD, then for IDLE.  Scripts that only send live
    commands never leave IDLE; treat them as done after a grace
    period.  *timeout* <= 0 means no limit.
    """
    deadline = None if timeout <= 0 else time.monotonic() + timeout
    start_deadline = time.monotonic() + _RUIDA_START_GRACE
    started = False
    while True:
        if session.state.error is not None:
            return False
        status = session.state.status.name
        if status in ("RUN", "HOLD"):
            started = True
        elif started and status == "IDLE":
            return True
        now = time.monotonic()
        if not started and now >= start_deadline:
            return True
        if deadline is not None and now >= deadline:
            return False
        await asyncio.sleep(0.2)


async def _run_ruida(args) -> int:
    """Run an rpascript job on a Ruida controller."""
    with open(args.file) as handle:
        script = handle.read()
    lines = [line for line in script.splitlines() if line.strip()]
    if not lines:
        print("error: no rpascript lines in file", file=sys.stderr)
        return 2

    connected = asyncio.get_running_loop().create_future()

    def on_event(event: DriverEvent):
        match event:
            case DriverEvent.ConnectionStatusChanged(status="CONNECTED") if (
                not connected.done()
            ):
                connected.set_result(True)
            case DriverEvent.ConnectionStatusChanged(
                status="ERROR", message=message
            ) if not connected.done():
                connected.set_exception(
                    ConnectionError(message or "connection failed")
                )

    try:
        session, device_task, source, cleanup = await open_source_session(
            args, on_event
        )
    except ValueError as err:
        print(f"error: {err}", file=sys.stderr)
        return 2
    assert isinstance(session, RuidaDriver)

    print(f"Connecting to {source} ...")
    try:
        await session.connect()
        try:
            await asyncio.wait_for(connected, timeout=args.timeout)
        except asyncio.TimeoutError:
            print("error: controller did not respond", file=sys.stderr)
            return 1
        version = (
            session.machine_status.get("mainboard_version")
            or "Ruida controller"
        )
        print(f"Device: {version}")
        print(
            f"Job: {len(lines)} rpascript lines ({len(script.encode())} bytes)"
        )
        await session.run_raw(script)
        if args.no_wait:
            print("job sent")
            return 0
        print("waiting for the controller to become idle ...")
        if await _wait_for_ruida_idle(session, args.wait_timeout):
            print("done: controller idle")
            return 0
        error = session.state.error
        if error is not None:
            message = f"{error.title}: {error.description}"
        else:
            message = "controller did not become idle"
        print(f"error: {message}", file=sys.stderr)
        return 1
    except asyncio.CancelledError:
        print("\nInterrupted — stopping the job ...")
        await asyncio.shield(session.cancel(emergency=True))
        raise
    except Exception as err:  # noqa: BLE001 - surfaced to the user
        print(f"error: {err}", file=sys.stderr)
        return 1
    finally:
        try:
            await asyncio.shield(session.disconnect())
        except Exception:  # noqa: BLE001 - teardown best effort
            pass
        if device_task is not None:
            device_task.cancel()
        if cleanup is not None:
            try:
                await cleanup()
            except Exception:  # noqa: BLE001 - teardown best effort
                pass


async def cmd_run(args) -> int:
    if getattr(args, "protocol", "grbl") == "ruida":
        return await _run_ruida(args)
    with open(args.file) as handle:
        text = handle.read()
    lines = [raw for raw in text.splitlines() if strip_gcode_comments(raw)]
    if not lines:
        print("error: no G-code lines in file", file=sys.stderr)
        return 2

    view = RunView(len(lines))

    connected = asyncio.get_running_loop().create_future()

    def on_event(event: DriverEvent):
        match event:
            case DriverEvent.StateChanged():
                view.on_state(event.state)
            case DriverEvent.ConnectionStatusChanged(status="CONNECTED") if (
                not connected.done()
            ):
                connected.set_result(True)
            case DriverEvent.ConnectionStatusChanged(
                status="ERROR", message=message
            ) if not connected.done():
                connected.set_exception(
                    ConnectionError(message or "connection failed")
                )

    session, device_task = _open_session(args, on_event)
    print(
        "Connecting to "
        + ("emulated device" if args.emulator else args.port)
        + " ..."
    )
    try:
        await session.connect()
        try:
            await asyncio.wait_for(connected, timeout=args.timeout)
        except asyncio.TimeoutError:
            print("error: device did not respond", file=sys.stderr)
            return 1
        info = await session.execute_interactive_command("$I")
        print("Device: " + " | ".join(info))

        op_map = {i: i for i in range(len(lines))}
        await session.run(text, op_map, [], view.progress)
        # Read the state snapshot synchronously: the events that
        # carry the error may still be in flight when run() returns.
        error = session.state.error
        success = view.acked >= len(lines) and error is None
        message = "job did not complete"
        if error is not None:
            message = f"{error.title}: {error.description}"
        view.finish(success, message)
        return 0 if success else 1
    except asyncio.CancelledError:
        print("\nInterrupted — cancelling job ...")
        await asyncio.shield(session.cancel(emergency=True))
        view.finish(False, "interrupted")
        raise
    finally:
        try:
            await asyncio.shield(session.disconnect())
        except Exception:  # noqa: BLE001 - teardown best effort
            pass
        if device_task is not None:
            device_task.cancel()


async def cmd_status(args) -> int:
    def on_event(event: DriverEvent):
        match event:
            case DriverEvent.StateChanged():
                state = event.state
                mpos = ",".join(
                    f"{v:8.3f}" if v is not None else "     -  "
                    for v in state.machine_pos
                )
                error = f" ! {state.error.title}" if state.error else ""
                print(
                    f"{time.strftime('%H:%M:%S')} {state.status.name:8}"
                    f" MPos:{mpos}{error}"
                )

    session, device_task = _open_session(args, on_event)
    try:
        await session.connect()
        await asyncio.sleep(args.seconds)
    finally:
        await session.disconnect()
        if device_task is not None:
            device_task.cancel()
    return 0


def build_parser():
    parser = argparse.ArgumentParser(
        prog="raydriver",
        description="Test raydriver sessions against real devices "
        "or the built-in firmware emulators.",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    def common(run_parser):
        run_parser.add_argument(
            "--port", default="/dev/ttyUSB0", help="serial port (GRBL)"
        )
        run_parser.add_argument(
            "--baudrate", type=int, default=115200, help="baud rate (GRBL)"
        )
        run_parser.add_argument(
            "--emulator",
            action="store_true",
            help="run against the built-in firmware emulator instead "
            "of hardware",
        )
        run_parser.add_argument(
            "--speed-factor",
            type=float,
            default=2000.0,
            help="emulator motion speed multiplier (GRBL only)",
        )
        run_parser.add_argument(
            "--log-file",
            default=None,
            help="write Rust-side session diagnostics to this file "
            "(default: stderr).  Level defaults to info for the file "
            "and can be overridden with RUST_LOG",
        )

    run_parser = subparsers.add_parser(
        "run", help="stream a G-code (GRBL) or rpascript (Ruida) file"
    )
    common(run_parser)
    add_protocol_arguments(run_parser, choices=["grbl", "ruida"])
    run_parser.add_argument(
        "file", help="G-code file (GRBL) or rpascript file (Ruida)"
    )
    run_parser.add_argument(
        "--timeout",
        type=float,
        default=10.0,
        help="seconds to wait for the device to connect",
    )
    run_parser.add_argument(
        "--no-poll",
        action="store_true",
        help="disable status polling during the job (GRBL only)",
    )
    run_parser.add_argument(
        "--no-wait",
        action="store_true",
        help="return after sending; do not wait for the controller "
        "to become idle (Ruida only)",
    )
    run_parser.add_argument(
        "--wait-timeout",
        type=float,
        default=0.0,
        help="seconds to wait for the job to finish after sending "
        "(0: no limit; Ruida only)",
    )
    run_parser.set_defaults(func=cmd_run)

    status_parser = subparsers.add_parser(
        "status", help="connect and print live status reports"
    )
    common(status_parser)
    status_parser.add_argument(
        "--seconds", type=float, default=5.0, help="duration to watch"
    )
    status_parser.set_defaults(func=cmd_status)

    add_analyzer_parser(subparsers)
    add_daemon_parser(subparsers)
    add_debug_parser(subparsers)
    return parser


def _configure_logging(args) -> None:
    # Not every subcommand defines --log-file (e.g. the debug
    # commands); default to stderr logging when it is absent.
    log_file = getattr(args, "log_file", None)
    if log_file and "RUST_LOG" not in os.environ:
        # Diagnostics belong in the log file; info level by default.
        os.environ["RUST_LOG"] = "info"
    _native.init_logging(path=log_file)


async def _await_result(result: Awaitable[int]) -> int:
    return await result


async def amain(argv) -> int:
    args = build_parser().parse_args(argv)
    _configure_logging(args)
    result = args.func(args)
    if inspect.isawaitable(result):
        return await _await_result(result)
    return result


def main() -> None:
    try:
        # The TUI subcommand owns its event loop (curses needs the
        # main thread), so dispatch outside asyncio.run when the
        # command is synchronous.
        args = build_parser().parse_args(sys.argv[1:])
        _configure_logging(args)
        result = args.func(args)
        if inspect.isawaitable(result):
            result = asyncio.run(_await_result(result))
        sys.exit(result)
    except KeyboardInterrupt:
        sys.exit(130)


if __name__ == "__main__":
    main()
