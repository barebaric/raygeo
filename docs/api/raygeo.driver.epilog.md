---
title: raygeo.driver.epilog
sidebar_label: raygeo.driver.epilog
---

Epilog laser support implemented in Rust. EpilogDriver uploads pre-encoded print jobs (the encoder
decides the printer language) over the BSD LPD protocol (port 515) and reports reachability. The
machine exposes no status channel, so every other operation raises NotImplementedError.

## EpilogDriver

An Epilog laser session: prints jobs through the machine's LPD print queue.

All async methods return asyncio futures; a typed `DriverEvent` is delivered to
`event_callback(event)` on the event loop thread. The machine has no status channel, so most session
operations raise `NotImplementedError`; job submission uses `upload`.

### `probe()`

```python
probe() -> Awaitable[None]
```

Probe the LPD port without submitting anything.

| Parameter | Type              | Description |
| --------- | ----------------- | ----------- |
| _Returns_ | `Awaitable[None]` |             |

### `upload()`

```python
upload(payload: Union[bytes, bytearray], title: str) -> Awaitable[None]
```

Upload a complete print job and wait until the machine accepts it.

| Parameter | Type                      | Description                                   |
| --------- | ------------------------- | --------------------------------------------- |
| `payload` | `Union[bytes, bytearray]` | the encoder's byte stream for this job.       |
| `title`   | `str`                     | the job name shown in the machine's job list. |
| _Returns_ | `Awaitable[None]`         |                                               |
