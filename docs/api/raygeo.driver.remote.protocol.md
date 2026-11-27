---
title: raygeo.driver.remote.protocol
sidebar_label: raygeo.driver.remote.protocol
---

## Constants

### AUTH

`str`

### AUTH_FAILED

`str`

### BAD_REQUEST

`str`

### BAD_VERSION

`str`

### CLIENT_LIST

`str`

### DEFAULT_KEEPALIVE_INTERVAL

`float`

### DEFAULT_KEEPALIVE_MISSES

`int`

### DEVICE_LIST

`str`

### DRIVERS

`list[str]`

### DRIVER_LIST

`str`

### EVENT

`str`

### FRAME_TOO_LARGE

`str`

### INTERNAL

`str`

### JOB_RUNNING

`str`

### KIND_JOB_ERROR

`str`

### KIND_LEASE

`str`

### KIND_PROGRESS

`str`

### LEASE_HELD

`str`

### LEASE_TAKEN

`str`

### LENGTH_PREFIX

`LengthPrefix`

### MAX_FRAME_BYTES

`int`

### PING

`str`

### PONG

`str`

### PROTOCOL_VERSION

`int`

### ProtocolError

`type`

### REQUEST_TYPES

`list[str]`

### SESSION_ATTACH

`str`

### SESSION_CLOSE

`str`

### SESSION_COMMAND

`str`

### SESSION_DETACH

`str`

### SESSION_LIST

`str`

### SESSION_OPEN

`str`

### SESSION_PROPERTY

`str`

### UNKNOWN_DEVICE

`str`

### UNKNOWN_METHOD

`str`

### UNKNOWN_PROPERTY

`str`

### UNKNOWN_SESSION

`str`

### UNKNOWN_TYPE

`str`

### WELCOME

`str`

## LengthPrefix

### `size`

```python
size: int
```

### `pack()`

```python
pack(value: int) -> bytes
```

| Parameter | Type    | Description |
| --------- | ------- | ----------- |
| `value`   | `int`   |             |
| _Returns_ | `bytes` |             |

## Functions

### `decode_event()`

```python
decode_event(payload: dict[str, object]) -> Optional[core.DriverEvent]
```

Decodes an event frame payload; `None` when it is not an event.

| Parameter | Type                         | Description |
| --------- | ---------------------------- | ----------- |
| `payload` | `dict[str, object]`          |             |
| _Returns_ | `Optional[core.DriverEvent]` |             |

### `decode_frame()`

```python
decode_frame(
    payload: Sequence[int],
    max_bytes: Optional[int] = None,
) -> dict[str, object]
```

| Parameter   | Type                   | Description |
| ----------- | ---------------------- | ----------- |
| `payload`   | `Sequence[int]`        |             |
| `max_bytes` | `Optional[int] = None` |             |
| _Returns_   | `dict[str, object]`    |             |

### `decode_state()`

```python
decode_state(payload: dict[str, object]) -> types.DeviceState
```

| Parameter | Type                | Description |
| --------- | ------------------- | ----------- |
| `payload` | `dict[str, object]` |             |
| _Returns_ | `types.DeviceState` |             |

### `encode_event()`

```python
encode_event(event: core.DriverEvent) -> dict[str, object]
```

| Parameter | Type                | Description |
| --------- | ------------------- | ----------- |
| `event`   | `core.DriverEvent`  |             |
| _Returns_ | `dict[str, object]` |             |

### `encode_frame()`

```python
encode_frame(
    message: dict[str, object],
    max_bytes: Optional[int] = None,
) -> bytes
```

| Parameter   | Type                   | Description |
| ----------- | ---------------------- | ----------- |
| `message`   | `dict[str, object]`    |             |
| `max_bytes` | `Optional[int] = None` |             |
| _Returns_   | `bytes`                |             |

### `encode_state()`

```python
encode_state(state: types.DeviceState) -> dict[str, object]
```

| Parameter | Type                | Description |
| --------- | ------------------- | ----------- |
| `state`   | `types.DeviceState` |             |
| _Returns_ | `dict[str, object]` |             |

### `error_response()`

```python
error_response(msg_id: int, code: str, message: str) -> dict[str, object]
```

| Parameter | Type                | Description |
| --------- | ------------------- | ----------- |
| `msg_id`  | `int`               |             |
| `code`    | `str`               |             |
| `message` | `str`               |             |
| _Returns_ | `dict[str, object]` |             |

### `event_frame()`

```python
event_frame(
    session_id: str,
    seq: int,
    event: dict[str, object],
) -> dict[str, object]
```

| Parameter    | Type                | Description |
| ------------ | ------------------- | ----------- |
| `session_id` | `str`               |             |
| `seq`        | `int`               |             |
| `event`      | `dict[str, object]` |             |
| _Returns_    | `dict[str, object]` |             |

### `read_frame()`

```python
read_frame(reader: object) -> Awaitable[dict[str, object]]
```

| Parameter | Type                           | Description |
| --------- | ------------------------------ | ----------- |
| `reader`  | `object`                       |             |
| _Returns_ | `Awaitable[dict[str, object]]` |             |

### `request()`

```python
request(
    msg_id: int,
    type_: str,
    payload: Optional[dict[str, object]] = None,
) -> dict[str, object]
```

| Parameter | Type                                 | Description |
| --------- | ------------------------------------ | ----------- |
| `msg_id`  | `int`                                |             |
| `type_`   | `str`                                |             |
| `payload` | `Optional[dict[str, object]] = None` |             |
| _Returns_ | `dict[str, object]`                  |             |

### `response()`

```python
response(
    msg_id: int,
    payload: Optional[dict[str, object]] = None,
) -> dict[str, object]
```

| Parameter | Type                                 | Description |
| --------- | ------------------------------------ | ----------- |
| `msg_id`  | `int`                                |             |
| `payload` | `Optional[dict[str, object]] = None` |             |
| _Returns_ | `dict[str, object]`                  |             |

### `welcome_payload()`

```python
welcome_payload(
    raydriver_version: str,
    auth_required: bool,
) -> dict[str, object]
```

| Parameter           | Type                | Description |
| ------------------- | ------------------- | ----------- |
| `raydriver_version` | `str`               |             |
| `auth_required`     | `bool`              |             |
| _Returns_           | `dict[str, object]` |             |

### `write_frame()`

```python
write_frame(writer: object, message: dict[str, object]) -> Awaitable[None]
```

| Parameter | Type                | Description |
| --------- | ------------------- | ----------- |
| `writer`  | `object`            |             |
| `message` | `dict[str, object]` |             |
| _Returns_ | `Awaitable[None]`   |             |
