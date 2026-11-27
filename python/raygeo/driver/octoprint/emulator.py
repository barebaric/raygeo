"""An OctoPrint server emulator (not a mock) for tests.

Speaks the protocol the OctoPrint session expects: the authenticated
REST API (login, version, printer state, job control, printer
commands, file upload) and the raw SockJS WebSocket carrying push
updates, both served on one port. The printer state model is
simplified but faithful where the session observes it: state flags,
job progress and the terminal print events.

``/sockjs/websocket`` is the raw transport: it sends no ``o`` open
frame, expects the auth message as plain JSON, and pushes plain JSON
messages. Set ``framed_ws=True`` to emulate a SockJS-framed peer
(``o`` open frame, ``a[...]`` batches) instead.

Used by the test suite for hardware-free session coverage.
"""

from __future__ import annotations

import asyncio
import base64
import hashlib
import json

TICK = 0.05

DEFAULT_STATE = {
    "flags": {
        "operational": True,
        "closedOrError": False,
        "error": False,
        "paused": False,
        "printing": False,
        "cancelling": False,
        "pausing": False,
    },
    "text": "Operational",
}

DEFAULT_PRINTER_INFO = {
    "printer": {"name": "TestPrinter"},
    "dimensions": {"x_length": 200, "y_length": 300},
}

DEFAULT_VERSION = {"version": "1.9.3", "server": "1.9.3"}


def _parse_headers(raw: bytes) -> dict:
    """Parses the header block of one HTTP request."""
    headers = {}
    for line in raw.decode("utf-8", errors="replace").split("\r\n"):
        name, _, value = line.partition(":")
        if name.strip():
            headers[name.strip().lower()] = value.strip()
    return headers


def _normalize_path_prefix(path: str) -> str:
    prefix = (path or "/").strip()
    while len(prefix) > 1 and prefix.endswith("/"):
        prefix = prefix[:-1]
    return "" if prefix == "/" else prefix


def _parse_multipart(content_type: str, body: bytes) -> dict:
    """Extracts the fields of a multipart/form-data body: file parts
    map to their bytes, value parts to their text."""
    boundary = content_type.split("boundary=", 1)[1].strip()
    fields: dict = {}
    for part in body.split(f"--{boundary}".encode()):
        part = part.lstrip(b"\r\n")
        if not part or part.startswith(b"--"):
            continue
        head, _, payload = part.partition(b"\r\n\r\n")
        payload = payload.rstrip(b"\r\n")
        name = ""
        is_file = False
        for line in head.decode("utf-8", errors="replace").split("\r\n"):
            if line.lower().startswith("content-disposition:"):
                for token in line.split(";"):
                    token = token.strip()
                    if token.startswith("name="):
                        name = token[5:].strip('"')
                    elif token.startswith("filename="):
                        is_file = True
        if name:
            fields[name] = (
                payload
                if is_file
                else payload.decode("utf-8", errors="replace")
            )
    return fields


class OctoPrintEmulator:
    """A firmware-faithful OctoPrint server for tests.

    Runs one asyncio server (HTTP + the SockJS WebSocket upgrade) on
    an ephemeral port; tests interact through :attr:`http_port`,
    :attr:`commands`, :attr:`uploaded` and the state attributes.
    """

    def __init__(
        self,
        api_key="test-api-key",
        version=None,
        printer_info=None,
        require_auth=True,
        path_prefix="/",
        framed_ws=False,
    ):
        self.api_key = api_key
        self.path_prefix = _normalize_path_prefix(path_prefix)
        self.framed_ws = framed_ws
        self.version = dict(DEFAULT_VERSION)
        if version:
            self.version.update(version)
        self.printer_info = json.loads(json.dumps(DEFAULT_PRINTER_INFO))
        if printer_info:
            self.printer_info.update(printer_info)
        self.require_auth = require_auth
        self.state = json.loads(json.dumps(DEFAULT_STATE))
        self.commands: list[dict] = []
        self.uploaded: list[bytes] = []
        self.ws_messages: list[str] = []
        self.job_active = False
        self.job_completion: float | None = None
        self.fail_next_upload = False
        self.http_port: int | None = None
        self._server: asyncio.AbstractServer | None = None
        self._ws_clients: list = []
        self._job_task: asyncio.Task | None = None

    async def start(self):
        self._server = await asyncio.start_server(
            self._handle_connection, "127.0.0.1", 0
        )
        self.http_port = self._server.sockets[0].getsockname()[1]

    async def stop(self):
        if self._server is not None:
            self._server.close()
            await self._server.wait_closed()
        if self._job_task is not None:
            self._job_task.cancel()
        for writer in list(self._ws_clients):
            writer.close()

    # -- test-facing state changes -----------------------------------

    def set_state_text(self, text: str, **flags):
        self.state["text"] = text
        self.state["flags"].update(flags)
        self._push_current()

    def start_print(self, duration_s: float = 0.2):
        """Run a print job that completes on its own."""
        self.job_active = True
        self.state["flags"]["printing"] = True
        self.state["text"] = "Printing"
        self._job_task = asyncio.ensure_future(self._run_job(duration_s))
        self._push_current()

    def finish_print(self, success: bool = True):
        self.job_active = False
        self.state["flags"]["printing"] = False
        self.state["flags"]["operational"] = True
        self.state["text"] = "Operational"
        self.job_completion = None
        event_type = "PrintDone" if success else "PrintFailed"
        self._push_event(event_type)
        self._push_current()

    def _push_current(self):
        self._push(
            json.dumps(
                {
                    "current": {
                        "state": self.state,
                        "progress": {"completion": self.job_completion},
                    }
                }
            )
        )

    def _push_event(self, event_type: str):
        self._push(json.dumps({"event": {"type": event_type}}))

    def _push(self, payload: str):
        frame = payload
        if self.framed_ws:
            # SockJS data frames wrap the JSON-string messages in an
            # ``a[...]`` array frame.
            frame = "a" + json.dumps([payload])
        self.send_frame(frame)

    def send_frame(self, text: str):
        """Sends one raw text frame to every connected client."""
        for writer in list(self._ws_clients):
            try:
                self._ws_send(writer, text)
            except (ConnectionError, OSError):
                if writer in self._ws_clients:
                    self._ws_clients.remove(writer)

    # -- connection handling ------------------------------------------

    async def _handle_connection(self, reader, writer):
        try:
            request_line = await reader.readline()
            header_lines = await reader.readuntil(b"\r\n\r\n")
        except (ConnectionError, asyncio.IncompleteReadError):
            writer.close()
            return
        headers = _parse_headers(header_lines)
        headers["__request_line__"] = request_line.decode(
            "utf-8", errors="replace"
        )
        if headers.get("upgrade", "").lower() == "websocket":
            target = headers["__request_line__"].split(" ")[1]
            path = target.split("?", 1)[0]
            if self.path_prefix and not (
                path == self.path_prefix
                or path.startswith(self.path_prefix + "/")
            ):
                writer.write(
                    b"HTTP/1.1 404 Not Found\r\n"
                    b"Content-Length: 0\r\n"
                    b"Connection: close\r\n\r\n"
                )
                await writer.drain()
                writer.close()
                return
            await self._handle_ws(headers, reader, writer)
        else:
            await self._handle_http(headers, reader, writer)

    async def _handle_http(self, headers, reader, writer):
        # One request per connection: the session's HTTP client
        # always sends `Connection: close` and reads the response to
        # EOF.
        try:
            method, target, body = await self._read_request(headers, reader)
            target = self._strip_prefix(target)
            if target is None:
                status, payload = 404, {"error": "wrong path prefix"}
            else:
                status, payload = self._route(method, target, headers, body)
            data = json.dumps(payload).encode() if payload is not None else b""
            writer.write(
                f"HTTP/1.1 {status} OK\r\n"
                f"Content-Type: application/json\r\n"
                f"Content-Length: {len(data)}\r\n"
                f"Connection: close\r\n\r\n".encode()
                + data
            )
            await writer.drain()
        except (ConnectionError, OSError, asyncio.IncompleteReadError):
            pass
        finally:
            writer.close()

    def _strip_prefix(self, target: str):
        if not self.path_prefix:
            return target
        if target == self.path_prefix:
            return "/"
        if target.startswith(self.path_prefix + "/"):
            return target[len(self.path_prefix) :]
        return None

    async def _read_request(self, headers, reader):
        method, target, _version = (
            headers["__request_line__"].strip().split(" ")
        )
        body = b""
        length = int(headers.get("content-length", "0"))
        if length:
            body = await reader.readexactly(length)
        return method, target, body

    def _authorized(self, headers) -> bool:
        if not self.require_auth:
            return True
        return headers.get("x-api-key") == self.api_key

    def _route(self, method, target, headers, body):
        if self.require_auth and not self._authorized(headers):
            return 403, {"error": "authentication failed"}

        path = target.split("?", 1)[0]
        if path == "/api/version":
            return 200, self.version

        if path == "/api/printer":
            payload = {"state": self.state}
            payload.update(self.printer_info)
            return 200, payload

        if path == "/api/login":
            return 200, {
                "session": "emulator-session-key",
                "name": "rayforge",
            }

        if path == "/api/job":
            if method == "GET":
                return 200, {
                    "state": (
                        "Printing" if self.job_active else "Operational"
                    ),
                    "progress": {"completion": self.job_completion},
                }
            payload = json.loads(body.decode() or "{}")
            command = payload.get("command")
            self.commands.append(payload)
            if command == "cancel":
                if not self.job_active:
                    return 409, {"error": "no job to cancel"}
                self.finish_print(success=True)
                return 204, None
            if command == "pause":
                return 204, None
            return 400, {"error": f"unknown job command {command}"}

        if path == "/api/printer/command":
            payload = json.loads(body.decode() or "{}")
            if "command" in payload:
                self.commands.append({"command": payload["command"]})
            else:
                for entry in payload.get("commands", []):
                    self.commands.append({"command": entry})
            return 204, None

        if path == "/api/printer/printhead":
            payload = json.loads(body.decode() or "{}")
            self.commands.append(payload)
            return 204, None

        if path == "/api/files/local":
            if self.fail_next_upload:
                self.fail_next_upload = False
                return 409, {"error": "printer is busy"}
            fields = _parse_multipart(headers.get("content-type", ""), body)
            self.uploaded.append(fields.get("file", b""))
            select = fields.get("select") == "true"
            print_requested = fields.get("print") == "true"
            effective = False
            if print_requested and select:
                effective = True
                self.start_print()
            return 201, {"effectivePrint": effective}

        return 404, {"error": f"no route for {path}"}

    # -- SockJS WebSocket ---------------------------------------------

    async def _handle_ws(self, headers, reader, writer):
        try:
            key = headers.get("sec-websocket-key", "")
            accept = base64.b64encode(
                hashlib.sha1(
                    (key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()
                ).digest()
            ).decode()
            writer.write(
                "HTTP/1.1 101 Switching Protocols\r\n"
                "Upgrade: websocket\r\nConnection: Upgrade\r\n"
                f"Sec-WebSocket-Accept: {accept}\r\n\r\n".encode()
            )
            await writer.drain()
            self._ws_clients.append(writer)

            if self.framed_ws:
                self._ws_send(writer, "o")
            while True:
                opcode, payload = await self._ws_read_frame(reader)
                if opcode is None or opcode == 0x8:
                    break
                if opcode == 0x9:
                    self._ws_send_pong(writer, payload)
                elif opcode == 0x1:
                    text = payload.decode("utf-8", errors="replace")
                    self.ws_messages.append(text)
                    if self._is_auth_message(text):
                        # Real OctoPrint sends the current state
                        # right after authenticating the socket.
                        self._push_current()
                # Auth frames are recorded but not enforced; the
                # emulator serves updates to every connected client.
        except (ConnectionError, OSError, asyncio.IncompleteReadError):
            pass
        finally:
            if writer in self._ws_clients:
                self._ws_clients.remove(writer)
            writer.close()

    @staticmethod
    def _is_auth_message(text: str) -> bool:
        def has_auth(value) -> bool:
            return isinstance(value, dict) and "auth" in value

        try:
            parsed = json.loads(text)
        except (json.JSONDecodeError, TypeError):
            return False
        if isinstance(parsed, list):
            for item in parsed:
                if isinstance(item, str):
                    try:
                        item = json.loads(item)
                    except json.JSONDecodeError:
                        continue
                if has_auth(item):
                    return True
            return False
        return has_auth(parsed)

    def _ws_send(self, writer, text: str):
        # Server-to-client frames are never masked (RFC 6455).
        payload = text.encode()
        header = bytearray([0x81])
        length = len(payload)
        if length > 125:
            header.append(126)
            header.extend(length.to_bytes(2, "big"))
        else:
            header.append(length)
        writer.write(bytes(header) + payload)

    def _ws_send_pong(self, writer, payload: bytes):
        header = bytearray([0x8A])
        header.append(len(payload))
        writer.write(bytes(header) + payload)

    async def _ws_read_frame(self, reader):
        header = await reader.readexactly(2)
        opcode = header[0] & 0x0F
        length = header[1] & 0x7F
        if length == 126:
            length = int.from_bytes(await reader.readexactly(2), "big")
        mask_bit = header[1] & 0x80
        mask = await reader.readexactly(4) if mask_bit else None
        payload = await reader.readexactly(length)
        if mask:
            payload = bytes(
                byte ^ mask[i % 4] for i, byte in enumerate(payload)
            )
        return opcode, payload

    async def _run_job(self, duration_s: float):
        try:
            steps = max(1, int(duration_s / TICK))
            for step in range(1, steps + 1):
                await asyncio.sleep(TICK)
                self.job_completion = round(100.0 * step / steps, 1)
                self._push_current()
            self.finish_print(success=True)
        except asyncio.CancelledError:
            pass
