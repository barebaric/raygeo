---
title: raygeo.driver.remote.inspect
sidebar_label: raygeo.driver.remote.inspect
---

## Functions

### `daemon_rpc()`

```python
daemon_rpc(
    host: str,
    port: int,
    token: Optional[str],
    tls_ca: Optional[str],
    requests: Sequence[tuple[str, dict[str, object]]],
) -> Awaitable[tuple[Optional[dict[str, object]], list[dict[str, object]]]]
```

| Parameter  | Type                                                                     | Description |
| ---------- | ------------------------------------------------------------------------ | ----------- |
| `host`     | `str`                                                                    |             |
| `port`     | `int`                                                                    |             |
| `token`    | `Optional[str]`                                                          |             |
| `tls_ca`   | `Optional[str]`                                                          |             |
| `requests` | `Sequence[tuple[str, dict[str, object]]]`                                |             |
| _Returns_  | `Awaitable[tuple[Optional[dict[str, object]], list[dict[str, object]]]]` |             |

### `snapshot()`

```python
snapshot(
    host: str,
    port: int,
    token: Optional[str] = None,
    tls_ca: Optional[str] = None,
) -> Awaitable[dict[str, object]]
```

| Parameter | Type                           | Description |
| --------- | ------------------------------ | ----------- |
| `host`    | `str`                          |             |
| `port`    | `int`                          |             |
| `token`   | `Optional[str] = None`         |             |
| `tls_ca`  | `Optional[str] = None`         |             |
| _Returns_ | `Awaitable[dict[str, object]]` |             |

### `welcome_probe()`

```python
welcome_probe(host: str, port: int) -> Awaitable[Optional[dict[str, object]]]
```

| Parameter | Type                                     | Description |
| --------- | ---------------------------------------- | ----------- |
| `host`    | `str`                                    |             |
| `port`    | `int`                                    |             |
| _Returns_ | `Awaitable[Optional[dict[str, object]]]` |             |
