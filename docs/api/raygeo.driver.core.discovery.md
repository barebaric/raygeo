---
title: raygeo.driver.core.discovery
sidebar_label: raygeo.driver.core.discovery
---

## DeviceIdentity

Everything known about a discovered device that can be used to match it against a known device
profile.

### `banner`

```python
banner: Optional[str]
```

### `firmware`

```python
firmware: Optional[str]
```

### `tokens`

```python
tokens: frozenset[str]
```

The identity tokens as a frozenset of strings.

### `usb_pid`

```python
usb_pid: Optional[int]
```

### `usb_vid`

```python
usb_vid: Optional[int]
```

## DiscoveredDevice

A device found by evaluating scan output against a driver's discovery declaration.

### `detail`

```python
detail: str
```

### `driver_name`

```python
driver_name: str
```

### `identity`

```python
identity: DeviceIdentity
```

### `key`

```python
key: str
```

Stable identity for deduplication in the UI.

### `label`

```python
label: str
```

### `params`

```python
params: dict[str, object]
```

### `probe_name`

```python
probe_name: Optional[str]
```

The machine name the device itself reported, if any.

### `probe_profile`

```python
probe_profile: Optional[object]
```

Profile data collected by probing the device after discovery; attached by the embedding
application's UI. Opaque here: the embedding application's profile object, not a core type.

## Functions

### `device_key()`

```python
device_key(driver_name: str, params: dict[str, object]) -> str
```

Stable identity for a device from its driver and connection parameters: serial ports by path,
network devices by host and TCP port, remote devices by daemon endpoint plus device resource.

| Parameter     | Type                | Description |
| ------------- | ------------------- | ----------- |
| `driver_name` | `str`               |             |
| `params`      | `dict[str, object]` |             |
| _Returns_     | `str`               |             |

### `find_subnet_devices()`

```python
find_subnet_devices(
    specs: Sequence[dict[str, object]],
    subnets: Optional[Sequence[tuple[str, int]]] = None,
) -> Awaitable[list[DiscoveredDevice]]
```

Scan the local subnets for declared TCP ports and confirm each reachable host with its spec's probe.

*specs* is a list of dicts with the keys `id` (opaque caller identifier), `port`, and optionally
`probe_command` (sent to confirm the firmware) and `match_prefix` (response-line prefix identifying
it), plus the declaring driver's display fields (`driver_name`, `label`, `firmware`). Without a
probe command any listener on the port is claimed. Returns one **DiscoveredDevice** per claimed
host; the first matching spec claims a host.

*subnets* overrides the interface enumeration as (address, prefixlen) pairs; None scans the local
interfaces.

| Parameter | Type                                         | Description |
| --------- | -------------------------------------------- | ----------- |
| `specs`   | `Sequence[dict[str, object]]`                |             |
| `subnets` | `Optional[Sequence[tuple[str, int]]] = None` |             |
| _Returns_ | `Awaitable[list[DiscoveredDevice]]`          |             |

### `local_subnets()`

```python
local_subnets() -> list[tuple[str, int]]
```

The local non-loopback IPv4 interfaces as (address, prefixlen) pairs, for subnet scans.

| Parameter | Type                    | Description |
| --------- | ----------------------- | ----------- |
| _Returns_ | `list[tuple[str, int]]` |             |

### `scan_mdns_services()`

```python
scan_mdns_services(
    service_types: Sequence[str],
    browse_window: float = 2.0,
) -> Awaitable[list[dict[str, object]]]
```

Browse mDNS for the given service types.

Returns one dict per resolved service with the keys `service_type` (normalized, e.g. `_http._tcp`),
`name`, `host`, `port`, `server` and `txt` (a str->str dict). `browse_window` is the listen time in
seconds.

| Parameter       | Type                                 | Description |
| --------------- | ------------------------------------ | ----------- |
| `service_types` | `Sequence[str]`                      |             |
| `browse_window` | `float = 2.0`                        |             |
| _Returns_       | `Awaitable[list[dict[str, object]]]` |             |

### `scan_serial_ports()`

```python
scan_serial_ports(
    ports: Sequence[str],
    baud_rates: Optional[Sequence[int]] = None,
    timeout: Optional[float] = None,
) -> Awaitable[list[dict[str, object]]]
```

Probe serial ports for device output.

Opens each port once, tries the baud rates in order and captures everything a talking device sends.
Returns one dict per talking port with the keys `port`, `baud_rate` and `data` (bytes). `timeout`
bounds the whole scan; observations collected so far are still returned when it expires.

| Parameter    | Type                                 | Description |
| ------------ | ------------------------------------ | ----------- |
| `ports`      | `Sequence[str]`                      |             |
| `baud_rates` | `Optional[Sequence[int]] = None`     |             |
| `timeout`    | `Optional[float] = None`             |             |
| _Returns_    | `Awaitable[list[dict[str, object]]]` |             |

### `scan_subnet()`

```python
scan_subnet(
    hosts: Sequence[str],
    port: int,
    connect_timeout: float = 0.5,
    concurrency: int = 32,
) -> Awaitable[list[str]]
```

TCP-connect to *port* on every host and return the hosts that accept a connection, in input order.

At most *concurrency* connects run in flight, each bounded by *connect_timeout* seconds. Intended
for explicit, user-requested scans only.

| Parameter         | Type                   | Description |
| ----------------- | ---------------------- | ----------- |
| `hosts`           | `Sequence[str]`        |             |
| `port`            | `int`                  |             |
| `connect_timeout` | `float = 0.5`          |             |
| `concurrency`     | `int = 32`             |             |
| _Returns_         | `Awaitable[list[str]]` |             |

### `subnet_hosts()`

```python
subnet_hosts(address: str, prefixlen: int) -> list[str]
```

The host addresses to probe on the subnet containing *address*.

Networks wider than /24 are narrowed to the /24 around the address; network and broadcast addresses
are excluded.

| Parameter   | Type        | Description |
| ----------- | ----------- | ----------- |
| `address`   | `str`       |             |
| `prefixlen` | `int`       |             |
| _Returns_   | `list[str]` |             |
