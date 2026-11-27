---
title: raygeo.driver.octoprint
sidebar_label: raygeo.driver.octoprint
---

OctoPrint server support implemented in Rust. OctoPrintDriver submits G-code jobs over the server's
REST API and tracks printer state via its SockJS WebSocket (with REST polling fallback).

## OctoPrintDriver

An OctoPrint server session: job submission and live printer state over the REST API and SockJS
WebSocket.

All async methods return asyncio futures; a typed `DriverEvent` is delivered to
`event_callback(event)` on the event loop thread. Operations the OctoPrint API cannot perform (e.g.
writing firmware settings) raise `NotImplementedError`.

### `move_to_absolute()`

```python
move_to_absolute(
    x: float,
    y: float,
    z: Optional[float] = None,
    speed: Optional[float] = None,
) -> Awaitable[None]
```

Move to an absolute position through the printhead jog API. Without a speed the server uses its
configured default.

| Parameter | Type                     | Description |
| --------- | ------------------------ | ----------- |
| `x`       | `float`                  |             |
| `y`       | `float`                  |             |
| `z`       | `Optional[float] = None` |             |
| `speed`   | `Optional[float] = None` |             |
| _Returns_ | `Awaitable[None]`        |             |

### `probe_device()`

```python
probe_device() -> Awaitable[probe.OctoPrintDeviceInfo]
```

Run the device census over the session: the parsed `/api/version` and `/api/printer` responses,
interpreted into the structured census dict. Local and remote sessions probe identically.

| Parameter | Type                                   | Description |
| --------- | -------------------------------------- | ----------- |
| _Returns_ | `Awaitable[probe.OctoPrintDeviceInfo]` |             |

### `probe_info()`

```python
probe_info() -> Awaitable[list[dict[str, object]]]
```

Fetch the census payloads (the parsed `/api/version` and `/api/printer` responses) over the session,
so local and remote probing use the same code path.

| Parameter | Type                                 | Description |
| --------- | ------------------------------------ | ----------- |
| _Returns_ | `Awaitable[list[dict[str, object]]]` |             |
