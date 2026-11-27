---
title: raygeo.driver
sidebar_label: raygeo.driver
---

The common driver base class and its typed metadata.

Every protocol driver is a subclass of Driver: the uniform surface (connection lifecycle, job
streaming, machine control, probing and settings) is declared here once, with strict types, and
inherited by every driver class. Operations a protocol cannot perform raise NotImplementedError.

## Constants

### drivers

`list[type[Driver]]`

## Driver

The base class of every raydriver driver.

Declares the uniform surface once: connection lifecycle, job streaming, machine control, probing and
settings. Operations a protocol cannot perform raise `NotImplementedError`; the method is still
present on every driver.

### `buffer_count`

```python
buffer_count: int
```

Bytes currently occupying the device's RX buffer.

### `info`

```python
info: DriverInfo
```

The driver's typed identity and capabilities.

### `job_running`

```python
job_running: bool
```

True while a job is running or held.

### `pending_commands`

```python
pending_commands: list[str]
```

Commands sent but not yet acknowledged.

### `resource_uri`

```python
resource_uri: Optional[str]
```

Identifier of the physical resource, or `None`.

### `rx_buffer_size`

```python
rx_buffer_size: int
```

The device's RX buffer size in bytes, or 0 when unknown.

### `state`

```python
state: types.DeviceState
```

Current snapshot of the device state.

### `cancel()`

```python
cancel(emergency: bool = False) -> Awaitable[None]
```

Cancel the running job.

| Parameter   | Type              | Description |
| ----------- | ----------------- | ----------- |
| `emergency` | `bool = False`    |             |
| _Returns_   | `Awaitable[None]` |             |

### `clear_alarm()`

```python
clear_alarm() -> Awaitable[None]
```

Clear a device alarm.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `Awaitable[None]` |             |

### `connect()`

```python
connect() -> Awaitable[None]
```

Open the connection and keep it up until `disconnect`.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `Awaitable[None]` |             |

### `detect_unit_system()`

```python
detect_unit_system() -> Awaitable[Optional[str]]
```

Detect the device's unit system (`"metric"` or `"imperial"`), or `None` when unknown.

| Parameter | Type                       | Description |
| --------- | -------------------------- | ----------- |
| _Returns_ | `Awaitable[Optional[str]]` |             |

### `disconnect()`

```python
disconnect() -> Awaitable[None]
```

Stop all background tasks and close the connection.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `Awaitable[None]` |             |

### `execute_command()`

```python
execute_command(command: str) -> Awaitable[list[str]]
```

Send one command and return its response lines.

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| `command` | `str`                  |             |
| _Returns_ | `Awaitable[list[str]]` |             |

### `execute_interactive_command()`

```python
execute_interactive_command(command: str) -> Awaitable[list[str]]
```

Send one interactive command and return its response lines.

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

Home all axes or the given axes.

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

Relative jog by per-axis distances.

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

Absolute move in the machine's current coordinate frame.

| Parameter | Type                     | Description |
| --------- | ------------------------ | ----------- |
| `speed`   | `float`                  |             |
| `x`       | `float`                  |             |
| `y`       | `float`                  |             |
| `z`       | `Optional[float] = None` |             |
| _Returns_ | `Awaitable[None]`        |             |

### `read_parser_state()`

```python
read_parser_state() -> Awaitable[Optional[str]]
```

Read the G-code parser state.

| Parameter | Type                       | Description |
| --------- | -------------------------- | ----------- |
| _Returns_ | `Awaitable[Optional[str]]` |             |

### `read_settings()`

```python
read_settings() -> Awaitable[list[tuple[str, str]]]
```

Read the device's firmware settings.

| Parameter | Type                               | Description |
| --------- | ---------------------------------- | ----------- |
| _Returns_ | `Awaitable[list[tuple[str, str]]]` |             |

### `read_wcs_offsets()`

```python
read_wcs_offsets() -> Awaitable[dict[str, tuple[float, float, float]]]
```

Read the work-coordinate-system offsets.

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

Stream a G-code job.

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

Run a probe cycle; `None` = no trigger reported.

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

Execute pre-rendered machine code.

| Parameter      | Type              | Description |
| -------------- | ----------------- | ----------- |
| `machine_code` | `str`             |             |
| _Returns_      | `Awaitable[None]` |             |

### `select_tool()`

```python
select_tool(tool_number: int) -> Awaitable[None]
```

Select a tool.

| Parameter     | Type              | Description |
| ------------- | ----------------- | ----------- |
| `tool_number` | `int`             |             |
| _Returns_     | `Awaitable[None]` |             |

### `set_focus_power()`

```python
set_focus_power(power: Optional[float] = None) -> Awaitable[None]
```

Set the laser power used while focusing.

| Parameter | Type                     | Description |
| --------- | ------------------------ | ----------- |
| `power`   | `Optional[float] = None` |             |
| _Returns_ | `Awaitable[None]`        |             |

### `set_hold()`

```python
set_hold(hold: bool = True) -> Awaitable[None]
```

Pause or resume the running job.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `hold`    | `bool = True`     |             |
| _Returns_ | `Awaitable[None]` |             |

### `set_power()`

```python
set_power(power: Optional[float] = None) -> Awaitable[None]
```

Set the laser/spindle power; `None` disables the tool.

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

Write a work-coordinate-system offset.

| Parameter  | Type                     | Description |
| ---------- | ------------------------ | ----------- |
| `wcs_slot` | `str`                    |             |
| `x`        | `float`                  |             |
| `y`        | `float`                  |             |
| `z`        | `Optional[float] = None` |             |
| _Returns_  | `Awaitable[None]`        |             |

### `update_dialect()`

```python
update_dialect(dialect: dict) -> None
```

Replace the command templates.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `dialect` | `dict` |             |
| _Returns_ | `None` |             |

### `write_setting()`

```python
write_setting(key: str, value: str) -> Awaitable[None]
```

Write one firmware setting.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| `key`     | `str`             |             |
| `value`   | `str`             |             |
| _Returns_ | `Awaitable[None]` |             |

## DriverInfo

Typed identity and capabilities of one driver class.

### `capabilities`

```python
capabilities: dict[str, bool]
```

The capability flags.

### `config_keys`

```python
config_keys: list[str]
```

The connection config keys the driver accepts.

### `connection`

```python
connection: str
```

The physical connection the driver dials.

### `machine_space_wcs`

```python
machine_space_wcs: str
```

The WCS id that denotes machine space.

### `maturity`

```python
maturity: str
```

The maturity level (`stable`, `untested`, ...).

### `name`

```python
name: str
```

The class name, which is also the persisted driver name.

### `setup`

```python
setup: list[SetupField]
```

The typed connection-setup form.

### `supported_wcs`

```python
supported_wcs: list[str]
```

The selectable work-coordinate systems.

### `uses_gcode`

```python
uses_gcode: bool
```

Whether the protocol speaks G-code.

### `to_dict()`

```python
to_dict() -> dict
```

The wire form served by `driver/list`.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## SetupField

One connection-setup field a driver accepts.

### `choices`

```python
choices: list[str]
```

The allowed values for a choice field.

### `default`

```python
default: Optional[str]
```

The default value, if any.

### `key`

```python
key: str
```

The config key.

### `kind`

```python
kind: str
```

The value kind (`serial_port`, `hostname`, `port`, ...).

### `max`

```python
max: Optional[float]
```

The maximum value for numeric fields.

### `min`

```python
min: Optional[float]
```

The minimum value for numeric fields.

### `required`

```python
required: bool
```

Whether the field must be configured.

### `to_dict()`

```python
to_dict() -> dict
```

The mapping form.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `dict` |             |

## Functions

### `driver_info()`

```python
driver_info(name: str) -> DriverInfo
```

Look up a built-in driver's typed metadata by class name.

| Parameter | Type         | Description |
| --------- | ------------ | ----------- |
| `name`    | `str`        |             |
| _Returns_ | `DriverInfo` |             |

### `get_driver_cls()`

```python
get_driver_cls(name: str, default: Optional[Any] = None) -> type[Driver]
```

Resolve a persisted driver name.

| Parameter | Type                   | Description |
| --------- | ---------------------- | ----------- |
| `name`    | `str`                  |             |
| `default` | `Optional[Any] = None` |             |
| _Returns_ | `type[Driver]`         |             |

### `register_driver()`

```python
register_driver(driver: type[Driver]) -> None
```

Register an addon driver class.

| Parameter | Type           | Description |
| --------- | -------------- | ----------- |
| `driver`  | `type[Driver]` |             |
| _Returns_ | `None`         |             |
