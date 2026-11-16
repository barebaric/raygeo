"""Document model layer for the sketcher.

The entity and constraint classes are implemented in Rust and
imported from :mod:`raygeo.sketch.entities` and
:mod:`raygeo.sketch.constraints`; this package carries the
GTK-free Python contract surface (titles, applicability, expression
handling, serialization hooks) as thin subclasses of the compiled
classes.
"""
