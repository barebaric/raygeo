---
title: raygeo.driver.core.discovery.ports
sidebar_label: raygeo.driver.core.discovery.ports
---

## Constants

### list_serial_ports

`Callable[[], list[dict[str, object]]]`

## SerialPortInfo

A serial port path together with optional metadata used to identify the underlying USB device.

### `description`

```python
description: Optional[str]
```

### `device`

```python
device: str
```

### `manufacturer`

```python
manufacturer: Optional[str]
```

### `pid`

```python
pid: Optional[int]
```

### `vid`

```python
vid: Optional[int]
```

## Functions

### `check_permission_issue()`

```python
check_permission_issue() -> str
```

Diagnoses systemic serial-port access problems (missing `dialout` group membership, snap
confinement). Returns a structured result; the application supplies the localized message.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `str` |             |

### `glob_ports()`

```python
glob_ports(dir: str, prefix: str) -> list[str]
```

Glob-style prefix listing of one directory; the enumeration backend used by the sandbox-safe
listing. Exposed as a module attribute so tests can replace it.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| `dir`     | `str`       |             |
| `prefix`  | `str`       |             |
| _Returns_ | `list[str]` |             |

### `is_usb_serial_port()`

```python
is_usb_serial_port(port: str) -> bool
```

Heuristically determines if a port path refers to a USB serial device. On non-POSIX systems (e.g.
Windows), every port is treated as potentially USB.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `port`    | `str`  |             |
| _Returns_ | `bool` |             |

### `list_baud_rates()`

```python
list_baud_rates() -> list[int]
```

The common serial baud rates offered to applications.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[int]` |             |

### `list_port_info()`

```python
list_port_info() -> list[SerialPortInfo]
```

Lists available serial ports together with a human-readable description when the platform provides
one. USB adapters are listed first.

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| _Returns_ | `list[SerialPortInfo]` |             |

### `list_ports()`

```python
list_ports() -> list[str]
```

Lists available serial ports, USB adapters first.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[str]` |             |

### `list_usb_ports()`

```python
list_usb_ports() -> list[str]
```

Like list_ports, but only returns USB serial ports.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[str]` |             |

### `natural_key()`

```python
natural_key(s: str) -> list
```

Splits a string into lowercase words and numbers so ttyUSB2 sorts before ttyUSB10.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `s`       | `str`  |             |
| _Returns_ | `list` |             |

### `resolve_vidpid()`

```python
resolve_vidpid(vid: int, pid: int) -> Optional[str]
```

Resolves a USB `vid:pid` pair (e.g. FTDI 0x0403/0x6001) to a concrete device path by scanning the
available ports; `None` when no matching device is present.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| `vid`     | `int`           |             |
| `pid`     | `int`           |             |
| _Returns_ | `Optional[str]` |             |

### `safe_list_ports_linux()`

```python
safe_list_ports_linux() -> list[str]
```

A non-crashing port listing for sandboxed Linux envs.

Direct enumeration of /dev/ttyS\* is forbidden by the snap sandbox and crashes pyserial-style
scanners; only the common USB-to-serial patterns permitted by the serial-port interface are globbed.

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[str]` |             |

### `sort_ports()`

```python
sort_ports(ports: Sequence[str]) -> list[str]
```

Sorts ports so USB serial adapters come first, followed by all other ports. Each group is ordered
naturally, e.g. ttyUSB2 before ttyUSB10.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| `ports`   | `Sequence[str]` |             |
| _Returns_ | `list[str]`     |             |
