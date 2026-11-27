---
title: raygeo.driver.grbl.simple
sidebar_label: raygeo.driver.grbl.simple
---

The minimal ping-pong GRBL driver: one line at a time, waiting for each ok/error acknowledgement.
For hardware where the full driver misbehaves.

## GrblSerialSimpleDriver

A minimal GRBL device driver using ping-pong communication.

Sends one G-code line at a time and waits for `ok` before proceeding. No character-counting, no
buffer management, no deadlock detection — for hardware where the full
\[`GrblSerialDriver`\][raygeo.driver.grbl.GrblSerialDriver] misbehaves.

All async methods return asyncio futures; a typed `DriverEvent` is delivered to
`event_callback(event)` on the event loop thread.

### `probe_device()`

```python
probe_device() -> Awaitable[probe.GrblDeviceInfo]
```

Run the device census: query `$I` and `$$`, and return the structured result.

| Parameter | Type                              | Description |
| --------- | --------------------------------- | ----------- |
| _Returns_ | `Awaitable[probe.GrblDeviceInfo]` |             |

### `with_transport()`

```python
with_transport(
    transport: transport.MockTransport,
    config: Optional[Union[dict, config.GrblSimpleConfig]] = None,
    dialect: Optional[dict] = None,
    event_callback: Optional[Callable[[core.DriverEvent], object]] = None,
) -> GrblSerialSimpleDriver
```

Build a driver around a \[`MockTransport`\] (for tests).

| Parameter        | Type                                                    | Description |
| ---------------- | ------------------------------------------------------- | ----------- |
| `transport`      | `transport.MockTransport`                               |             |
| `config`         | `Optional[Union[dict, config.GrblSimpleConfig]] = None` |             |
| `dialect`        | `Optional[dict] = None`                                 |             |
| `event_callback` | `Optional[Callable[[core.DriverEvent], object]] = None` |             |
| _Returns_        | `GrblSerialSimpleDriver`                                |             |
