---
title: raygeo.driver.marlin.parser
sidebar_label: raygeo.driver.marlin.parser
---

Pure parsers for Marlin response data: ok/error classification, boot banners, M114 positions, M115
firmware info and M149 units. All functions are side-effect free.

## Functions

### `detect_unit_system_from_m149()`

```python
detect_unit_system_from_m149(lines: Sequence[str]) -> Optional[str]
```

Infer the device unit system from M149 response lines ("metric"/"imperial"), or None.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| `lines`   | `Sequence[str]` |             |
| _Returns_ | `Optional[str]` |             |

### `extract_marlin_banner_from_output()`

```python
extract_marlin_banner_from_output(data: Sequence[int]) -> Optional[str]
```

Extract the first boot message line from raw serial output, or None.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| `data`    | `Sequence[int]` |             |
| _Returns_ | `Optional[str]` |             |

### `extract_marlin_device_name()`

```python
extract_marlin_device_name(
    m115_lines: Sequence[str],
    boot_lines: Optional[Sequence[str]],
) -> str
```

Extract a human-readable device name from M115 and boot output.

| Parameter    | Type                      | Description |
| ------------ | ------------------------- | ----------- |
| `m115_lines` | `Sequence[str]`           |             |
| `boot_lines` | `Optional[Sequence[str]]` |             |
| _Returns_    | `str`                     |             |

### `is_boot_message()`

```python
is_boot_message(line: str) -> bool
```

True when *line* is a Marlin boot/startup message.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `line`    | `str`  |             |
| _Returns_ | `bool` |             |

### `is_error_response()`

```python
is_error_response(line: str) -> bool
```

True when *line* is a Marlin error response (`Error:...`).

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `line`    | `str`  |             |
| _Returns_ | `bool` |             |

### `is_m114_position()`

```python
is_m114_position(line: str) -> bool
```

True when *line* carries an M114 position report.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `line`    | `str`  |             |
| _Returns_ | `bool` |             |

### `is_marlin_output()`

```python
is_marlin_output(data: Sequence[int]) -> bool
```

True when raw serial output identifies a Marlin device.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| `data`    | `Sequence[int]` |             |
| _Returns_ | `bool`          |             |

### `is_ok_response()`

```python
is_ok_response(line: str) -> bool
```

True when *line* is a Marlin `ok` acknowledgement (plain or with temperature payload).

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| `line`    | `str`  |             |
| _Returns_ | `bool` |             |

### `parse_error_message()`

```python
parse_error_message(line: str) -> str
```

Extract the message portion of an `Error:...` line.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| `line`    | `str` |             |
| _Returns_ | `str` |             |

### `parse_m114_line()`

```python
parse_m114_line(line: str) -> Optional[tuple[float, float, float]]
```

Parse a single M114 position line, or None.

| Parameter | Type                                   | Description |
| --------- | -------------------------------------- | ----------- |
| `line`    | `str`                                  |             |
| _Returns_ | `Optional[tuple[float, float, float]]` |             |

### `parse_m114_position()`

```python
parse_m114_position(
    lines: Sequence[str],
) -> Optional[tuple[float, float, float]]
```

Parse M114 output lines into an (x, y, z) position, or None.

| Parameter | Type                                   | Description |
| --------- | -------------------------------------- | ----------- |
| `lines`   | `Sequence[str]`                        |             |
| _Returns_ | `Optional[tuple[float, float, float]]` |             |

### `parse_m115_firmware_info()`

```python
parse_m115_firmware_info(lines: Sequence[str]) -> dict[str, str]
```

Parse M115 response lines into firmware name and machine type.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `lines`   | `Sequence[str]`  |             |
| _Returns_ | `dict[str, str]` |             |

### `parse_m211_endstops()`

```python
parse_m211_endstops(lines: Sequence[str]) -> Optional[tuple[float, float]]
```

Parse M211 output into (x_max, y_max) software endstop travel, or None.

| Parameter | Type                            | Description |
| --------- | ------------------------------- | ----------- |
| `lines`   | `Sequence[str]`                 |             |
| _Returns_ | `Optional[tuple[float, float]]` |             |

### `parse_m503_settings()`

```python
parse_m503_settings(lines: Sequence[str]) -> dict[str, float]
```

Parse M503 output into key motion settings.

| Parameter | Type               | Description |
| --------- | ------------------ | ----------- |
| `lines`   | `Sequence[str]`    |             |
| _Returns_ | `dict[str, float]` |             |

### `parse_marlin_version()`

```python
parse_marlin_version(line: str) -> Optional[str]
```

Extract the Marlin version from a boot line like `Marlin 2.1.2.7`, or None.

| Parameter | Type            | Description |
| --------- | --------------- | ----------- |
| `line`    | `str`           |             |
| _Returns_ | `Optional[str]` |             |
