---
title: raygeo.driver.core.discovery.mdns_scan
sidebar_label: raygeo.driver.core.discovery.mdns_scan
---

## MDNSService

One service resolved during an mDNS browse.

### `host`

```python
host: str
```

Address to connect to; IPv4 preferred over IPv6.

### `name`

```python
name: str
```

Instance name, e.g. `OctoPrint on octopi`.

### `port`

```python
port: int
```

### `server`

```python
server: str
```

The server's own hostname, e.g. `octopi.local`.

### `service_type`

```python
service_type: str
```

Normalized service type, e.g. `_octoprint._tcp`.

### `txt`

```python
txt: dict
```

TXT record contents, values decoded to strings.

## Functions

### `normalize_service_type()`

```python
normalize_service_type(service_type: str) -> str
```

Reduces a service type to its canonical `_name._transport` form: `_octoprint._tcp.local.` becomes
`_octoprint._tcp`.

| Parameter      | Type  | Description |
| -------------- | ----- | ----------- |
| `service_type` | `str` |             |
| _Returns_      | `str` |             |

### `scan_mdns_services()`

```python
scan_mdns_services(
    service_types: Iterable[str],
    browse_window: float = 2.0,
) -> Awaitable[list[MDNSService]]
```

Browses *service_types* over mDNS for a short window and returns every service that could be
resolved to an address. Failures (no network, no responder) are logged and yield an empty list
rather than raising.

| Parameter       | Type                           | Description |
| --------------- | ------------------------------ | ----------- |
| `service_types` | `Iterable[str]`                |             |
| `browse_window` | `float = 2.0`                  |             |
| _Returns_       | `Awaitable[list[MDNSService]]` |             |
