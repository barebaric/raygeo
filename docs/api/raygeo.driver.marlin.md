---
title: raygeo.driver.marlin
sidebar_label: raygeo.driver.marlin
---

Marlin device session: strict ping-pong commands, M114 position polling. Dialects remain data owned
by Rayforge: resolved command templates are passed into the session.

## MarlinSerialDriver

A Marlin device session using ping-pong communication.

Sends one command at a time and waits for `ok` before proceeding; the device announces itself with a
`start` line and positions are polled with `M114`. Marlin has no feed-hold support — pausing is the
caller's concern.

All async methods return asyncio futures; a typed `DriverEvent` is delivered to
`event_callback(event)` on the event loop thread.

### `boot_lines`

```python
boot_lines: list[str]
```

Boot banner lines received from the device.

### `probe_device()`

```python
probe_device(
    boot_lines: Optional[Sequence[str]] = None,
) -> Awaitable[probe.MarlinDeviceInfo]
```

Open the connection and start the background tasks. Run the device census: query M115, M211, M503
and M149, and return the structured result as a dict. *boot_lines* are the startup messages the
caller observed on the session, used to refine the firmware version.

| Parameter    | Type                                | Description |
| ------------ | ----------------------------------- | ----------- |
| `boot_lines` | `Optional[Sequence[str]] = None`    |             |
| _Returns_    | `Awaitable[probe.MarlinDeviceInfo]` |             |

### `with_transport()`

```python
with_transport(
    transport: transport.MockTransport,
    config: Optional[Union[dict, config.MarlinConfig]] = None,
    dialect: Optional[dict] = None,
    event_callback: Optional[Callable[[core.DriverEvent], object]] = None,
) -> MarlinSerialDriver
```

Build a session around a \[`MockTransport`\] (for tests).

| Parameter        | Type                                                    | Description |
| ---------------- | ------------------------------------------------------- | ----------- |
| `transport`      | `transport.MockTransport`                               |             |
| `config`         | `Optional[Union[dict, config.MarlinConfig]] = None`     |             |
| `dialect`        | `Optional[dict] = None`                                 |             |
| `event_callback` | `Optional[Callable[[core.DriverEvent], object]] = None` |             |
| _Returns_        | `MarlinSerialDriver`                                    |             |
