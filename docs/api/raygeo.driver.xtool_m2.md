---
title: raygeo.driver.xtool_m2
sidebar_label: raygeo.driver.xtool_m2
---

xTool M2 WS-V2 driver. The session supports monitoring, motion and job control for jobs already
staged on the machine; G-code upload is not implemented.

## XtoolM2Driver

Monitor and control an xTool M2 over its WS-V2 TLS service. This session does not upload G-code
jobs. `start` controls a job that is already staged on the device, such as a job sent by xTool
Studio.

### `is_job_running`

```python
is_job_running: bool
```

Whether the state currently indicates an active or held job.

### `start()`

```python
start() -> Awaitable[None]
```

Start a job already staged on the device.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `Awaitable[None]` |             |
