
def __getattr__(name):
    import raygeo.raygeo as _raygeo
    return getattr(_raygeo.ops, name)
