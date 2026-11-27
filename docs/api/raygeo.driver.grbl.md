---
title: raygeo.driver.grbl
sidebar_label: raygeo.driver.grbl
---

GRBL device drivers implemented in Rust.

GrblSerialDriver, GrblTelnetDriver, GrblBluetoothDriver and GrblNetworkDriver each own a connection
lifecycle (handshake, status polling, reconnect), interactive commands, and job streaming with
character-counting flow control, stall detection and deadlock recovery; they differ only in their
transport. GrblSerialSimpleDriver (raygeo.driver.grbl.simple) is the minimal alternative: strict
ping-pong, for hardware where the full driver misbehaves. Dialects remain data owned by the
application: resolved command templates are passed into the driver.

## GrblBluetoothDriver

A GRBL-compatible device on a Bluetooth serial channel.

### `probe_device()`

```python
probe_device() -> Awaitable[probe.GrblDeviceInfo]
```

Run the device census: query `$I` and `$$`.

| Parameter | Type                              | Description |
| --------- | --------------------------------- | ----------- |
| _Returns_ | `Awaitable[probe.GrblDeviceInfo]` |             |

### `with_transport()`

```python
with_transport(
    transport: transport.MockTransport,
    config: Optional[Union[dict, config.GrblBluetoothConfig]] = None,
    dialect: Optional[dict] = None,
    event_callback: Optional[Callable[[core.DriverEvent], object]] = None,
) -> GrblBluetoothDriver
```

Build a driver around a \[`MockTransport`\] (for tests).

| Parameter        | Type                                                       | Description |
| ---------------- | ---------------------------------------------------------- | ----------- |
| `transport`      | `transport.MockTransport`                                  |             |
| `config`         | `Optional[Union[dict, config.GrblBluetoothConfig]] = None` |             |
| `dialect`        | `Optional[dict] = None`                                    |             |
| `event_callback` | `Optional[Callable[[core.DriverEvent], object]] = None`    |             |
| _Returns_        | `GrblBluetoothDriver`                                      |             |

## GrblNetworkDriver

A GRBL-compatible device on the FluidNC / ESP3D HTTP + WebSocket interface.

### `probe_device()`

```python
probe_device() -> Awaitable[probe.GrblDeviceInfo]
```

Run the device census: query `$I` and `$$`.

| Parameter | Type                              | Description |
| --------- | --------------------------------- | ----------- |
| _Returns_ | `Awaitable[probe.GrblDeviceInfo]` |             |

### `with_transport()`

```python
with_transport(
    transport: transport.MockTransport,
    config: Optional[Union[dict, config.Esp3dConfig]] = None,
    dialect: Optional[dict] = None,
    event_callback: Optional[Callable[[core.DriverEvent], object]] = None,
) -> GrblNetworkDriver
```

Build a driver around a \[`MockTransport`\] (for tests).

| Parameter        | Type                                                    | Description |
| ---------------- | ------------------------------------------------------- | ----------- |
| `transport`      | `transport.MockTransport`                               |             |
| `config`         | `Optional[Union[dict, config.Esp3dConfig]] = None`      |             |
| `dialect`        | `Optional[dict] = None`                                 |             |
| `event_callback` | `Optional[Callable[[core.DriverEvent], object]] = None` |             |
| _Returns_        | `GrblNetworkDriver`                                     |             |

## GrblSerialDriver

A GRBL-compatible device on a serial port: connection lifecycle, streaming and interactive commands,
entirely in Rust.

### `probe_device()`

```python
probe_device() -> Awaitable[probe.GrblDeviceInfo]
```

Run the device census: query `$I` and `$$`.

| Parameter | Type                              | Description |
| --------- | --------------------------------- | ----------- |
| _Returns_ | `Awaitable[probe.GrblDeviceInfo]` |             |

### `with_transport()`

```python
with_transport(
    transport: transport.MockTransport,
    config: Optional[Union[dict, config.GrblSerialConfig]] = None,
    dialect: Optional[dict] = None,
    event_callback: Optional[Callable[[core.DriverEvent], object]] = None,
) -> GrblSerialDriver
```

Build a driver around a \[`MockTransport`\] (for tests).

| Parameter        | Type                                                    | Description |
| ---------------- | ------------------------------------------------------- | ----------- |
| `transport`      | `transport.MockTransport`                               |             |
| `config`         | `Optional[Union[dict, config.GrblSerialConfig]] = None` |             |
| `dialect`        | `Optional[dict] = None`                                 |             |
| `event_callback` | `Optional[Callable[[core.DriverEvent], object]] = None` |             |
| _Returns_        | `GrblSerialDriver`                                      |             |

## GrblTelnetDriver

A GRBL-compatible device over a telnet connection.

### `probe_device()`

```python
probe_device() -> Awaitable[probe.GrblDeviceInfo]
```

Run the device census: query `$I` and `$$`.

| Parameter | Type                              | Description |
| --------- | --------------------------------- | ----------- |
| _Returns_ | `Awaitable[probe.GrblDeviceInfo]` |             |

### `with_transport()`

```python
with_transport(
    transport: transport.MockTransport,
    config: Optional[Union[dict, config.GrblTelnetConfig]] = None,
    dialect: Optional[dict] = None,
    event_callback: Optional[Callable[[core.DriverEvent], object]] = None,
) -> GrblTelnetDriver
```

Build a driver around a \[`MockTransport`\] (for tests).

| Parameter        | Type                                                    | Description |
| ---------------- | ------------------------------------------------------- | ----------- |
| `transport`      | `transport.MockTransport`                               |             |
| `config`         | `Optional[Union[dict, config.GrblTelnetConfig]] = None` |             |
| `dialect`        | `Optional[dict] = None`                                 |             |
| `event_callback` | `Optional[Callable[[core.DriverEvent], object]] = None` |             |
| _Returns_        | `GrblTelnetDriver`                                      |             |
