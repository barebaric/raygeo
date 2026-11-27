---
title: raygeo.driver.remote.daemon
sidebar_label: raygeo.driver.remote.daemon
---

## Constants

### DEFAULT_LEASE_GRACE

`float`

### DEFAULT_PORT

`int`

### DEFAULT_RING_SIZE

`int`

### SEND_QUEUE_LIMIT

`int`

## ClientQueue

### `get()`

```python
get() -> Awaitable[dict[str, object]]
```

| Parameter | Type                           | Description |
| --------- | ------------------------------ | ----------- |
| _Returns_ | `Awaitable[dict[str, object]]` |             |

### `put()`

```python
put(frame: dict[str, object]) -> None
```

| Parameter | Type                | Description |
| --------- | ------------------- | ----------- |
| `frame`   | `dict[str, object]` |             |
| _Returns_ | `None`              |             |

## Daemon

### `config`

```python
config: DaemonConfig
```

### `start()`

```python
start() -> Awaitable[StartedServer]
```

| Parameter | Type                       | Description |
| --------- | -------------------------- | ----------- |
| _Returns_ | `Awaitable[StartedServer]` |             |

### `stop()`

```python
stop() -> Awaitable[None]
```

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `Awaitable[None]` |             |

## DaemonConfig

### `cameras`

```python
cameras: list[dict[str, object]]
```

### `host`

```python
host: str
```

### `keepalive_interval`

```python
keepalive_interval: float
```

### `keepalive_misses`

```python
keepalive_misses: int
```

### `lease_grace`

```python
lease_grace: float
```

### `mdns`

```python
mdns: bool
```

### `port`

```python
port: int
```

### `ring_size`

```python
ring_size: int
```

### `sessions`

```python
sessions: list[dict[str, object]]
```

### `tls_cert`

```python
tls_cert: Optional[str]
```

### `tls_key`

```python
tls_key: Optional[str]
```

### `token`

```python
token: Optional[str]
```

## SocketInfo

### `getsockname()`

```python
getsockname() -> tuple[str, int]
```

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `tuple[str, int]` |             |

## StartedServer

### `sockets`

```python
sockets: tuple[SocketInfo, ...]
```

## Functions

### `validate_bind_policy()`

```python
validate_bind_policy(
    host: str,
    token: Optional[str],
    tls_cert: Optional[str],
) -> None
```

Raise **raygeo.driver.errors.InsecureListenerError** unless *host* is loopback or both *token* and
*tls_cert* are configured.

| Parameter  | Type            | Description |
| ---------- | --------------- | ----------- |
| `host`     | `str`           |             |
| `token`    | `Optional[str]` |             |
| `tls_cert` | `Optional[str]` |             |
| _Returns_  | `None`          |             |
