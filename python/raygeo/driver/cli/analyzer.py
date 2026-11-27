"""Analyzer CLI: capture, decode, replay and proxy device traffic.

    raydriver analyzer record --port /dev/ttyUSB0 --out cap.jsonl
    raydriver analyzer record --emulator            # hardware-free
    raydriver analyzer tui --port /dev/ttyUSB0      # live monitor
    raydriver analyzer record --protocol ruida --host 192.168.1.50
    raydriver analyzer tui --protocol ruida --emulator
    raydriver analyzer decode cap.jsonl
    raydriver analyzer replay cap.jsonl --to 192.168.1.50:23
    raydriver analyzer proxy --upstream 192.168.1.50:23 --out mitm.jsonl

Recording attaches to a GRBL or Ruida session and logs every line
or wire chunk both ways; the proxy instead sits between a network
application and a TCP device (the man-in-the-middle workflow for
characterizing controllers and other applications).  Network
endpoints follow the analyzer security policy: loopback by default,
token and TLS required for exposed listeners.
"""

import argparse
import asyncio
import sys
import time

from raygeo.driver.analyzer.capture import Capture, Frame, Recorder
from raygeo.driver.analyzer.decode import (
    decoder_names,
    format_frame,
    get_decoder,
    render_listing,
)
from raygeo.driver.analyzer.listener import connect_authenticated
from raygeo.driver.analyzer.proxy import (
    FRAMING_LINES,
    FRAMING_RAW,
    run_proxy,
)
from raygeo.driver.analyzer.replay import replay_capture
from raygeo.driver.core.transport import MockTransport
from raygeo.driver.grbl import GrblSerialDriver
from raygeo.driver.ruida import RuidaDriver

#: Short timeouts so the hardware-free Ruida emulator shows live
#: traffic instead of waiting out the production query interval.
_RUIDA_EMULATOR_TIMINGS = {
    "timeout": 0.5,
    "query_interval": 0.2,
    "ping_retries": 3,
    "ping_delay": 0.05,
}


def _parse_endpoint(value: str) -> tuple[str, int]:
    host, _, port = value.rpartition(":")
    if not host or not port.isdigit():
        raise argparse.ArgumentTypeError(f"expected HOST:PORT, got {value!r}")
    return host, int(port)


def _ruida_config(args) -> dict:
    """Ruida connection parameters from the CLI."""
    config = {
        "port": args.net_port,
        "protocol": args.transport,
        "magic_number": f"0x{args.magic:02x}",
    }
    if args.usb_device:
        config["usb_device"] = args.usb_device
    else:
        config["host"] = args.host
    return config


def _source_label(args) -> str:
    """A human-readable source name for the listing header."""
    if args.protocol == "ruida":
        if args.emulator:
            return "emulated Ruida controller"
        if args.usb_device:
            return f"Ruida USB {args.usb_device}"
        return f"Ruida {args.transport}://{args.host}:{args.net_port}"
    if args.emulator:
        return "emulated GRBL device"
    return f"{args.port} @ {args.baudrate}"


def _validate_source(args) -> None:
    """Reject source combinations that cannot connect."""
    if args.protocol != "ruida":
        return
    if args.emulator:
        if args.host or args.usb_device:
            raise ValueError(
                "--emulator cannot be combined with --host or --usb-device"
            )
        return
    if args.host and args.usb_device:
        raise ValueError("use either --host or --usb-device, not both")
    if not args.host and not args.usb_device:
        raise ValueError("Ruida needs --host, --usb-device, or --emulator")


async def open_source_session(args, event_callback):
    """A driver with *event_callback* attached, against the selected
    source.  Returns ``(session, device_task, source label,
    cleanup)``."""
    _validate_source(args)
    if args.protocol == "ruida":
        if args.emulator:
            from raygeo.driver.ruida.emulator import RuidaEmulator

            emulator = RuidaEmulator()
            await emulator.start()
            session = RuidaDriver(
                config={
                    "host": "127.0.0.1",
                    "port": emulator.port,
                    "protocol": "tcp",
                    **_RUIDA_EMULATOR_TIMINGS,
                },
                event_callback=event_callback,
            )
            return session, None, _source_label(args), emulator.stop
        session = RuidaDriver(
            config=_ruida_config(args), event_callback=event_callback
        )
        return session, None, _source_label(args), None
    config = {"poll_status_while_running": not args.no_poll}
    if args.emulator:
        from raygeo.driver.grbl.emulator import GrblEmulator

        mock = MockTransport()
        emulator = GrblEmulator(mock, speed_factor=args.speed_factor)
        session = GrblSerialDriver.with_transport(
            mock, config=config, dialect={}, event_callback=event_callback
        )
        return (
            session,
            asyncio.ensure_future(emulator.run()),
            _source_label(args),
            None,
        )
    session = GrblSerialDriver(
        config={
            "port": args.port,
            "baudrate": args.baudrate,
            **config,
        },
        dialect={},
        event_callback=event_callback,
    )
    return session, None, _source_label(args), None


async def _print_new_frames(recorder, decoder, printed: int) -> int:
    for index in range(printed, len(recorder.capture.frames)):
        frame = recorder.capture.frames[index]
        print(format_frame(index, frame, decoder), flush=True)
    return len(recorder.capture.frames)


def _finish(recorder, args) -> None:
    if args.out:
        recorder.capture.save(args.out)
        print(
            f"saved {len(recorder.capture.frames)} frames to {args.out}",
            file=sys.stderr,
        )


async def cmd_record(args) -> int:
    recorder = Recorder()
    decoder = get_decoder(args.protocol)
    try:
        session, device_task, source, cleanup = await open_source_session(
            args, recorder
        )
    except ValueError as err:
        print(f"error: {err}", file=sys.stderr)
        return 2
    recorder.capture.protocol = args.protocol
    recorder.capture.source = source
    printed = 0
    print(f"Recording {source} — Ctrl-C to stop", file=sys.stderr)
    try:
        await session.connect()
        deadline = (
            time.monotonic() + args.seconds if args.seconds > 0 else None
        )
        while deadline is None or time.monotonic() < deadline:
            printed = await _print_new_frames(recorder, decoder, printed)
            await asyncio.sleep(0.05)
    except asyncio.CancelledError:
        pass
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
        _finish(recorder, args)
    return 0


def cmd_tui(args) -> int:
    """Run the interactive monitor.

    This command is synchronous on purpose: curses needs the main
    thread, and the TUI's own ``asyncio.run`` cannot nest inside the
    CLI's event loop.
    """
    try:
        from raygeo.driver.analyzer.tui import run_tui
    except ImportError as err:
        print(f"error: TUI unavailable: {err}", file=sys.stderr)
        return 2
    try:
        _validate_source(args)
    except ValueError as err:
        print(f"error: {err}", file=sys.stderr)
        return 2
    recorder = Recorder()
    decoder = get_decoder(args.protocol)
    source = _source_label(args)
    recorder.capture.protocol = args.protocol
    recorder.capture.source = source

    async def work():
        session = None
        device_task = None
        cleanup = None
        try:
            opened = await open_source_session(args, recorder)
            session, device_task, _source, cleanup = opened
            await session.connect()
            while True:
                await asyncio.sleep(1.0)
        except Exception as err:  # noqa: BLE001 - shown in the view
            recorder.capture.append_note(f"session error: {err}")
        finally:
            if session is not None:
                try:
                    await session.disconnect()
                except Exception:  # noqa: BLE001 - teardown best effort
                    pass
            if device_task is not None:
                device_task.cancel()
            if cleanup is not None:
                try:
                    await cleanup()
                except Exception:  # noqa: BLE001 - teardown best effort
                    pass

    run_tui(
        recorder,
        decoder,
        source=source,
        save_path=args.out,
        work=work,
    )
    return 0


async def cmd_decode(args) -> int:
    capture = Capture.load(args.file)
    decoder = get_decoder(args.protocol or capture.protocol)
    sys.stdout.write(
        render_listing(capture, decoder, with_times=not args.no_times)
    )
    return 0


async def cmd_replay(args) -> int:
    capture = Capture.load(args.file)
    decoder = get_decoder(args.protocol or capture.protocol)
    host, port = args.to
    print(
        f"Replaying {len(capture.frames)} frames to "
        f"{host}:{port} (speed {args.speed:g})",
        file=sys.stderr,
    )
    reader, writer = await connect_authenticated(
        host,
        port,
        token=args.token,
        tls_ca=args.tls_ca,
        tls_hostname=args.tls_hostname,
    )
    started = time.monotonic()
    responses: list[str] = []

    async def collect_responses():
        while True:
            line = await reader.readline()
            if not line:
                return
            responses.append(line.decode(errors="replace").rstrip("\r\n"))

    collector = asyncio.ensure_future(collect_responses())

    async def send(data):
        writer.write(data)
        await writer.drain()

    try:
        written = await replay_capture(capture, send, speed=args.speed)
        print(
            f"sent {written} frames; collecting responses ...", file=sys.stderr
        )
        await asyncio.sleep(args.wait)
    finally:
        collector.cancel()
        writer.close()
    for index, line in enumerate(responses):
        frame = Frame("rx", time.monotonic() - started, text=line)
        print(format_frame(index, frame, decoder), flush=True)
    return 0


async def cmd_proxy(args) -> int:
    upstream_host, upstream_port = args.upstream
    recorder = Recorder()
    recorder.capture.protocol = args.protocol
    recorder.capture.source = f"proxy -> {args.upstream[0]}:{args.upstream[1]}"
    decoder = get_decoder(args.protocol)
    proxy = await run_proxy(
        upstream_host,
        upstream_port,
        listen_host=args.listen_host,
        listen_port=args.listen_port,
        token=args.token,
        tls_cert=args.tls_cert,
        tls_key=args.tls_key,
        capture=recorder.capture,
        framing=args.framing,
    )
    print(
        f"listening on {args.listen_host}:{proxy.port}, forwarding to "
        f"{upstream_host}:{upstream_port} — Ctrl-C to stop",
        file=sys.stderr,
    )
    printed = 0
    try:
        while True:
            printed = await _print_new_frames(recorder, decoder, printed)
            await asyncio.sleep(0.05)
    except asyncio.CancelledError:
        pass
    finally:
        await proxy.stop()
        _finish(recorder, args)
    return 0


def add_protocol_arguments(
    parser, *, choices=None, decoder_help: bool = False
) -> None:
    """Add the protocol/source options shared by ``run`` and the
    analyzer."""
    if decoder_help:
        protocol_help = (
            "decoder used for live annotations and captures; "
            "ruida also selects the Ruida driver"
        )
    else:
        protocol_help = (
            "device protocol; ruida reads rpascript and uses the Ruida driver"
        )
    parser.add_argument(
        "--protocol",
        default="grbl",
        choices=choices or decoder_names(),
        help=protocol_help,
    )
    parser.add_argument(
        "--host",
        default=None,
        help="Ruida network host (--protocol ruida)",
    )
    parser.add_argument(
        "--net-port",
        type=int,
        default=50200,
        help="Ruida network port (--protocol ruida; default 50200)",
    )
    parser.add_argument(
        "--transport",
        default="udp",
        choices=["udp", "tcp"],
        help="Ruida network transport (--protocol ruida)",
    )
    parser.add_argument(
        "--usb-device",
        default=None,
        help="Ruida USB device path (--protocol ruida)",
    )
    parser.add_argument(
        "--magic",
        type=lambda value: int(value, 0),
        default=0x88,
        help="Ruida swizzle key (--protocol ruida; default 0x88)",
    )


def add_analyzer_parser(subparsers) -> None:
    parser = subparsers.add_parser(
        "analyzer",
        help="capture, decode, replay and proxy device traffic",
    )
    analyzer_subparsers = parser.add_subparsers(
        dest="analyzer_command", required=True
    )

    def source_arguments(record_parser):
        record_parser.add_argument(
            "--port", default="/dev/ttyUSB0", help="serial port"
        )
        record_parser.add_argument(
            "--baudrate", type=int, default=115200, help="baud rate"
        )
        record_parser.add_argument(
            "--emulator",
            action="store_true",
            help="use the built-in firmware emulator instead of "
            "hardware (GRBL or Ruida, according to --protocol)",
        )
        record_parser.add_argument(
            "--speed-factor",
            type=float,
            default=2000.0,
            help="emulator motion speed multiplier (GRBL only)",
        )
        record_parser.add_argument(
            "--no-poll",
            action="store_true",
            help="disable status polling while jobs run (GRBL only)",
        )
        add_protocol_arguments(record_parser, decoder_help=True)
        record_parser.add_argument(
            "--out", default=None, help="save the capture as JSONL"
        )

    record_parser = analyzer_subparsers.add_parser(
        "record", help="record a session to a capture file"
    )
    source_arguments(record_parser)
    record_parser.add_argument(
        "--seconds",
        type=float,
        default=10.0,
        help="recording duration (0: until Ctrl-C)",
    )
    record_parser.set_defaults(func=cmd_record)

    tui_parser = analyzer_subparsers.add_parser(
        "tui", help="interactive live monitor (needs a terminal)"
    )
    source_arguments(tui_parser)
    tui_parser.set_defaults(func=cmd_tui)

    decode_parser = analyzer_subparsers.add_parser(
        "decode", help="print a capture as a decoded listing"
    )
    decode_parser.add_argument("file", help="capture file (JSONL)")
    decode_parser.add_argument(
        "--protocol",
        default=None,
        choices=decoder_names(),
        help="decoder (default: the capture's protocol)",
    )
    decode_parser.add_argument(
        "--no-times",
        action="store_true",
        help="omit the timestamp column",
    )
    decode_parser.set_defaults(func=cmd_decode)

    replay_parser = analyzer_subparsers.add_parser(
        "replay", help="replay captured frames to a device"
    )
    replay_parser.add_argument("file", help="capture file (JSONL)")
    replay_parser.add_argument(
        "--to",
        type=_parse_endpoint,
        required=True,
        help="device endpoint, HOST:PORT",
    )
    replay_parser.add_argument(
        "--speed",
        type=float,
        default=1.0,
        help="replay speed multiplier (0: as fast as possible)",
    )
    replay_parser.add_argument(
        "--wait",
        type=float,
        default=2.0,
        help="seconds to collect responses after replaying",
    )
    replay_parser.add_argument(
        "--token", default=None, help="token for the target endpoint"
    )
    replay_parser.add_argument(
        "--tls-ca", default=None, help="CA file verifying the target"
    )
    replay_parser.add_argument(
        "--tls-hostname",
        default=None,
        help="expected TLS name of the target",
    )
    replay_parser.add_argument(
        "--protocol",
        default=None,
        choices=decoder_names(),
        help="decoder (default: the capture's protocol)",
    )
    replay_parser.set_defaults(func=cmd_replay)

    proxy_parser = analyzer_subparsers.add_parser(
        "proxy",
        help="run a man-in-the-middle proxy in front of a device",
    )
    proxy_parser.add_argument(
        "--upstream",
        type=_parse_endpoint,
        required=True,
        help="device endpoint, HOST:PORT",
    )
    proxy_parser.add_argument(
        "--listen-host",
        default="127.0.0.1",
        help="listen address (non-loopback requires --token and TLS)",
    )
    proxy_parser.add_argument(
        "--listen-port", type=int, default=0, help="listen port"
    )
    proxy_parser.add_argument(
        "--token",
        default=None,
        help="require this token from connecting clients",
    )
    proxy_parser.add_argument(
        "--tls-cert", default=None, help="TLS certificate file"
    )
    proxy_parser.add_argument("--tls-key", default=None, help="TLS key file")
    proxy_parser.add_argument(
        "--framing",
        default=FRAMING_LINES,
        choices=[FRAMING_LINES, FRAMING_RAW],
        help="record lines (text protocols) or raw chunks",
    )
    proxy_parser.add_argument(
        "--protocol",
        default="raw",
        choices=decoder_names(),
        help="decoder used for live annotations and captures",
    )
    proxy_parser.add_argument(
        "--out", default=None, help="save the capture as JSONL"
    )
    proxy_parser.set_defaults(func=cmd_proxy)
