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

See the [daemon guide](driver/daemon.md) for configuration, TLS,
cameras, and the security policy.

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
