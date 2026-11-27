"""Remote sessions: a device-host daemon and a client proxy.

The public API is implemented in Rust and exposed through the
compiled extension module; this package only re-exports its
submodules for stable import paths.
"""

import raygeo.raygeo as _raygeo  # type: ignore[import-untyped]

client = _raygeo.driver.remote.client
daemon = _raygeo.driver.remote.daemon
inspect = _raygeo.driver.remote.inspect
mdns = _raygeo.driver.remote.mdns
protocol = _raygeo.driver.remote.protocol

__all__ = ["client", "daemon", "inspect", "mdns", "protocol"]
