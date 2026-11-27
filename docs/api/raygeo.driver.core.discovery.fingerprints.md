---
title: raygeo.driver.core.discovery.fingerprints
sidebar_label: raygeo.driver.core.discovery.fingerprints
---

## HttpFingerprint

Confirms a generic HTTP service by matching the response body.

The probe GETs *path* and reports a match when the endpoint answers `200` with a body matching
*match_regex* (matched case-insensitively), bounded by *timeout* seconds and *max_bytes* of body. It
resolves to `True` on a match, and `False` on any failure.

### `match_regex`

```python
match_regex: str
```

### `max_bytes`

```python
max_bytes: int
```

### `path`

```python
path: str
```

### `timeout`

```python
timeout: float
```

## TcpFingerprint

Confirms a generic TCP service by matching a response line.

The probe waits *banner_wait* seconds for an unsolicited banner, then (when *probe_command* is set)
sends it and waits for the remaining *timeout* seconds. It resolves to whether a line starts with
*match_prefix*: `True` claims the candidate, `False` does not.

### `banner_wait`

```python
banner_wait: float
```

### `match_prefix`

```python
match_prefix: str
```

### `probe_command`

```python
probe_command: Optional[str]
```

### `timeout`

```python
timeout: float
```
