import asyncio
import base64
import hashlib
import json
from urllib.parse import parse_qs, urlsplit

GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"


def crc16(data):
    value = 0
    for byte in data:
        value ^= byte
        for _ in range(8):
            value = (value >> 1) ^ (0xA001 if value & 1 else 0)
    return value


def encode_protocol_frame(payload):
    header = bytearray(10)
    header[0:2] = b"\xba\xbe"
    header[2:5] = len(payload).to_bytes(3, "big")
    header[5] = 4
    header[6:8] = crc16(payload).to_bytes(2, "big")
    header[8:10] = crc16(header[:8]).to_bytes(2, "big")
    return bytes(header) + payload


def decode_protocol_frame(frame):
    if len(frame) < 10 or frame[:2] != b"\xba\xbe":
        raise ValueError("invalid protocol frame")
    length = int.from_bytes(frame[2:5], "big")
    if len(frame) != 10 + length:
        raise ValueError("invalid protocol frame length")
    if crc16(frame[:8]) != int.from_bytes(frame[8:10], "big"):
        raise ValueError("invalid header crc")
    payload = frame[10:]
    if crc16(payload) != int.from_bytes(frame[6:8], "big"):
        raise ValueError("invalid payload crc")
    return json.loads(payload)


class M2Emulator:
    """Emulate the M2 WS-V2 instruction channel for offline testing."""

    def __init__(self, ssl_context):
        self.mode = "Idle"
        self.requests = []
        self.server = None
        self.port = None
        self.ssl_context = ssl_context
        self._writers = set()

    async def start(self, host="127.0.0.1", port=0):
        self.server = await asyncio.start_server(
            self._handle,
            host,
            port,
            ssl=self.ssl_context,
        )
        self.port = self.server.sockets[0].getsockname()[1]

    async def close(self):
        if self.server is not None:
            self.server.close()
            await self.server.wait_closed()
            self.server = None
        for writer in list(self._writers):
            writer.close()
            await writer.wait_closed()
        self._writers.clear()

    async def _handle(self, reader, writer):
        self._writers.add(writer)
        try:
            header = await reader.readuntil(b"\r\n\r\n")
            lines = header.decode("ascii").split("\r\n")
            request_target = lines[0].split(" ")[1]
            headers = {}
            for line in lines[1:]:
                name, _, value = line.partition(":")
                headers[name.lower()] = value.strip()
            accept = base64.b64encode(
                hashlib.sha1(
                    (headers["sec-websocket-key"] + GUID).encode("ascii")
                ).digest()
            ).decode("ascii")
            writer.write(
                (
                    "HTTP/1.1 101 Switching Protocols\r\n"
                    "Upgrade: websocket\r\n"
                    "Connection: Upgrade\r\n"
                    f"Sec-WebSocket-Accept: {accept}\r\n\r\n"
                ).encode("ascii")
            )
            await writer.drain()
            query = parse_qs(urlsplit(request_target).query)
            if query.get("function") != ["instruction"]:
                return
            await self._read_requests(reader, writer)
        except (ConnectionError, asyncio.IncompleteReadError):
            pass
        finally:
            writer.close()
            self._writers.discard(writer)

    async def _read_requests(self, reader, writer):
        while True:
            frame = await self._read_websocket_frame(reader)
            if frame is None:
                return
            opcode, payload = frame
            if opcode == 8:
                return
            if opcode != 2:
                continue
            request = decode_protocol_frame(payload)
            self.requests.append(request)
            await self._dispatch(request, writer)

    async def _dispatch(self, request, writer):
        url = request["url"]
        data = {}
        if url == "/v1/platform/device/machine-info":
            data = {"deviceCode": "JS002", "deviceName": "xTool M2"}
        elif url == "/v1/platform/device/state/sync":
            data = {"curMode": {"mode": self.mode}}
        elif url == "/v1/project/device/coordinate":
            data = {"x": 1.0, "y": 2.0, "z": 0.0}
        elif url == "/v1/project/device/control":
            action = request["params"].get("action")
            self.mode = {
                "START": "WorkPlay",
                "PAUSE": "WorkPause",
                "RESUME": "WorkPlay",
                "CANCEL": "Idle",
            }[action]
        response = {
            "type": "response",
            "transactionId": request["transactionId"],
            "code": 0,
            "data": data,
            "msg": "ok",
        }
        await self._send_message(writer, response)
        if url == "/v1/project/device/control":
            await self._send_message(
                writer,
                {
                    "type": "request",
                    "method": "REPORT",
                    "data": {"/v1/platform/device/state": {"mode": self.mode}},
                },
            )

    async def _send_message(self, writer, payload):
        data = encode_protocol_frame(
            json.dumps(payload, separators=(",", ":")).encode()
        )
        header = bytearray([0x82])
        if len(data) < 126:
            header.append(len(data))
        elif len(data) < 65536:
            header.append(126)
            header.extend(len(data).to_bytes(2, "big"))
        else:
            header.append(127)
            header.extend(len(data).to_bytes(8, "big"))
        writer.write(header + data)
        await writer.drain()

    async def _read_websocket_frame(self, reader):
        header = await reader.readexactly(2)
        opcode = header[0] & 0x0F
        masked = header[1] & 0x80
        length = header[1] & 0x7F
        if length == 126:
            length = int.from_bytes(await reader.readexactly(2), "big")
        elif length == 127:
            length = int.from_bytes(await reader.readexactly(8), "big")
        mask = await reader.readexactly(4) if masked else None
        payload = await reader.readexactly(length)
        if mask is not None:
            payload = bytes(
                value ^ mask[i % 4] for i, value in enumerate(payload)
            )
        return opcode, payload
