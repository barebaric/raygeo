# raydriver daemon

The daemon hosts raydriver sessions next to the hardware (e.g. on
a Raspberry Pi in the workshop) while Rayforge — or any raydriver
consumer — runs elsewhere on the LAN. Jobs run daemon-side and
survive client disconnects; reattaching clients resync progress
exactly. The daemon hosts *sessions*, not machines: device
connection configs live in `daemon.json`, and clients send the
command dialect with each session open. A `daemon.json` entry may
pin a dialect for an auto-opened session; the client's dialect wins
for interactive opens.

Everything here works on Linux, macOS and Windows. The daemon,
client proxy, wire codec and mDNS helpers are implemented in Rust;
the Python modules only adapt them to asyncio and PyO3.

## Install and first run

```sh
pip install raydriver              # includes network discovery
raydriver daemon add /dev/ttyUSB0  # config + generated token + device
raydriver daemon install           # install + start the platform service
```

`add` writes the config on first use (mode 0600) with a generated
**token** — the password clients use to connect — and prints it;
`daemon status` shows it again. `daemon --setup` is the
config-only variant (no device) and also prints the certificate
one-liner for exposed listeners. The config lives in the
platform's per-user location:

| Platform | Config path                                        |
| -------- | -------------------------------------------------- |
| Linux    | `~/.config/raydriver/daemon.json` (or `$XDG_CONFIG_HOME`) |
| macOS    | `~/Library/Application Support/raydriver/daemon.json` |
| Windows  | `%APPDATA%\raydriver\daemon.json`                  |

`--config PATH` overrides it everywhere; `python -m raydriver
daemon` works too. To run in the foreground instead of as a
service: `raydriver daemon`.

## Service installation

`raydriver daemon install` uses the platform's native mechanism:

- **Linux**: a systemd *user* unit
  (`~/.config/systemd/user/raydriver-daemon.service`), enabled and
  started immediately. For boot start without login:
  `loginctl enable-linger $USER`. `--system` installs a system unit
  instead (run with sudo).
- **macOS**: a launchd LaunchAgent
  (`~/Library/LaunchAgents/org.raydriver.daemon.plist`), loaded at
  login and kept alive; logs go to `~/Library/Logs/raydriver-daemon.log`.
- **Windows**: a scheduled task (`raydriver-daemon`) running at
  logon, using `pythonw.exe` when available so no console window
  stays open.

`raydriver daemon uninstall` stops and removes it;
`raydriver daemon status` shows the service state, the token, and
a live probe of the daemon (version, auth mode). Without a
service it reports `service: not installed` and still probes.

## Managing devices

```sh
raydriver daemon add /dev/ttyUSB0    # grbl at 115200 (the defaults)
raydriver daemon add /dev/ttyACM0 --protocol marlin --baudrate 250000
raydriver daemon remove /dev/ttyUSB0
raydriver list                       # configured devices + live state
```

`add` upserts by serial port and creates the config with a
generated token when none exists; `--protocol` is one of `grbl`,
`grbl_simple`, `smoothie`, `marlin`, `octoprint`. ESP3D and FluidNC
sessions are network-attached, so they are configured in
`daemon.json` with a `host`, `http_port`, `ws_port` and optional
`protocol` variant. xTool network sessions are local-only and are
not hosted by the daemon.
Configured sessions open at daemon start, so restart the daemon to
apply changes. The config can also be edited by hand — see the
reference below.

Manual systemd example (what `install` writes):

```ini
[Unit]
Description=raydriver device daemon
After=network-online.target

[Service]
ExecStart=/usr/bin/python3 -m raydriver daemon --config /etc/raydriver/daemon.json
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
```

## Config reference (`daemon.json`)

| Key                  | Default            | Meaning                              |
| -------------------- | ------------------ | ------------------------------------ |
| `host`               | `127.0.0.1`        | bind address                         |
| `port`               | `8765`             | bind port (`0` = ephemeral)          |
| `token`              | none               | shared secret, constant-time checked |
| `tls_cert`/`tls_key` | none               | PEM certificate and key              |
| `keepalive_interval` | `2.0`              | seconds between pings                |
| `keepalive_misses`   | `3`                | missed pings before disconnect       |
| `lease_grace`        | `5.0`              | seconds a disconnected holder keeps the control lease |
| `ring_size`          | `1000`             | replayable events per session        |
| `cameras`            | `[]`               | advertised stream endpoints (below)  |
| `sessions`           | `[]`               | sessions to auto-open at boot        |
| `mdns`               | `true`             | announce `_raydriver._tcp` for discovery |

Auto-open example:

```json
{
  "host": "0.0.0.0",
  "token": "…",
  "tls_cert": "/etc/raydriver/daemon-cert.pem",
  "tls_key": "/etc/raydriver/daemon-key.pem",
  "sessions": [
    {"protocol": "grbl", "config": {"port": "/dev/ttyUSB0", "baudrate": 115200}}
  ]
}
```

## Debug commands

```sh
raydriver scan                    # local serial ports (vid:pid, product)
raydriver scan --network          # also browse for daemons via mDNS
raydriver info /dev/ttyUSB0       # one serial port's details
raydriver info 192.168.1.50:8765  # a daemon: sessions, devices, cameras
raydriver list                    # configured devices + live connection state
raydriver connection              # Rayforge clients connected to the daemon
raydriver daemon status           # service state + daemon probe
```

Every command accepts `--json` for machine-readable output. Daemon
commands read the endpoint and credentials from the config
(`--config` to override, `--token`/`--tls-ca` to override
credentials). `info` also accepts `raydriver://host:port#token`
URLs. `connection` shows each client's address, authentication
state and per-session roles, plus the control lease holder — the
daemon-side view of "who is driving the machine right now".

## Security policy

The analyzer's bind policy applies unchanged: loopback may run
without auth; a non-loopback or wildcard bind refuses to open
without **both** a token and TLS; explicitly configured auth is
never dropped. Clients refuse to send a token over a non-TLS
connection, except to loopback endpoints (`127.0.0.1`, `::1`,
`localhost`), where the token never leaves the machine — so the
config `--setup` writes works as-is for same-machine use. For an
exposed listener, generate a certificate with:

```sh
openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout daemon-key.pem -out daemon-cert.pem -days 365 \
  -subj /CN=raydriver \
  -addext subjectAltName=DNS:localhost,IP:127.0.0.1
```

Add the LAN hostname/IP to `subjectAltName` for remote clients,
and pin the certificate on the client (`tls_ca`). Keep the daemon
port firewalled to the LAN; there is no WAN/NAT traversal.

## Cameras on the device host

Video never crosses the control protocol. The daemon only
*advertises* camera endpoints; Rayforge's existing network-camera
sources (`http_stream`, `http_snapshot`, `rtsp`) consume them.

- IP cameras: add the camera's own stream URL in Rayforge.
- USB cameras on the host: run a standard streamer, e.g.
  `ustreamer --device /dev/video0 --host 0.0.0.0 --port 8080`,
  then advertise it:

```json
{
  "cameras": [
    {"name": "enclosure", "uri": "http://pi.local:8080/stream", "type": "http_stream"}
  ]
}
```

The endpoints surface in `device/list`; the mDNS TXT records
announce how many cameras the daemon has. Query the full list with
the Rust-backed camera helper (token and TLS included):

```python
from raydriver.core.discovery import daemon as daemon_api

cameras = await daemon_api.list_cameras(
    "pi.local",
    8765,
    token="<your token>",
    tls_ca="/path/to/daemon-cert.pem",
)
for camera in cameras:
    print(camera["name"], camera["type"], camera["uri"])
```

`tls_hostname` overrides the verified certificate name when it
differs from the address. A token is only sent over TLS, except to
loopback. Keep video endpoints LAN-only or firewalled — they are
separate from the TLS control channel.

## Clients

**Rayforge**: machine setup is documented on the Rayforge website
— see the
[first-time setup guide](https://rayforge.org/docs/getting-started/first-time-setup).

### Protocol-agnostic access (recommended)

The protocol a device speaks is the **daemon's** knowledge, not the
client's. A client connects to a *device* and lets the daemon
resolve the protocol, exposing it as info afterwards:

```python
from raydriver.remote.client import list_devices, open_session

endpoint = "raydriver://127.0.0.1:8765#<your token>"

for device in await list_devices(endpoint):
    print(device["device"], device["label"])   # /dev/ttyUSB0  GRBL

session = open_session(endpoint, device="/dev/ttyUSB0", dialect={})
await session.connect()
print(session.protocol)            # "grbl" — resolved by the daemon
print(session.metadata["capabilities"])
```

`list_devices` returns the daemon's configured (and open) devices,
each with its `protocol` and `label` as info; `list_protocols`
returns the full metadata table (label, capabilities, config keys,
dialect family). `open_session` accepts a device reference or an
existing `session_id` — never a protocol. The token is the one
printed by `daemon add` / `daemon status`. The endpoint may be a
URL or a dict (`remote_host`, `remote_port`, `token`, `tls_ca`,
`tls_hostname`).

The returned proxy exposes the same method and event surface as a
local session. A device that is not configured on the daemon is
resolved by a best-effort firmware probe, or rejected with
`unknown_device` (add it with `raydriver daemon add`).

### Explicit protocol (lower level)

`create_session(protocol_name, config, …)` still exists for callers
that already know the protocol and want to open an ad-hoc session
with a specific config; it routes to a remote proxy when the config
carries an endpoint. Prefer `open_session` for application code so
the protocol stays the daemon's responsibility.