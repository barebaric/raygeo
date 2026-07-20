import raygeo.raygeo as _raygeo


def __getattr__(name):
    return getattr(_raygeo.pipeline, name)
