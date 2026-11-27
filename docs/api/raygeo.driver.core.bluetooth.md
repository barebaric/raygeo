---
title: raygeo.driver.core.bluetooth
sidebar_label: raygeo.driver.core.bluetooth
---

## BluetoothDeviceInfo

A paired Bluetooth device offered for a Serial Port Profile connection.

### `address`

```python
address: str
```

### `channel`

```python
channel: Optional[int]
```

### `name`

```python
name: Optional[str]
```

## Functions

### `is_supported()`

```python
is_supported() -> bool
```

Whether this build has a Bluetooth SPP backend.

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `bool` |             |

### `list_paired_devices()`

```python
list_paired_devices() -> list[BluetoothDeviceInfo]
```

Enumerate paired Bluetooth devices that can carry SPP/RFCOMM.

Backends that cannot query service records return every paired device. Enumeration failures (no
adapter, sandbox denial) yield an empty list.

| Parameter | Type                        | Description |
| --------- | --------------------------- | ----------- |
| _Returns_ | `list[BluetoothDeviceInfo]` |             |

### `normalize_address()`

```python
normalize_address(address: str) -> str
```

Normalizes a Bluetooth address to uppercase colon-separated form.

| Parameter | Type  | Description |
| --------- | ----- | ----------- |
| `address` | `str` |             |
| _Returns_ | `str` |             |
