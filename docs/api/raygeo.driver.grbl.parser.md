---
title: raygeo.driver.grbl.parser
sidebar_label: raygeo.driver.grbl.parser
---

Pure parsers for GRBL response data: status reports, build info, settings, WCS offsets, probe
results and raw-output classification. All functions are side-effect free.

## Functions

### `alarm_code_to_device_error()`

```python
alarm_code_to_device_error(alarm_code: str) -> types.DeviceError
```

Look up an alarm code in the GRBL alarm table.

| Parameter    | Type                | Description |
| ------------ | ------------------- | ----------- |
| `alarm_code` | `str`               |             |
| _Returns_    | `types.DeviceError` |             |

### `detect_unit_system_from_settings()`

```python
detect_unit_system_from_settings(
    settings_lines: Sequence[str],
) -> Optional[str]
```

Infer the unit system ("metric"/"imperial") from `$$` lines.

| Parameter        | Type            | Description |
| ---------------- | --------------- | ----------- |
| `settings_lines` | `Sequence[str]` |             |
| _Returns_        | `Optional[str]` |             |

### `error_code_to_device_error()`

```python
error_code_to_device_error(error_code: str) -> types.DeviceError
```

Look up an error code in the GRBL error table.

| Parameter    | Type                | Description |
| ------------ | ------------------- | ----------- |
| `error_code` | `str`               |             |
| _Returns_    | `types.DeviceError` |             |

### `extract_device_name()`

```python
extract_device_name(build_info: Sequence[str]) -> str
```

Extract a human-readable device name from build info lines.

| Parameter    | Type            | Description |
| ------------ | --------------- | ----------- |
| `build_info` | `Sequence[str]` |             |
| _Returns_    | `str`           |             |

### `extract_device_name_from_output()`

```python
extract_device_name_from_output(data: Sequence[int]) -> Optional[str]
```

Extract a device name from raw serial output, falling back to the first informative banner line.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| `data`    | `Sequence[int]` |             |
| _Returns_ | `Optional[str]` |             |

### `gcode_to_p_number()`

```python
gcode_to_p_number(wcs_slot: str) -> Optional[int]
```

Convert a G-code WCS name (e.g. "G54") to its P-number.

| Parameter  | Type            | Description |
| ---------- | --------------- | ----------- |
| `wcs_slot` | `str`           |             |
| _Returns_  | `Optional[int]` |             |

### `is_grbl_output()`

```python
is_grbl_output(data: Sequence[int]) -> bool
```

True when raw serial output identifies a Grbl device.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| `data`    | `Sequence[int]` |             |
| _Returns_ | `bool`          |             |

### `is_report_in_inches()`

```python
is_report_in_inches(settings_lines: Sequence[str]) -> bool
```

True when the `$13` (Report in inches) flag is set.

| Parameter        | Type            | Description |
| ---------------- | --------------- | ----------- |
| `settings_lines` | `Sequence[str]` |             |
| _Returns_        | `bool`          |             |

### `parse_board()`

```python
parse_board(line: str) -> Optional[str]
```

Parse a grblHAL `[BOARD:...]` build-info line.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| `line`    | `str`           |             |
| _Returns_ | `Optional[str]` |             |

### `parse_grbl_parser_state()`

```python
parse_grbl_parser_state(response_lines: Sequence[str]) -> Optional[str]
```

Parse `$G` response lines to find the active WCS (G54-G59).

| Parameter        | Type            | Description |
| ---------------- | --------------- | ----------- |
| `response_lines` | `Sequence[str]` |             |
| _Returns_        | `Optional[str]` |             |

### `parse_grbl_settings()`

```python
parse_grbl_settings(lines: Sequence[str]) -> dict[str, float]
```

Parse `$$` lines into `{key: value}` floats.

| Parameter | Type               | Description |
| --------- | ------------------ | ----------- |
| `lines`   | `Sequence[str]`    |             |
| _Returns_ | `dict[str, float]` |             |

### `parse_msg()`

```python
parse_msg(line: str) -> Optional[tuple[str, str]]
```

Parse a `[MSG:key:value]` line into `(key, value)`.

| Parameter | Type                        | Description |
| --------- | --------------------------- | ----------- |
| `line`    | `str`                       |             |
| _Returns_ | `Optional[tuple[str, str]]` |             |

### `parse_opt_flags()`

```python
parse_opt_flags(line: str) -> Optional[str]
```

Extract the compile option flags from an `[OPT:...]` line.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| `line`    | `str`           |             |
| _Returns_ | `Optional[str]` |             |

### `parse_opt_info()`

```python
parse_opt_info(line: str) -> Optional[int]
```

Extract the RX buffer size from an `[OPT:...]` line.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| `line`    | `str`           |             |
| _Returns_ | `Optional[int]` |             |

### `parse_probe_line()`

```python
parse_probe_line(
    line: str,
) -> Optional[tuple[tuple[float, float, float], bool]]
```

Parse a `[PRB:x,y,z:success]` line into `(pos, success)`.

| Parameter | Type                                                | Description |
| --------- | --------------------------------------------------- | ----------- |
| `line`    | `str`                                               |             |
| _Returns_ | `Optional[tuple[tuple[float, float, float], bool]]` |             |

### `parse_setting_pairs()`

```python
parse_setting_pairs(lines: Sequence[str]) -> list[tuple[str, str]]
```

Parse `$$` lines into ordered `(key, raw_value)` pairs.

| Parameter | Type                    | Description |
| --------- | ----------------------- | ----------- |
| `lines`   | `Sequence[str]`         |             |
| _Returns_ | `list[tuple[str, str]]` |             |

### `parse_state()`

```python
parse_state(
    state_str: str,
    default: Optional[types.DeviceState] = None,
    report_in_inches: bool = False,
) -> types.DeviceState
```

Parse a GRBL status string like `<Idle|MPos:10,20,30>` into a DeviceState, using *default* as the
base. When *report_in_inches* is true, positions are converted back to mm.

| Parameter          | Type                                 | Description |
| ------------------ | ------------------------------------ | ----------- |
| `state_str`        | `str`                                |             |
| `default`          | `Optional[types.DeviceState] = None` |             |
| `report_in_inches` | `bool = False`                       |             |
| _Returns_          | `types.DeviceState`                  |             |

### `parse_ver()`

```python
parse_ver(line: str) -> Optional[tuple[str, Optional[str]]]
```

Parse a `[VER:...]` line into `(version, build_name)`.

| Parameter | Type                                  | Description |
| --------- | ------------------------------------- | ----------- |
| `line`    | `str`                                 |             |
| _Returns_ | `Optional[tuple[str, Optional[str]]]` |             |

### `parse_version()`

```python
parse_version(response_lines: Sequence[str]) -> Optional[str]
```

Parse `$I` response lines to extract the firmware version.

| Parameter        | Type            | Description |
| ---------------- | --------------- | ----------- |
| `response_lines` | `Sequence[str]` |             |
| _Returns_        | `Optional[str]` |             |

### `parse_wcs_line()`

```python
parse_wcs_line(line: str) -> Optional[tuple[str, tuple[float, float, float]]]
```

Parse a `[G5x:x,y,z]` line from `$#` output.

| Parameter | Type                                               | Description |
| --------- | -------------------------------------------------- | ----------- |
| `line`    | `str`                                              |             |
| _Returns_ | `Optional[tuple[str, tuple[float, float, float]]]` |             |

### `split_realtime_commands()`

```python
split_realtime_commands(lines: Sequence[str]) -> tuple[list[str], list[str]]
```

Split command lines into `(gcode, realtime)` groups.

| Parameter | Type                          | Description |
| --------- | ----------------------------- | ----------- |
| `lines`   | `Sequence[str]`               |             |
| _Returns_ | `tuple[list[str], list[str]]` |             |

### `strip_gcode_comments()`

```python
strip_gcode_comments(line: str) -> str
```

Strip G-code comments from a line.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| `line`    | `str` |             |
| _Returns_ | `str` |             |

### `version_supports_single_axis_homing()`

```python
version_supports_single_axis_homing(
    version_num: float,
    version_letter: str = '',
) -> bool
```

True if a GRBL version supports single-axis homing (> 1.1 or 1.1g and newer).

| Parameter        | Type       | Description |
| ---------------- | ---------- | ----------- |
| `version_num`    | `float`    |             |
| `version_letter` | `str = ''` |             |
| _Returns_        | `bool`     |             |
