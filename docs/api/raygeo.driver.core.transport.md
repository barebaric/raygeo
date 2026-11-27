---
title: raygeo.driver.core.transport
sidebar_label: raygeo.driver.core.transport
---

## MockTransport

An in-memory byte transport for tests.

Feed bytes to the session with `push()` (as if the device had sent them), inspect everything the
session sent with `sent()`, and simulate link state with `set_connected()` / `set_write_error()`.

### `is_connected`

```python
is_connected: bool
```

True while the transport link is open (i.e. a session connected through this mock).

### `clear_sent()`

```python
clear_sent() -> None
```

Clear the recorded sent bytes.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |

### `push()`

```python
push(data: Sequence[int]) -> None
```

Deliver bytes to the session as if the device sent them.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| `data`    | `Sequence[int]` |             |
| _Returns_ | `None`          |             |

### `sent()`

```python
sent() -> list[bytes]
```

All bytes written to the device so far.

| Parameter | Type          | Description |
| --------- | ------------- | ----------- |
| _Returns_ | `list[bytes]` |             |

### `set_connected()`

```python
set_connected(connected: bool) -> None
```

Simulate connect/disconnect of the physical link.

| Parameter   | Type   | Description |
| ----------- | ------ | ----------- |
| `connected` | `bool` |             |
| _Returns_   | `None` |             |

### `set_write_error()`

```python
set_write_error(message: Optional[str] = None) -> None
```

Fail subsequent writes with this message (and mark the link down), simulating an unplugged cable.

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| `message` | `Optional[str] = None` |             |
| _Returns_ | `None`                 |             |

### `take_new_sent()`

```python
take_new_sent() -> list[bytes]
```

Writes recorded since the previous call (or the last `clear_sent`): a monotonic cursor, so clearing
the log never skips or replays chunks. This is what device emulators should consume.

| Parameter | Type          | Description |
| --------- | ------------- | ----------- |
| _Returns_ | `list[bytes]` |             |
