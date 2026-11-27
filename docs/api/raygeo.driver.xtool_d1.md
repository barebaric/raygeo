---
title: raygeo.driver.xtool_d1
sidebar_label: raygeo.driver.xtool_d1
---

xTool D-series driver implemented in Rust. XtoolNetworkDriver supports D-series G-code jobs over the
HTTP API and the companion WebSocket notification channel.

## XtoolNetworkDriver

An xTool device session: connection lifecycle, interactive commands and job execution over the
proprietary WiFi API, entirely in Rust.

All async methods return asyncio futures; a typed `DriverEvent` is delivered to
`event_callback(event)` on the event loop thread.

### `is_job_running`

```python
is_job_running: bool
```

### `execute_block()`

```python
execute_block(block: str) -> Awaitable[None]
```

Execute a multi-line block of G-code via POST /cmd.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `block`   | `str`             |             |
| _Returns_ | `Awaitable[None]` |             |

## Functions

### `query_machine_type()`

```python
query_machine_type(
    host: str,
    port: int,
    timeout: float = 2.0,
) -> Awaitable[Optional[str]]
```

Queries the machine type an xTool device reports for itself.

GETs the machine-type endpoint at *host*:*port* and resolves to the reported type when it names an
xTool machine, and `None` on any failure.

| Parameter | Type                       | Description |
| --------- | -------------------------- | ----------- |
| `host`    | `str`                      |             |
| `port`    | `int`                      |             |
| `timeout` | `float = 2.0`              |             |
| _Returns_ | `Awaitable[Optional[str]]` |             |
