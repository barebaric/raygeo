"""An ESP3D / FluidNC web-api firmware emulator.

This is not a mock: it serves the same HTTP command endpoint and
WebSocket response channel as the firmware, including the ESP3D
handshake commands, ``$$``/``$#``/``$G`` replies, status reports and
the ``[ESP220]`` upload-and-execute job flow.  Jobs complete after
``job_duration`` seconds so tests do not need real motion.
"""

import asyncio
import base64
import hashlib
import re
from urllib.parse import parse_qs, unquote, urlsplit

WS_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"

STOCK_SETTINGS = [
    "$0=10",
    "$1=25",
    "$2=0",
    "$3=0",
    "$4=0",
    "$5=0",
    "$6=0",
    "$10=3",
    "$11=0.020",
    "$12=0.002",
    "$13=0",
    "$20=0",
    "$21=0",
    "$22=0",
    "$23=0",
    "$24=250.000",
    "$25=500.000",
    "$26=250",
    "$27=1.000",
    "$30=1000.000",
    "$31=0.000",
    "$32=0",
    "$100=250.000",
    "$101=250.000",
    "$102=250.000",
    "$110=500.000",
    "$111=500.000",
    "$112=500.000",
    "$120=10.000",
    "$121=10.000",
    "$122=10.000",
    "$130=400.000",
    "$131=300.000",
    "$132=100.000",
]

STATUS_IDLE = "<Idle|MPos:0.000,0.000,0.000|FS:0,0>"
STATUS_RUN = "<Run|MPos:0.000,0.000,0.000|FS:100,0>"


class Esp3dEmulator:
    """HTTP + WebSocket emulator for an ESP3D/FluidNC device."""

    def __init__(self, job_duration=0.05):
        self.job_duration = job_duration
        self.down = False
        self.http_port = None
        self.ws_port = None
        self.commands = []
        self.uploaded = None
        self.started_files = []
        self.status_line = STATUS_IDLE
        self._clients = set()
        self._servers = []
        self._job_task = None

    async def start(self, http_port=0, ws_port=0):
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
        if self._job_task is not None:
            self._job_task.cancel()
            self._job_task = None
        # Close active WebSocket connections before waiting for the
        # servers: on Windows, wait_closed() waits for active client
        # connections to finish.
        for writer in list(self._clients):
            writer.close()
        self._clients.clear()
        for server in self._servers:
            server.close()
        try:
            await asyncio.wait_for(
                asyncio.gather(
                    *(server.wait_closed() for server in self._servers)
                ),
                timeout=2.0,
            )
        except asyncio.TimeoutError:
            pass
        self._servers.clear()

    def broadcast(self, line):
        self._broadcast(line)

    def set_status(self, line):
        """Set the status reported for ``?`` polls and broadcast it."""
        self.status_line = line
        self.broadcast(line)

    # -------------------------------------------------------------
    # HTTP
    # -------------------------------------------------------------

    async def _handle_http(self, reader, writer):
        try:
            request = await self._read_request(reader)
            if request is None:
                return
            method, target, headers, body = request
            response = self._route(method, target, headers, body)
            status = 200 if response is not None else 404
            payload = (response or "").encode()
            writer.write(
                (
                    f"HTTP/1.1 {status} "
                    f"{'OK' if status == 200 else 'NOT FOUND'}\r\n"
                    "Content-Type: text/plain\r\n"
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

    async def _read_request(self, reader):
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
        if self.down:
            return None
        split = urlsplit(target)
        path = split.path
        query = parse_qs(split.query)

        if path == "/api/status":
            return "Idle"

        if path in ("/command", "/"):
            plain = query.get("plain", [""])[0]
            if plain:
                return f"[{plain}]"
            command = unquote(query.get("commandText", [""])[0])
            self._handle_command(command)
            return ""

        if path == "/upload":
            self.uploaded = self._parse_multipart(headers, body)
            return "ok"

        return None

    def _handle_command(self, command):
        command = command.strip()
        if not command:
            return
        self.commands.append(command)
        if command == "?":
            self.broadcast(self.status_line)
            return
        if command.startswith("[ESP220]"):
            filename = command.removeprefix("[ESP220]").lstrip("/")
            self.started_files.append(filename)
            self._start_job()
            return
        if command == "$I":
            self.broadcast("[VER:1.1h.20190825:]")
            self.broadcast("[OPT:V,15,128]")
        elif command == "$$":
            for line in STOCK_SETTINGS:
                self.broadcast(line)
        elif command == "$#":
            for slot in ("G54", "G55", "G56", "G57", "G58", "G59"):
                self.broadcast(f"[{slot}:0.000,0.000,0.000]")
        elif command == "$G":
            self.broadcast("[G54 G17 G21 G90 G94 M5 M9 T0 F0 S0]")
        elif command.startswith("G38.2"):
            self.broadcast("[PRB:0.000,0.000,0.000:1]")
        self.broadcast("ok")

    def _start_job(self):
        if self._job_task is not None:
            self._job_task.cancel()
        self.set_status(STATUS_RUN)

        async def _job():
            await asyncio.sleep(self.job_duration)
            self.set_status(STATUS_IDLE)

        self._job_task = asyncio.ensure_future(_job())

    # -------------------------------------------------------------
    # WebSocket
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
            if opcode == 0x9:
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

    async def _send_broadcast(self, writer, line):
        try:
            await self._send_frame(writer, 0x1, line.encode())
        except (ConnectionError, OSError):
            self._clients.discard(writer)

    def _broadcast(self, line):
        for writer in list(self._clients):
            asyncio.ensure_future(self._send_broadcast(writer, line))

    @staticmethod
    def _parse_multipart(headers, body):
        match = re.search(
            r'boundary=([^;\s"]+)', headers.get("content-type", "")
        )
        if not match:
            return None
        boundary = match.group(1).encode()
        for segment in body.split(b"--" + boundary):
            if b"filename=" not in segment:
                continue
            _, _, payload = segment.partition(b"\r\n\r\n")
            return payload.removesuffix(b"\r\n")
        return None
