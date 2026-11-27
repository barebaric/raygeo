---
title: raygeo.driver.core.discovery.serial_channel
sidebar_label: raygeo.driver.core.discovery.serial_channel
---

## Constants

### CORPORATE_TOKENS

`set[str]`

### GENERIC_DEVICE_TOKENS

`set[str]`

### GENERIC_TOKENS

`set[str]`

### SERIAL_SCAN_TIMEOUT

`float`

### USB_CHIP_TOKENS

`set[str]`

### scan_serial_ports

`Callable[..., Awaitable[list[serial_scan.PortObservation]]]`

## Functions

### `build_identity()`

```python
build_identity(
    firmware: Optional[str],
    port_info: Optional[ports.SerialPortInfo],
    device_name: Optional[str] = None,
) -> discovery.DeviceIdentity
```

Assembles a `DeviceIdentity` from serial scan artifacts. A *device_name* extracted from the device's
own output (when the recognizer provides one) is the identity banner and contributes matching
tokens.

| Parameter     | Type                             | Description |
| ------------- | -------------------------------- | ----------- |
| `firmware`    | `Optional[str]`                  |             |
| `port_info`   | `Optional[ports.SerialPortInfo]` |             |
| `device_name` | `Optional[str] = None`           |             |
| _Returns_     | `discovery.DeviceIdentity`       |             |

### `collect_serial_recognizers()`

```python
collect_serial_recognizers(
    driver_classes: Sequence[type],
) -> list[tuple[type, spec.SerialRecognizer]]
```

Pairs every driver whose `DISCOVERY` spec declares a valid serial recognizer with it.

| Parameter        | Type                                       | Description |
| ---------------- | ------------------------------------------ | ----------- |
| `driver_classes` | `Sequence[type]`                           |             |
| _Returns_        | `list[tuple[type, spec.SerialRecognizer]]` |             |

### `find_serial_devices()`

```python
find_serial_devices(
    driver_classes: Sequence[type],
    ports: Optional[Iterable[str]] = None,
    baud_rates: Optional[Iterable[int]] = None,
    exclude_ports: Optional[Iterable[str]] = None,
) -> Awaitable[list[discovery.DiscoveredDevice]]
```

Runs one serial scan and evaluates it against every declared serial recognizer. Like every channel,
this never raises: failures are logged, and the scan is bounded by *SERIAL_SCAN_TIMEOUT*, so a slow
or silent adapter can at most cost its own probe time.

| Parameter        | Type                                          | Description |
| ---------------- | --------------------------------------------- | ----------- |
| `driver_classes` | `Sequence[type]`                              |             |
| `ports`          | `Optional[Iterable[str]] = None`              |             |
| `baud_rates`     | `Optional[Iterable[int]] = None`              |             |
| `exclude_ports`  | `Optional[Iterable[str]] = None`              |             |
| _Returns_        | `Awaitable[list[discovery.DiscoveredDevice]]` |             |

### `normalize_tokens()`

```python
normalize_tokens(*texts: Optional[str]) -> frozenset[str]
```

Splits arbitrary identification strings (descriptions, by-id link names, manufacturer strings) into
lowercase word tokens suitable for containment checks.

| Parameter | Type             | Description |
| --------- | ---------------- | ----------- |
| `*texts`  | `Optional[str]`  |             |
| _Returns_ | `frozenset[str]` |             |
