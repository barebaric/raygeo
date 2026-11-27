# raydriver CLI

The `raydriver` command drives and debugs machines, hosts remote
sessions, and analyzes traffic. It ships in the `raygeo` package; the
console script keeps its historical name so existing service units and
scripts continue to work.

```bash
pip install raygeo
raydriver --help
```

`python -m raygeo.driver` is equivalent to the console script.

If you previously installed the standalone `raydriver` package, remove
it first with `pip uninstall raydriver`; it would collide with the
console script provided by raygeo. The command refuses to run while the
old distribution is present.

## Run a job

`run` streams G-code to a GRBL device or sends rpascript to a Ruida
controller, showing live progress:

```bash
raydriver run job.gcode --port /dev/ttyUSB0
raydriver run job.gcode --emulator
raydriver run job.rds --protocol ruida --host 192.168.1.50
```

GRBL options include `--port`, `--baudrate`, `--emulator`,
`--speed-factor`, `--no-poll`, and `--timeout`. For Ruida, select
`--protocol ruida` and use `--host`, `--net-port`, `--transport`, or
`--usb-device`; `--no-wait` returns after sending the script.

Driver diagnostics go to stderr. `--log-file PATH` redirects them to a
file, and `RUST_LOG` controls verbosity (for example
`RUST_LOG=debug`).

## Watch device status

```bash
raydriver status --port /dev/ttyUSB0
raydriver status --emulator --seconds 10
```

## Host devices with the daemon

The daemon keeps sessions open next to the hardware and lets clients
elsewhere on the LAN connect to them:

```bash
raydriver daemon add /dev/ttyUSB0
raydriver daemon add /dev/ttyACM0 --protocol MarlinSerialDriver
raydriver daemon remove /dev/ttyUSB0
raydriver daemon --setup
raydriver daemon
```

`daemon --setup` writes a token-only config without starting the
service. Use `--config PATH` to select another config file.

Install, inspect, or remove the platform service:

```bash
raydriver daemon install
raydriver daemon status
raydriver daemon uninstall
```

The config lives in the platform per-user directory
(`~/.config/raydriver/daemon.json` on Linux,
`~/Library/Application Support/raydriver/daemon.json` on macOS, and
`%APPDATA%\raydriver\daemon.json` on Windows). `daemon --setup` writes
a loopback config with a generated token. A non-loopback bind requires
both a token and TLS; clients never send a token over an unencrypted
non-loopback connection.

`daemon install` uses systemd on Linux (a user unit by default, or
`--system` for a system unit), launchd on macOS, and a logon scheduled
task on Windows. `daemon status` shows the service state, the token,
and a live probe of the daemon.

## Analyze traffic

```bash
raydriver analyzer record --port /dev/ttyUSB0 --out cap.jsonl
raydriver analyzer record --emulator --seconds 5 --out cap.jsonl
raydriver analyzer tui --emulator
raydriver analyzer decode cap.jsonl
raydriver analyzer replay cap.jsonl --to 192.168.1.50:23
raydriver analyzer proxy --upstream 192.168.1.50:23 --out mitm.jsonl
```

`record` and `tui` accept `--protocol ruida` together with `--host` or
`--usb-device`. `replay` and `proxy` can require or supply tokens and
TLS options; run `raydriver analyzer <command> --help` for details.

## Discover and inspect devices

```bash
raydriver scan
raydriver scan --network --json
raydriver discover --ports /dev/ttyUSB0 /dev/ttyACM0
raydriver discover --tcp 23 --probe "?" --match ok
raydriver info /dev/ttyUSB0
raydriver info 192.168.1.50:8765
raydriver list
raydriver connection
```

`scan` lists local serial ports and, with `--network`, browses for
daemons via mDNS. `discover` runs the full serial, mDNS, and subnet
discovery engine. `info`, `list`, and `connection` inspect a daemon or
its sessions; they accept `--config`, `--token`, and `--tls-ca`, and
also understand `raydriver://host:port#token` endpoints.

All debug commands accept `--json` for machine-readable output.
