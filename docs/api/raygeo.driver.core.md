---
title: raygeo.driver.core
sidebar_label: raygeo.driver.core
---

Shared device primitives: transports, value types and session events. The protocol namespaces
(raygeo.driver.grbl, raygeo.driver.smoothie, raygeo.driver.marlin) build on these.

## DriverEvent

One typed session event, delivered to `event_callback`.

### CommandStatusChanged

### ConfigChanged

### ConnectionStatusChanged

### JobError

### JobFinished

### ProbeStatusChanged

### RawRx

### RawRxBytes

### RawTx

### RawTxBytes

### StateChanged

### WcsUpdated

## Functions

### `list_serial_ports()`

```python
list_serial_ports() -> list[dict]
```

Enumerate the serial ports this host can reach.

Returns one dict per port with the keys `port` (the OS device path), `vid`, `pid`, `serial_number`,
`manufacturer` and `product`. The USB fields are `None` for non-USB ports.

| Parameter | Type         | Description |
| --------- | ------------ | ----------- |
| _Returns_ | `list[dict]` |             |
