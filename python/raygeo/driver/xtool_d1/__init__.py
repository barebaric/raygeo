import raygeo.raygeo as _raygeo  # type: ignore[import-untyped]


def __getattr__(name):
    return getattr(_raygeo.driver.xtool_d1, name)
