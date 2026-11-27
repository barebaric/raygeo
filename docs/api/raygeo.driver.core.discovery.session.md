---
title: raygeo.driver.core.discovery.session
sidebar_label: raygeo.driver.core.discovery.session
---

## DiscoverySession

Accumulates discovered devices across repeated scans and reports the difference between scans as
plain data:

- `apply_scan` returns `(added, removed_keys)` — devices are matched by their stable
  `DiscoveredDevice.key`.
- Serial ports of found devices (those without a `host` parameter) are remembered as *held*; held
  ports are meant to be excluded from rescans (re-opening them would reset some boards) and their
  devices stay listed even though a rescan cannot see them. They leave only via `prune_absent_ports`
  when the port vanishes from the system.
- `apply_probe` attaches a probed profile to a device and returns the enriched device.

### `devices`

```python
devices: list
```

### `held_ports`

```python
held_ports: set
```

Ports of known devices; exclude from rescans.

### `apply_probe()`

```python
apply_probe(key: str, profile: object) -> Optional[discovery.DiscoveredDevice]
```

Attaches a probed profile to the device with *key* and returns the enriched device, or None when the
key is unknown.

| Parameter | Type                                   | Description |
| --------- | -------------------------------------- | ----------- |
| `key`     | `str`                                  |             |
| `profile` | `object`                               |             |
| _Returns_ | `Optional[discovery.DiscoveredDevice]` |             |

### `apply_scan()`

```python
apply_scan(
    devices: Sequence[discovery.DiscoveredDevice],
) -> tuple[list[discovery.DiscoveredDevice], list[str]]
```

Reconciles the known devices with a fresh scan result. Returns the newly seen devices and the keys
of devices that disappeared (excluding those on held ports).

| Parameter | Type                                                 | Description |
| --------- | ---------------------------------------------------- | ----------- |
| `devices` | `Sequence[discovery.DiscoveredDevice]`               |             |
| _Returns_ | `tuple[list[discovery.DiscoveredDevice], list[str]]` |             |

### `get()`

```python
get(key: str) -> Optional[discovery.DiscoveredDevice]
```

| Parameter | Type                                   | Description |
| --------- | -------------------------------------- | ----------- |
| `key`     | `str`                                  |             |
| _Returns_ | `Optional[discovery.DiscoveredDevice]` |             |

### `prune_absent_ports()`

```python
prune_absent_ports(present_ports: Iterable[str]) -> list[str]
```

Drops devices whose serial port is no longer present on the system. Returns the removed keys.

| Parameter       | Type            | Description |
| --------------- | --------------- | ----------- |
| `present_ports` | `Iterable[str]` |             |
| _Returns_       | `list[str]`     |             |

### `reset()`

```python
reset() -> None
```

Forgets everything (e.g. when the UI re-enters discovery).

| Parameter | Type   | Description |
| --------- | ------ | ----------- |
| _Returns_ | `None` |             |
