---
title: raygeo.driver.smoothie
sidebar_label: raygeo.driver.smoothie
---

Smoothieware device session: strict ping-pong commands, Grbl-style status reports. Dialects remain
data owned by Rayforge: resolved command templates are passed into the session.

## SmoothieDriver

A Smoothieware device session using ping-pong communication.

Sends one command at a time and waits for `ok` before proceeding; status reports are Grbl-style
`<...>` replies to `?`.

All async methods return asyncio futures; a typed `DriverEvent` is delivered to
`event_callback(event)` on the event loop thread.

### `probe_device()`

```python
probe_device() -> Awaitable[probe.SmoothieDeviceInfo]
```

Send a command and await its full response. Run the device census: query `version` and the
`config-get` motion keys, and return the structured result as a dict.

| Parameter | Type                                  | Description |
| --------- | ------------------------------------- | ----------- |
| _Returns_ | `Awaitable[probe.SmoothieDeviceInfo]` |             |

### `with_transport()`

```python
with_transport(
    transport: transport.MockTransport,
    config: Optional[Union[dict, config.SmoothieConfig]] = None,
    dialect: Optional[dict] = None,
    event_callback: Optional[Callable[[core.DriverEvent], object]] = None,
) -> SmoothieDriver
```

Build a session around a \[`MockTransport`\] (for tests).

| Parameter        | Type                                                    | Description |
| ---------------- | ------------------------------------------------------- | ----------- |
| `transport`      | `transport.MockTransport`                               |             |
| `config`         | `Optional[Union[dict, config.SmoothieConfig]] = None`   |             |
| `dialect`        | `Optional[dict] = None`                                 |             |
| `event_callback` | `Optional[Callable[[core.DriverEvent], object]] = None` |             |
| _Returns_        | `SmoothieDriver`                                        |             |
