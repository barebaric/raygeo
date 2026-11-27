---
title: raygeo.driver.remote.mdns
sidebar_label: raygeo.driver.remote.mdns
---

## Constants

### SERVICE_TYPE

`str`

## Announcer

### `info`

```python
info: ServiceInfo
```

### `close()`

```python
close() -> Awaitable[None]
```

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `Awaitable[None]` |             |

## ServiceInfo

### `port`

```python
port: int
```

### `properties`

```python
properties: dict[bytes, bytes]
```

### `parsed_addresses()`

```python
parsed_addresses() -> list[str]
```

| Parameter | Type        | Description |
| --------- | ----------- | ----------- |
| _Returns_ | `list[str]` |             |

## Functions

### `_announce_address()`

```python
_announce_address(config: daemon.DaemonConfig) -> str
```

| Parameter | Type                  | Description |
| --------- | --------------------- | ----------- |
| `config`  | `daemon.DaemonConfig` |             |
| _Returns_ | `str`                 |             |

### `_service_name()`

```python
_service_name() -> str
```

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| _Returns_ | `str` |             |

### `_service_name_for()`

```python
_service_name_for(host: str) -> str
```

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| `host`    | `str` |             |
| _Returns_ | `str` |             |

### `announce()`

```python
announce(
    config: daemon.DaemonConfig,
    port: int,
    name: Optional[str] = None,
) -> Awaitable[Optional[Announcer]]
```

| Parameter | Type                             | Description |
| --------- | -------------------------------- | ----------- |
| `config`  | `daemon.DaemonConfig`            |             |
| `port`    | `int`                            |             |
| `name`    | `Optional[str] = None`           |             |
| _Returns_ | `Awaitable[Optional[Announcer]]` |             |

### `browse_daemons()`

```python
browse_daemons(timeout: float = 3.0) -> Awaitable[list[dict[str, object]]]
```

| Parameter | Type                                 | Description |
| --------- | ------------------------------------ | ----------- |
| `timeout` | `float = 3.0`                        |             |
| _Returns_ | `Awaitable[list[dict[str, object]]]` |             |
