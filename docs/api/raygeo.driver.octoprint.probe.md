---
title: raygeo.driver.octoprint.probe
sidebar_label: raygeo.driver.octoprint.probe
---

## OctoPrintDeviceInfo

The structured result of probing an OctoPrint server: server identity and the work area the server
reports.

### `max_travel`

```python
max_travel: Optional[tuple[float, float]]
```

### `name`

```python
name: str
```

### `server_version`

```python
server_version: Optional[str]
```

### `to_dict()`

```python
to_dict() -> dict
```

The mapping form of the census (the daemon wire form).

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## Functions

### `describe()`

```python
describe(
    version_info: Optional[dict[str, object]] = None,
    printer_info: Optional[dict[str, object]] = None,
) -> OctoPrintDeviceInfo
```

Builds the census from the parsed OctoPrint API payloads.

Pure data transformation with no I/O; the caller is responsible for the HTTP requests.
*version_info* is the `GET /api/version` response, *printer_info* the `GET /api/printer` response
(either may be `None` when the request failed).

| Parameter      | Type                                 | Description |
| -------------- | ------------------------------------ | ----------- |
| `version_info` | `Optional[dict[str, object]] = None` |             |
| `printer_info` | `Optional[dict[str, object]] = None` |             |
| _Returns_      | `OctoPrintDeviceInfo`                |             |

### `probe_device()`

```python
probe_device(session: Any) -> Awaitable[OctoPrintDeviceInfo]
```

Runs the device census over *session* and resolves to \[`OctoPrintDeviceInfo`\].

| Parameter | Type                             | Description |
| --------- | -------------------------------- | ----------- |
| `session` | `Any`                            |             |
| _Returns_ | `Awaitable[OctoPrintDeviceInfo]` |             |
