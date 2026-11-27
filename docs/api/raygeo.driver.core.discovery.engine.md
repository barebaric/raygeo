---
title: raygeo.driver.core.discovery.engine
sidebar_label: raygeo.driver.core.discovery.engine
---

## Constants

### SUBNET_SCAN_TIMEOUT

`float`

### find_network_devices

`Callable[..., Awaitable[list[discovery.DiscoveredDevice]]]`

### find_raygeo_devices

`Callable[..., Awaitable[list[discovery.DiscoveredDevice]]]`

### find_serial_devices

`Callable[..., Awaitable[list[discovery.DiscoveredDevice]]]`

## Functions

### `find_all_devices()`

```python
find_all_devices(
    driver_classes: Sequence[type],
    ports: Optional[Iterable[str]] = None,
    baud_rates: Optional[Iterable[int]] = None,
    exclude_ports: Optional[Iterable[str]] = None,
) -> Awaitable[list[discovery.DiscoveredDevice]]
```

Runs every discovery channel concurrently and returns the merged, deduplicated results.

*exclude_ports* keeps the serial scan away from ports whose devices are already held (e.g. being
probed by the caller). Channels never raise: a failing or slow scan yields no devices from that
channel only.

| Parameter        | Type                                          | Description |
| ---------------- | --------------------------------------------- | ----------- |
| `driver_classes` | `Sequence[type]`                              |             |
| `ports`          | `Optional[Iterable[str]] = None`              |             |
| `baud_rates`     | `Optional[Iterable[int]] = None`              |             |
| `exclude_ports`  | `Optional[Iterable[str]] = None`              |             |
| _Returns_        | `Awaitable[list[discovery.DiscoveredDevice]]` |             |

### `find_subnet_devices()`

```python
find_subnet_devices(
    driver_classes: Sequence[type],
) -> Awaitable[list[discovery.DiscoveredDevice]]
```

Scans the local subnets for the TCP ports declared by drivers' `TcpRecognizer` and returns the
devices the Rust core confirmed with each driver's probe. Finds controllers that announce nothing
via mDNS, such as grblHAL boards with the raw telnet service.

| Parameter        | Type                                          | Description |
| ---------------- | --------------------------------------------- | ----------- |
| `driver_classes` | `Sequence[type]`                              |             |
| _Returns_        | `Awaitable[list[discovery.DiscoveredDevice]]` |             |
