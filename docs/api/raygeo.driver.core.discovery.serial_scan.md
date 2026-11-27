---
title: raygeo.driver.core.discovery.serial_scan
sidebar_label: raygeo.driver.core.discovery.serial_scan
---

## Constants

### BAUD_CANDIDATES

`list[int]`

### list_port_info

`Callable[[], list[ports.SerialPortInfo]]`

## PortObservation

Raw output captured from one port during a scan.

### `baud_rate`

```python
baud_rate: int
```

### `data`

```python
data: bytes
```

Everything the device sent while the port was open.

### `info`

```python
info: Optional[ports.SerialPortInfo]
```

USB metadata for the port, when available.

### `port`

```python
port: str
```

## Functions

### `looks_like_device_output()`

```python
looks_like_device_output(data: Sequence[int]) -> bool
```

Generic, firmware-agnostic test for "a device is talking on this port": the capture contains at
least one line of mostly-printable ASCII. Output received at a wrong baud rate is typically framing
garbage and fails this test.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| `data`    | `Sequence[int]` |             |
| _Returns_ | `bool`          |             |

### `scan_serial_ports()`

```python
scan_serial_ports(
    ports: Optional[Iterable[str]] = None,
    baud_rates: Optional[Iterable[int]] = None,
    exclude_ports: Optional[Iterable[str]] = None,
    timeout: Optional[float] = None,
) -> Awaitable[list[PortObservation]]
```

Probes every *port* at *baud_rates*, all ports in parallel, and returns one observation per port
that produced device-like output. Ports named in *exclude_ports* are skipped entirely (used by
callers that hold devices on those ports). Ports that are busy, absent, or silent are skipped
silently.

When *timeout* is given the scan stops waiting after that many seconds: observations collected so
far are still returned and probes still in flight are cancelled, so a slow or silent adapter can
never push a responsive machine past the deadline. Results keep the order of *ports*.

| Parameter       | Type                               | Description |
| --------------- | ---------------------------------- | ----------- |
| `ports`         | `Optional[Iterable[str]] = None`   |             |
| `baud_rates`    | `Optional[Iterable[int]] = None`   |             |
| `exclude_ports` | `Optional[Iterable[str]] = None`   |             |
| `timeout`       | `Optional[float] = None`           |             |
| _Returns_       | `Awaitable[list[PortObservation]]` |             |
