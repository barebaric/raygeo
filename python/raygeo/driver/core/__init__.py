"""Shared device primitives: transports, value types and session
events.

The protocol namespaces (``raygeo.driver.grbl``, ``raygeo.driver.smoothie``,
``raygeo.driver.marlin``, ``raygeo.driver.esp3d``, ``raygeo.driver.octoprint``,
``raygeo.driver.xtool_d1``) build on these.
"""

import raygeo.raygeo as _raygeo  # type: ignore[import-untyped]


def __getattr__(name):
    return getattr(_raygeo.driver.core, name)
