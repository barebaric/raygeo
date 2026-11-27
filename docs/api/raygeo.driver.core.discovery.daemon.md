---
title: raygeo.driver.core.discovery.daemon
sidebar_label: raygeo.driver.core.discovery.daemon
---

## Functions

### `list_cameras()`

```python
list_cameras(
    host: str,
    port: int,
    token: Optional[str] = None,
    tls_ca: Optional[str] = None,
    tls_hostname: Optional[str] = None,
    timeout: float = 5.0,
) -> Awaitable[list[dict[str, object]]]
```

Lists the cameras a daemon advertises on the device host.

Each entry is a dict with `name`, `uri` and `type` (e.g. `http_stream`). Video never crosses the
control link: clients open *uri* with their own network-camera source.

*token* authenticates against a protected daemon and is only sent over TLS, except to loopback. TLS
is enabled when *tls_ca* names a PEM certificate; *tls_hostname* overrides the verified name.

| Parameter      | Type                                 | Description |
| -------------- | ------------------------------------ | ----------- |
| `host`         | `str`                                |             |
| `port`         | `int`                                |             |
| `token`        | `Optional[str] = None`               |             |
| `tls_ca`       | `Optional[str] = None`               |             |
| `tls_hostname` | `Optional[str] = None`               |             |
| `timeout`      | `float = 5.0`                        |             |
| _Returns_      | `Awaitable[list[dict[str, object]]]` |             |
