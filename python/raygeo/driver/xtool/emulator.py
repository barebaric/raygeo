"""An xTool WiFi-module firmware emulator.

Like the GRBL emulator this is *not* a mock: it models the observable
behavior of xTool's proprietary WiFi API — the JSON acknowledgement
envelope of every HTTP endpoint, job execution started via /read with
its WebSocket job-state events (``ok:WORKING_*`` … ``ok:IDLE``),
homing via M28, job pause/resume/stop via /cnc/data, and the safety
alarms (``err:*``) — so session code is exercised against the same
protocol dynamics it sees on hardware.

Two asyncio servers are used: one for the HTTP API (default port
8080) and one for the WebSocket notification channel (default port
8081).  Motion is not simulated; a job simply runs for
``job_duration`` seconds (scaled down for tests), while homing runs
for ``homing_duration`` seconds.
"""

import asyncio
import base64
import hashlib
import json
import re
from urllib.parse import parse_qs, unquote, urlsplit

WS_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
MACHINE_TYPE = "xTool D1Pro"
LASER_POWER = 20

# Homing duration on hardware, scaled by the emulator.
HOMING_DURATION_S = 10.0
# Duration a started job runs, scaled by the emulator.
JOB_DURATION_S = 5.0

DEFAULT_SPEED_FACTOR = 20.0


class XtoolEmulator:
    """A firmware emulator driven by the running event loop.

    Behavior is controlled through the public attributes: tests can
    set ``down`` to simulate an unreachable device and call
    ``trigger_error`` / ``fail_next_start`` to exercise error paths.
    """

    def __init__(
        self,
        machine_type=MACHINE_TYPE,
        homing_duration=None,
        job_duration=None,
        speed_factor=DEFAULT_SPEED_FACTOR,
    ):
        self.machine_type = machine_type
        self.homing_duration = (
            homing_duration
            if homing_duration is not None
            else HOMING_DURATION_S / speed_factor
        )
        self.job_duration = (
            job_duration
            if job_duration is not None
            else JOB_DURATION_S / speed_factor
        )
        self.down = False
        self.fail_next_start = False
        self.reject_commands = False
        self.http_port: int | None = None
        self.ws_port: int | None = None
        self._job_task: asyncio.Task | None = None
        self.commands = []
        self.blocks = []
        self.uploaded = None
        self.started_files = []
        self.job_actions = []
        self._servers = []
        self._clients = set()
        self._job_task = None
        self._job_paused = False
        self._job_stopped = False

    # -------------------------------------------------------------
    # Lifecycle
    # -------------------------------------------------------------

    async def start(self, http_port=0, ws_port=0):
        """Start both servers; call once before driving sessions.

        Explicit ports (e.g. to re-attach a previously dead device)
        can be passed; the default binds ephemeral ports."""
        http_server = await asyncio.start_server(
            self._handle_http, "127.0.0.1", http_port
        )
        ws_server = await asyncio.start_server(
            self._handle_ws, "127.0.0.1", ws_port
        )
        self._servers = [http_server, ws_server]
        self.http_port = http_server.sockets[0].getsockname()[1]
        self.ws_port = ws_server.sockets[0].getsockname()[1]
        return self.http_port, self.ws_port

    async def stop(self):
        for server in self._servers:
            server.close()
            await server.wait_closed()
        self._servers.clear()
        if self._job_task is not None:
            self._job_task.cancel()
        for writer in list(self._clients):
            writer.close()

    # -------------------------------------------------------------
    # Test controls
    # -------------------------------------------------------------

    def trigger_error(self, kind):
        """Push a safety-alarm event to every connected client."""
        self._broadcast(f"err:{kind}")

    def broadcast(self, line):
        """Push an arbitrary event line to every connected client."""
        self._broadcast(line)

    # -------------------------------------------------------------
    # HTTP server
    # -------------------------------------------------------------

    async def _handle_http(self, reader, writer):
        try:
            request = await self._read_request(reader, writer)
            if request is None:
                return
            method, target, headers, body = request
            response = self._route(method, target, headers, body)
            status = 200 if response is not None else 404
            payload = (
                response
                if isinstance(response, bytes)
                else (response or "").encode()
            )
            writer.write(
                (
                    f"HTTP/1.1 {status} "
                    f"{'OK' if status == 200 else 'NOT FOUND'}\r\n"
                    "Content-Type: application/json\r\n"
                    f"Content-Length: {len(payload)}\r\n"
                    "Connection: close\r\n"
                    "\r\n"
                ).encode()
            )
            writer.write(payload)
            await writer.drain()
        except (ConnectionError, asyncio.IncompleteReadError):
            pass
        finally:
            writer.close()

    async def _read_request(self, reader, writer):
        try:
            head = await reader.readuntil(b"\r\n\r\n")
        except (asyncio.IncompleteReadError, ConnectionError):
            return None
        lines = head.decode("utf-8", "replace").split("\r\n")
        method, target, _ = lines[0].split(" ", 2)
        headers = {}
        for line in lines[1:]:
            name, _, value = line.partition(":")
            headers[name.strip().lower()] = value.strip()
        length = int(headers.get("content-length", "0"))
        body = await reader.readexactly(length) if length else b""
        return method, target, headers, body

    def _route(self, method, target, headers, body):
        split = urlsplit(target)
        path = split.path
        query = parse_qs(split.query)
        if self.down:
            return None

        if path == "/ping":
            return self._result("ok")

        if path == "/getmachinetype":
            return json.dumps({"result": "ok", "type": self.machine_type})

        if path == "/getlaserpowertype":
            return json.dumps({"result": "ok", "power": LASER_POWER})

        if path == "/system":
            action = query.get("action", [""])[0]
            if action == "version":
                return json.dumps(
                    {
                        "result": "ok",
                        "sn": "EMU123456",
                        "version": "V40.31.006.01 B2",
                    }
                )
            if action == "get_working_sta":
                working = (
                    "1"
                    if self._job_task is not None and not self._job_task.done()
                    else "0"
                )
                return json.dumps({"result": "ok", "working": working})
            return self._result("ok")

        if path == "/cmd":
            if self.reject_commands:
                return self._result("fail")
            if method == "POST":
                block = body.decode("utf-8", "replace")
                self.blocks.append(block)
                self._execute_block(block)
            else:
                command = unquote(query.get("cmd", [""])[0])
                self.commands.append(command)
                self._execute_block(command)
            return self._result("ok")

        if path == "/upload":
            self.uploaded = self._parse_multipart(headers, body)
            return self._result("ok")

        if path == "/read":
            filename = unquote(query.get("file", [""])[0])
            self.started_files.append(filename)
            if self.fail_next_start:
                self.fail_next_start = False
                return self._result("fail")
            if self.uploaded is None:
                return self._result("fail")
            self._start_job()
            return self._result("ok")

        if path == "/cnc/data":
            action = query.get("action", [""])[0]
            self.job_actions.append(action)
            if action == "pause":
                if self._job_task is not None:
                    self._job_paused = True
                    self._broadcast("ok:PAUSING")
                    return self._result("ok")
                return self._result("fail")
            if action == "resume":
                if self._job_task is not None:
                    self._job_paused = False
                    self._broadcast("ok:WORKING_ONLINE")
                    return self._result("ok")
                return self._result("fail")
            if action == "stop":
                self._stop_job()
                return self._result("ok")
            return self._result("fail")

        if path == "/progress":
            return json.dumps({"progress": 0.0, "working": 0, "line": 0})

        return None

    # -------------------------------------------------------------
    # Machine state
    # -------------------------------------------------------------

    def _result(self, value):
        return json.dumps({"result": value})

    def _execute_block(self, block):
        for line in block.splitlines():
            command = line.strip()
            if not command:
                continue
            if command == "M28":
                self._start_homing()
            elif command.startswith("M205"):
                pass

    def _start_homing(self):
        async def homing():
            await asyncio.sleep(self.homing_duration)
            self._broadcast("ok:IDLE")

        asyncio.ensure_future(homing())

    def _start_job(self):
        if self._job_task is not None and not self._job_task.done():
            return

        async def _job():
            self._broadcast("ok:WORKING_ONLINE")
            remaining = self.job_duration
            while remaining > 0:
                await asyncio.sleep(0.01)
                if self._job_stopped:
                    self._job_stopped = False
                    self._broadcast("WORK_STOPPED")
                    self._broadcast("ok:IDLE")
                    return
                if not self._job_paused:
                    remaining -= 0.01
            self._job_paused = False
            self._broadcast("ok:IDLE")

        self._job_task = asyncio.ensure_future(_job())

    def _stop_job(self):
        if self._job_task is None or self._job_task.done():
            return
        self._job_stopped = True

    # -------------------------------------------------------------
    # WebSocket server
    # -------------------------------------------------------------

    async def _handle_ws(self, reader, writer):
        try:
            head = await reader.readuntil(b"\r\n\r\n")
        except (asyncio.IncompleteReadError, ConnectionError):
            writer.close()
            return
        headers = {}
        for line in head.decode("utf-8", "replace").split("\r\n")[1:]:
            name, _, value = line.partition(":")
            headers[name.strip().lower()] = value.strip()
        key = headers.get("sec-websocket-key")
        if not key or headers.get("upgrade", "").lower() != "websocket":
            writer.close()
            return
        accept = base64.b64encode(
            hashlib.sha1((key + WS_GUID).encode()).digest()
        ).decode()
        writer.write(
            (
                "HTTP/1.1 101 Switching Protocols\r\n"
                "Upgrade: websocket\r\n"
                "Connection: Upgrade\r\n"
                f"Sec-WebSocket-Accept: {accept}\r\n"
                "\r\n"
            ).encode()
        )
        await writer.drain()
        self._clients.add(writer)
        try:
            await self._read_ws_frames(reader, writer)
        except (asyncio.IncompleteReadError, ConnectionError, OSError):
            pass
        finally:
            self._clients.discard(writer)
            writer.close()

    async def _read_ws_frames(self, reader, writer):
        while True:
            header = await reader.readexactly(2)
            length = header[1] & 0x7F
            if length == 126:
                length = int.from_bytes(await reader.readexactly(2), "big")
            elif length == 127:
                length = int.from_bytes(await reader.readexactly(8), "big")
            mask = await reader.readexactly(4) if header[1] & 0x80 else None
            payload = await reader.readexactly(length) if length else b""
            if mask is not None:
                payload = bytes(b ^ mask[i % 4] for i, b in enumerate(payload))
            opcode = header[0] & 0x0F
            if opcode == 0x9:  # ping → pong
                await self._send_frame(writer, 0xA, payload)

    async def _send_frame(self, writer, opcode, payload):
        header = bytearray([0x80 | opcode])
        length = len(payload)
        if length < 126:
            header.append(length)
        elif length < 65536:
            header.append(126)
            header.extend(length.to_bytes(2, "big"))
        else:
            header.append(127)
            header.extend(length.to_bytes(8, "big"))
        writer.write(bytes(header) + payload)
        await writer.drain()

    def _broadcast(self, line):
        for writer in list(self._clients):
            try:
                asyncio.ensure_future(
                    self._send_frame(writer, 0x1, line.encode())
                )
            except ConnectionError:
                self._clients.discard(writer)

    # -------------------------------------------------------------
    # Helpers
    # -------------------------------------------------------------

    @staticmethod
    def _parse_multipart(headers, body):
        match = re.search(
            r'boundary=([^;\s"]+)', headers.get("content-type", "")
        )
        if not match:
            return None
        boundary = match.group(1).encode()
        segments = body.split(b"--" + boundary)
        for segment in segments:
            if b"filename=" not in segment:
                continue
            head, _, payload = segment.partition(b"\r\n\r\n")
            return payload.removesuffix(b"\r\n")
        return None
