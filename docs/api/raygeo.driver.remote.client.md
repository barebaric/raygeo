---
title: raygeo.driver.remote.client
sidebar_label: raygeo.driver.remote.client
---

## Constants

### DEFAULT_PORT

`int`

### REMOTE_KEYS

`list[str]`

## RemoteSession

The proxy with the union of the sessions' method surfaces.

### `buffer_count`

```python
buffer_count: int
```

### `job_running`

```python
job_running: bool
```

### `metadata`

```python
metadata: Optional[dict[str, object]]
```

### `pending_commands`

```python
pending_commands: list[str]
```

### `protocol`

```python
protocol: Optional[str]
```

### `resource_uri`

```python
resource_uri: Optional[str]
```

### `rx_buffer_size`

```python
rx_buffer_size: int
```

### `state`

```python
state: types.DeviceState
```

### `aclose()`

```python
aclose() -> Awaitable[None]
```

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `Awaitable[None]` |             |

### `attach()`

```python
attach() -> Awaitable[None]
```

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `Awaitable[None]` |             |

### `cancel()`

```python
cancel(emergency: bool = False) -> Awaitable[None]
```

| Parameter   | Type              | Description |
| ----------- | ----------------- | ----------- |
| `emergency` | `bool = False`    |             |
| _Returns_   | `Awaitable[None]` |             |

### `clear_alarm()`

```python
clear_alarm() -> Awaitable[None]
```

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `Awaitable[None]` |             |

### `connect()`

```python
connect() -> Awaitable[None]
```

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `Awaitable[None]` |             |

### `detect_unit_system()`

```python
detect_unit_system() -> Awaitable[Optional[str]]
```

| Parameter | Type                       | Description |
| --------- | -------------------------- | ----------- |
| _Returns_ | `Awaitable[Optional[str]]` |             |

### `disconnect()`

```python
disconnect() -> Awaitable[None]
```

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `Awaitable[None]` |             |

### `drop_connection()`

```python
drop_connection(message: Optional[str] = None) -> None
```

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| `message` | `Optional[str] = None` |             |
| _Returns_ | `None`                 |             |

### `execute_command()`

```python
execute_command(command: str) -> Awaitable[list[str]]
```

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| `command` | `str`                  |             |
| _Returns_ | `Awaitable[list[str]]` |             |

### `execute_interactive_command()`

```python
execute_interactive_command(command: str) -> Awaitable[list[str]]
```

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| `command` | `str`                  |             |
| _Returns_ | `Awaitable[list[str]]` |             |

### `home()`

```python
home(
    axes: Optional[Sequence[str]] = None,
    active_wcs: Optional[str] = None,
) -> Awaitable[None]
```

| Parameter    | Type                             | Description |
| ------------ | -------------------------------- | ----------- |
| `axes`       | `Optional[Sequence[str]] = None` |             |
| `active_wcs` | `Optional[str] = None`           |             |
| _Returns_    | `Awaitable[None]`                |             |

### `jog()`

```python
jog(
    speed: float,
    deltas: Optional[Sequence[tuple[str, float]]] = None,
) -> Awaitable[None]
```

| Parameter | Type                                           | Description |
| --------- | ---------------------------------------------- | ----------- |
| `speed`   | `float`                                        |             |
| `deltas`  | `Optional[Sequence[tuple[str, float]]] = None` |             |
| _Returns_ | `Awaitable[None]`                              |             |

### `move_to()`

```python
move_to(
    speed: float,
    x: float,
    y: float,
    z: Optional[float] = None,
) -> Awaitable[None]
```

| Parameter | Type                     | Description |
| --------- | ------------------------ | ----------- |
| `speed`   | `float`                  |             |
| `x`       | `float`                  |             |
| `y`       | `float`                  |             |
| `z`       | `Optional[float] = None` |             |
| _Returns_ | `Awaitable[None]`        |             |

### `probe_device()`

```python
probe_device(
    boot_lines: Optional[Sequence[str]] = None,
) -> Awaitable[grbl.probe.GrblDeviceInfo | marlin.probe.MarlinDeviceInfo | smoothie.probe.SmoothieDeviceInfo | octoprint.probe.OctoPrintDeviceInfo]
```

Run the device census over the session; marlin sessions accept the boot lines captured at connect
time.

| Parameter    | Type                                                                                                                                                            | Description |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- |
| `boot_lines` | `Optional[Sequence[str]] = None`                                                                                                                                |             |
| _Returns_    | `Awaitable[grbl.probe.GrblDeviceInfo &#124; marlin.probe.MarlinDeviceInfo &#124; smoothie.probe.SmoothieDeviceInfo &#124; octoprint.probe.OctoPrintDeviceInfo]` |             |

### `read_parser_state()`

```python
read_parser_state() -> Awaitable[Optional[str]]
```

| Parameter | Type                       | Description |
| --------- | -------------------------- | ----------- |
| _Returns_ | `Awaitable[Optional[str]]` |             |

### `read_settings()`

```python
read_settings() -> Awaitable[list[tuple[str, str]]]
```

| Parameter | Type                               | Description |
| --------- | ---------------------------------- | ----------- |
| _Returns_ | `Awaitable[list[tuple[str, str]]]` |             |

### `read_wcs_offsets()`

```python
read_wcs_offsets() -> Awaitable[dict[str, tuple[float, float, float]]]
```

| Parameter | Type                                               | Description |
| --------- | -------------------------------------------------- | ----------- |
| _Returns_ | `Awaitable[dict[str, tuple[float, float, float]]]` |             |

### `run()`

```python
run(
    gcode: str,
    op_line_map: Optional[Mapping[int, int]] = None,
    op_estimates: Optional[Sequence[float]] = None,
    progress_callback: Optional[Callable[[int], object]] = None,
) -> Awaitable[None]
```

| Parameter           | Type                                       | Description |
| ------------------- | ------------------------------------------ | ----------- |
| `gcode`             | `str`                                      |             |
| `op_line_map`       | `Optional[Mapping[int, int]] = None`       |             |
| `op_estimates`      | `Optional[Sequence[float]] = None`         |             |
| `progress_callback` | `Optional[Callable[[int], object]] = None` |             |
| _Returns_           | `Awaitable[None]`                          |             |

### `run_probe_cycle()`

```python
run_probe_cycle(
    axis_letter: str,
    max_travel: float,
    feed_rate: float,
) -> Awaitable[Optional[tuple[float, float, float]]]
```

| Parameter     | Type                                              | Description |
| ------------- | ------------------------------------------------- | ----------- |
| `axis_letter` | `str`                                             |             |
| `max_travel`  | `float`                                           |             |
| `feed_rate`   | `float`                                           |             |
| _Returns_     | `Awaitable[Optional[tuple[float, float, float]]]` |             |

### `run_raw()`

```python
run_raw(machine_code: str) -> Awaitable[None]
```

| Parameter      | Type              | Description |
| -------------- | ----------------- | ----------- |
| `machine_code` | `str`             |             |
| _Returns_      | `Awaitable[None]` |             |

### `select_tool()`

```python
select_tool(tool_number: int) -> Awaitable[None]
```

| Parameter     | Type              | Description |
| ------------- | ----------------- | ----------- |
| `tool_number` | `int`             |             |
| _Returns_     | `Awaitable[None]` |             |

### `set_focus_power()`

```python
set_focus_power(power: Optional[float] = None) -> Awaitable[None]
```

| Parameter | Type                     | Description |
| --------- | ------------------------ | ----------- |
| `power`   | `Optional[float] = None` |             |
| _Returns_ | `Awaitable[None]`        |             |

### `set_hold()`

```python
set_hold(hold: bool = True) -> Awaitable[None]
```

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `hold`    | `bool = True`     |             |
| _Returns_ | `Awaitable[None]` |             |

### `set_power()`

```python
set_power(power: Optional[float] = None) -> Awaitable[None]
```

| Parameter | Type                     | Description |
| --------- | ------------------------ | ----------- |
| `power`   | `Optional[float] = None` |             |
| _Returns_ | `Awaitable[None]`        |             |

### `set_wcs_offset()`

```python
set_wcs_offset(
    wcs_slot: str,
    x: float,
    y: float,
    z: Optional[float] = None,
) -> Awaitable[None]
```

| Parameter  | Type                     | Description |
| ---------- | ------------------------ | ----------- |
| `wcs_slot` | `str`                    |             |
| `x`        | `float`                  |             |
| `y`        | `float`                  |             |
| `z`        | `Optional[float] = None` |             |
| _Returns_  | `Awaitable[None]`        |             |

### `update_dialect()`

```python
update_dialect(dialect: dict[str, object]) -> None
```

| Parameter | Type                | Description |
| --------- | ------------------- | ----------- |
| `dialect` | `dict[str, object]` |             |
| _Returns_ | `None`              |             |

### `write_setting()`

```python
write_setting(key: str, value: str) -> Awaitable[None]
```

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `key`     | `str`             |             |
| `value`   | `str`             |             |
| _Returns_ | `Awaitable[None]` |             |

## Functions

### `create_session()`

```python
create_session(
    protocol_name: str,
    config: Union[dict[str, object], config.GrblSerialConfig, config.GrblSimpleConfig, config.GrblTelnetConfig, config.GrblBluetoothConfig, config.Esp3dConfig, config.SmoothieConfig, config.MarlinConfig, config.XtoolConfig, config.OctoPrintConfig, config.M2Config],
    dialect: Optional[dict[str, object]],
    event_callback: Optional[Callable[[core.DriverEvent], object]],
) -> Union[RemoteSession, grbl.GrblSerialDriver, simple.GrblSerialSimpleDriver, smoothie.SmoothieDriver, octoprint.OctoPrintDriver, marlin.MarlinSerialDriver]
```

| Parameter        | Type                                                                                                                                                                                                                                                           | Description |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- |
| `protocol_name`  | `str`                                                                                                                                                                                                                                                          |             |
| `config`         | `Union[dict[str, object], config.GrblSerialConfig, config.GrblSimpleConfig, config.GrblTelnetConfig, config.GrblBluetoothConfig, config.Esp3dConfig, config.SmoothieConfig, config.MarlinConfig, config.XtoolConfig, config.OctoPrintConfig, config.M2Config]` |             |
| `dialect`        | `Optional[dict[str, object]]`                                                                                                                                                                                                                                  |             |
| `event_callback` | `Optional[Callable[[core.DriverEvent], object]]`                                                                                                                                                                                                               |             |
| _Returns_        | `Union[RemoteSession, grbl.GrblSerialDriver, simple.GrblSerialSimpleDriver, smoothie.SmoothieDriver, octoprint.OctoPrintDriver, marlin.MarlinSerialDriver]`                                                                                                    |             |

### `list_devices()`

```python
list_devices(
    endpoint: Union[dict[str, object], str],
) -> Awaitable[list[dict[str, object]]]
```

| Parameter  | Type                                 | Description |
| ---------- | ------------------------------------ | ----------- |
| `endpoint` | `Union[dict[str, object], str]`      |             |
| _Returns_  | `Awaitable[list[dict[str, object]]]` |             |

### `list_drivers()`

```python
list_drivers(
    endpoint: Union[dict[str, object], str],
) -> Awaitable[list[dict[str, object]]]
```

| Parameter  | Type                                 | Description |
| ---------- | ------------------------------------ | ----------- |
| `endpoint` | `Union[dict[str, object], str]`      |             |
| _Returns_  | `Awaitable[list[dict[str, object]]]` |             |

### `open_session()`

```python
open_session(
    endpoint: Union[dict[str, object], str],
    device: Optional[str] = None,
    session_id: Optional[str] = None,
    config: Optional[Union[dict[str, object], config.GrblSerialConfig, config.GrblSimpleConfig, config.GrblTelnetConfig, config.GrblBluetoothConfig, config.Esp3dConfig, config.SmoothieConfig, config.MarlinConfig, config.XtoolConfig, config.OctoPrintConfig, config.M2Config]] = None,
    dialect: Optional[dict[str, object]] = None,
    event_callback: Optional[Callable[[core.DriverEvent], object]] = None,
) -> RemoteSession
```

| Parameter        | Type                                                                                                                                                                                                                                                                            | Description |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- |
| `endpoint`       | `Union[dict[str, object], str]`                                                                                                                                                                                                                                                 |             |
| `device`         | `Optional[str] = None`                                                                                                                                                                                                                                                          |             |
| `session_id`     | `Optional[str] = None`                                                                                                                                                                                                                                                          |             |
| `config`         | `Optional[Union[dict[str, object], config.GrblSerialConfig, config.GrblSimpleConfig, config.GrblTelnetConfig, config.GrblBluetoothConfig, config.Esp3dConfig, config.SmoothieConfig, config.MarlinConfig, config.XtoolConfig, config.OctoPrintConfig, config.M2Config]] = None` |             |
| `dialect`        | `Optional[dict[str, object]] = None`                                                                                                                                                                                                                                            |             |
| `event_callback` | `Optional[Callable[[core.DriverEvent], object]] = None`                                                                                                                                                                                                                         |             |
| _Returns_        | `RemoteSession`                                                                                                                                                                                                                                                                 |             |
