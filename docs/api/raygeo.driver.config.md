---
title: raygeo.driver.config
sidebar_label: raygeo.driver.config
---

## Esp3dConfig

ESP3D / FluidNC network session configuration.

### `cached_rx_buffer_size`

```python
cached_rx_buffer_size: Optional[int]
```

### `command_timeout`

```python
command_timeout: float
```

### `connect_timeout`

```python
connect_timeout: float
```

### `deadlock_detection`

```python
deadlock_detection: bool
```

### `handshake_poll_interval`

```python
handshake_poll_interval: float
```

### `handshake_timeout`

```python
handshake_timeout: float
```

### `homing_timeout`

```python
homing_timeout: float
```

### `host`

```python
host: str
```

### `http_port`

```python
http_port: int
```

### `poll_response_attempts`

```python
poll_response_attempts: int
```

### `poll_response_interval`

```python
poll_response_interval: float
```

### `poll_status_while_running`

```python
poll_status_while_running: bool
```

### `protocol`

```python
protocol: str
```

### `reconnect_delay`

```python
reconnect_delay: float
```

### `request_timeout`

```python
request_timeout: float
```

### `rx_buffer_size_override`

```python
rx_buffer_size_override: int
```

### `safety_shutdown_delay`

```python
safety_shutdown_delay: float
```

### `stall_timeout_default`

```python
stall_timeout_default: float
```

### `stall_timeout_max`

```python
stall_timeout_max: float
```

### `stall_timeout_min`

```python
stall_timeout_min: float
```

### `stall_timeout_safety_factor`

```python
stall_timeout_safety_factor: float
```

### `status_poll_interval`

```python
status_poll_interval: float
```

### `unanswered_poll_limit`

```python
unanswered_poll_limit: int
```

### `ws_port`

```python
ws_port: int
```

### `to_dict()`

```python
to_dict() -> dict
```

The mapping form expected by the session constructors; fields that are None are omitted.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## GrblBluetoothConfig

GRBL over Bluetooth Classic SPP/RFCOMM.

### `address`

```python
address: str
```

### `cached_rx_buffer_size`

```python
cached_rx_buffer_size: Optional[int]
```

### `channel`

```python
channel: int
```

### `command_timeout`

```python
command_timeout: float
```

### `deadlock_detection`

```python
deadlock_detection: bool
```

### `handshake_poll_interval`

```python
handshake_poll_interval: float
```

### `handshake_timeout`

```python
handshake_timeout: float
```

### `homing_timeout`

```python
homing_timeout: float
```

### `poll_response_attempts`

```python
poll_response_attempts: int
```

### `poll_response_interval`

```python
poll_response_interval: float
```

### `poll_status_while_running`

```python
poll_status_while_running: bool
```

### `reconnect_delay`

```python
reconnect_delay: float
```

### `rx_buffer_size_override`

```python
rx_buffer_size_override: int
```

### `safety_shutdown_delay`

```python
safety_shutdown_delay: float
```

### `stall_timeout_default`

```python
stall_timeout_default: float
```

### `stall_timeout_max`

```python
stall_timeout_max: float
```

### `stall_timeout_min`

```python
stall_timeout_min: float
```

### `stall_timeout_safety_factor`

```python
stall_timeout_safety_factor: float
```

### `status_poll_interval`

```python
status_poll_interval: float
```

### `unanswered_poll_limit`

```python
unanswered_poll_limit: int
```

### `to_dict()`

```python
to_dict() -> dict
```

The mapping form expected by the session constructors; fields that are None are omitted.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## GrblOptions

Timing and flow-control options shared by GRBL drivers.

### `cached_rx_buffer_size`

```python
cached_rx_buffer_size: Optional[int]
```

### `command_timeout`

```python
command_timeout: float
```

### `deadlock_detection`

```python
deadlock_detection: bool
```

### `handshake_poll_interval`

```python
handshake_poll_interval: float
```

### `handshake_timeout`

```python
handshake_timeout: float
```

### `homing_timeout`

```python
homing_timeout: float
```

### `poll_response_attempts`

```python
poll_response_attempts: int
```

### `poll_response_interval`

```python
poll_response_interval: float
```

### `poll_status_while_running`

```python
poll_status_while_running: bool
```

### `reconnect_delay`

```python
reconnect_delay: float
```

### `rx_buffer_size_override`

```python
rx_buffer_size_override: int
```

### `safety_shutdown_delay`

```python
safety_shutdown_delay: float
```

### `stall_timeout_default`

```python
stall_timeout_default: float
```

### `stall_timeout_max`

```python
stall_timeout_max: float
```

### `stall_timeout_min`

```python
stall_timeout_min: float
```

### `stall_timeout_safety_factor`

```python
stall_timeout_safety_factor: float
```

### `status_poll_interval`

```python
status_poll_interval: float
```

### `unanswered_poll_limit`

```python
unanswered_poll_limit: int
```

### `to_dict()`

```python
to_dict() -> dict
```

The mapping form expected by the session constructors; fields that are None are omitted.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## GrblSerialConfig

GRBL serial session configuration.

### `baudrate`

```python
baudrate: int
```

### `cached_rx_buffer_size`

```python
cached_rx_buffer_size: Optional[int]
```

### `command_timeout`

```python
command_timeout: float
```

### `deadlock_detection`

```python
deadlock_detection: bool
```

### `handshake_poll_interval`

```python
handshake_poll_interval: float
```

### `handshake_timeout`

```python
handshake_timeout: float
```

### `homing_timeout`

```python
homing_timeout: float
```

### `poll_response_attempts`

```python
poll_response_attempts: int
```

### `poll_response_interval`

```python
poll_response_interval: float
```

### `poll_status_while_running`

```python
poll_status_while_running: bool
```

### `port`

```python
port: str
```

### `reconnect_delay`

```python
reconnect_delay: float
```

### `rx_buffer_size_override`

```python
rx_buffer_size_override: int
```

### `safety_shutdown_delay`

```python
safety_shutdown_delay: float
```

### `stall_timeout_default`

```python
stall_timeout_default: float
```

### `stall_timeout_max`

```python
stall_timeout_max: float
```

### `stall_timeout_min`

```python
stall_timeout_min: float
```

### `stall_timeout_safety_factor`

```python
stall_timeout_safety_factor: float
```

### `status_poll_interval`

```python
status_poll_interval: float
```

### `unanswered_poll_limit`

```python
unanswered_poll_limit: int
```

### `to_dict()`

```python
to_dict() -> dict
```

The mapping form expected by the session constructors; fields that are None are omitted.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## GrblSimpleConfig

Configuration for the minimal ping-pong GRBL session.

### `baudrate`

```python
baudrate: int
```

### `cached_rx_buffer_size`

```python
cached_rx_buffer_size: Optional[int]
```

### `command_timeout`

```python
command_timeout: float
```

### `deadlock_detection`

```python
deadlock_detection: bool
```

### `handshake_poll_interval`

```python
handshake_poll_interval: float
```

### `handshake_timeout`

```python
handshake_timeout: float
```

### `homing_timeout`

```python
homing_timeout: float
```

### `poll_response_attempts`

```python
poll_response_attempts: int
```

### `poll_response_interval`

```python
poll_response_interval: float
```

### `poll_status_while_running`

```python
poll_status_while_running: bool
```

### `port`

```python
port: str
```

### `reconnect_delay`

```python
reconnect_delay: float
```

### `rx_buffer_size_override`

```python
rx_buffer_size_override: int
```

### `safety_shutdown_delay`

```python
safety_shutdown_delay: float
```

### `stall_timeout_default`

```python
stall_timeout_default: float
```

### `stall_timeout_max`

```python
stall_timeout_max: float
```

### `stall_timeout_min`

```python
stall_timeout_min: float
```

### `stall_timeout_safety_factor`

```python
stall_timeout_safety_factor: float
```

### `status_poll_interval`

```python
status_poll_interval: float
```

### `unanswered_poll_limit`

```python
unanswered_poll_limit: int
```

### `to_dict()`

```python
to_dict() -> dict
```

The mapping form expected by the session constructors; fields that are None are omitted.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## GrblTelnetConfig

GRBL telnet session configuration.

### `cached_rx_buffer_size`

```python
cached_rx_buffer_size: Optional[int]
```

### `command_timeout`

```python
command_timeout: float
```

### `deadlock_detection`

```python
deadlock_detection: bool
```

### `handshake_poll_interval`

```python
handshake_poll_interval: float
```

### `handshake_timeout`

```python
handshake_timeout: float
```

### `homing_timeout`

```python
homing_timeout: float
```

### `host`

```python
host: str
```

### `poll_response_attempts`

```python
poll_response_attempts: int
```

### `poll_response_interval`

```python
poll_response_interval: float
```

### `poll_status_while_running`

```python
poll_status_while_running: bool
```

### `reconnect_delay`

```python
reconnect_delay: float
```

### `rx_buffer_size_override`

```python
rx_buffer_size_override: int
```

### `safety_shutdown_delay`

```python
safety_shutdown_delay: float
```

### `stall_timeout_default`

```python
stall_timeout_default: float
```

### `stall_timeout_max`

```python
stall_timeout_max: float
```

### `stall_timeout_min`

```python
stall_timeout_min: float
```

### `stall_timeout_safety_factor`

```python
stall_timeout_safety_factor: float
```

### `status_poll_interval`

```python
status_poll_interval: float
```

### `tcp_port`

```python
tcp_port: int
```

### `unanswered_poll_limit`

```python
unanswered_poll_limit: int
```

### `to_dict()`

```python
to_dict() -> dict
```

The mapping form expected by the session constructors; fields that are None are omitted.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## M2Config

M2 WS-V2 monitoring session configuration.

### `connect_timeout`

```python
connect_timeout: float
```

### `host`

```python
host: str
```

### `poll_interval`

```python
poll_interval: float
```

### `port`

```python
port: int
```

### `reconnect_delay`

```python
reconnect_delay: float
```

### `request_timeout`

```python
request_timeout: float
```

### `tls_allow_invalid_certs`

```python
tls_allow_invalid_certs: bool
```

### `to_dict()`

```python
to_dict() -> dict
```

The mapping form expected by the session constructors; fields that are None are omitted.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## MarlinConfig

Marlin serial session configuration.

### `baudrate`

```python
baudrate: int
```

### `command_timeout`

```python
command_timeout: float
```

### `connect_settle`

```python
connect_settle: float
```

### `handshake_poll_interval`

```python
handshake_poll_interval: float
```

### `handshake_timeout`

```python
handshake_timeout: float
```

### `poll_status_while_running`

```python
poll_status_while_running: bool
```

### `port`

```python
port: str
```

### `reconnect_delay`

```python
reconnect_delay: float
```

### `status_poll_interval`

```python
status_poll_interval: float
```

### `to_dict()`

```python
to_dict() -> dict
```

The mapping form expected by the session constructors; fields that are None are omitted.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## OctoPrintConfig

Connection settings for an OctoPrint server.

### `api_key`

```python
api_key: str
```

### `connect_timeout`

```python
connect_timeout: float
```

### `host`

```python
host: str
```

### `path`

```python
path: str
```

### `poll_interval`

```python
poll_interval: float
```

### `port`

```python
port: int
```

### `reconnect_delay`

```python
reconnect_delay: float
```

### `request_timeout`

```python
request_timeout: float
```

### `ws_ping_interval`

```python
ws_ping_interval: float
```

### `to_dict()`

```python
to_dict() -> dict
```

The mapping form expected by the session constructors; fields that are None are omitted.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## RemoteEndpoint

The daemon endpoint used by remote sessions.

### `remote_host`

```python
remote_host: str
```

### `remote_port`

```python
remote_port: int
```

### `tls_ca`

```python
tls_ca: Optional[str]
```

### `tls_hostname`

```python
tls_hostname: Optional[str]
```

### `token`

```python
token: Optional[str]
```

### `to_dict()`

```python
to_dict() -> dict
```

The mapping form expected by the session constructors; fields that are None are omitted.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## SmoothieConfig

Smoothieware telnet session configuration.

### `command_timeout`

```python
command_timeout: float
```

### `handshake_poll_interval`

```python
handshake_poll_interval: float
```

### `handshake_timeout`

```python
handshake_timeout: float
```

### `host`

```python
host: str
```

### `poll_status_while_running`

```python
poll_status_while_running: bool
```

### `reconnect_delay`

```python
reconnect_delay: float
```

### `safety_shutdown_delay`

```python
safety_shutdown_delay: float
```

### `status_poll_interval`

```python
status_poll_interval: float
```

### `tcp_port`

```python
tcp_port: int
```

### `to_dict()`

```python
to_dict() -> dict
```

The mapping form expected by the session constructors; fields that are None are omitted.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## XtoolConfig

xTool D-series WiFi-API session configuration.

### `connect_timeout`

```python
connect_timeout: float
```

### `homing_timeout`

```python
homing_timeout: float
```

### `host`

```python
host: str
```

### `http_port`

```python
http_port: int
```

### `job_start_timeout`

```python
job_start_timeout: float
```

### `poll_interval`

```python
poll_interval: float
```

### `reconnect_delay`

```python
reconnect_delay: float
```

### `request_timeout`

```python
request_timeout: float
```

### `ws_port`

```python
ws_port: int
```

### `to_dict()`

```python
to_dict() -> dict
```

The mapping form expected by the session constructors; fields that are None are omitted.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |
