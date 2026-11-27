---
title: raygeo.driver.core.discovery.spec
sidebar_label: raygeo.driver.core.discovery.spec
---

## DiscoverySpec

Everything a driver declares about discoverability. Declared as the `DISCOVERY` class attribute;
evaluated by the discovery channels.

### `mdns`

```python
mdns: Optional[MdnsRecognizer]
```

### `serial`

```python
serial: Optional[SerialRecognizer]
```

### `tcp`

```python
tcp: Optional[TcpRecognizer]
```

## MdnsRecognizer

Declarative description of how a driver's devices appear on the network. Evaluated against resolved
mDNS services by the network channel.

### `fingerprint`

```python
fingerprint: Optional[Callable[..., Awaitable[bool]]]
```

### `fingerprint_services`

```python
fingerprint_services: tuple
```

### `services`

```python
services: tuple
```

### `txt_map`

```python
txt_map: dict
```

## SerialRecognizer

Declarative description of what a driver's device looks like on a serial wire. Evaluated against
captured port output by the serial channel.

### `firmware`

```python
firmware: Optional[str]
```

### `label`

```python
label: Callable[[], str]
```

### `matches`

```python
matches: Callable[[bytes], bool]
```

### `name`

```python
name: Optional[Callable[[bytes], Optional[str]]]
```

## TcpRecognizer

Declarative description of how a driver's devices appear as plain TCP listeners on the local network
— e.g. grblHAL's raw telnet service on boards whose firmware does not announce it via mDNS. The
subnet scan probes reachable hosts on the declared port with this data.

### `match_prefix`

```python
match_prefix: Optional[str]
```

### `port`

```python
port: int
```

### `probe_command`

```python
probe_command: Optional[str]
```
