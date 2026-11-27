"""Exceptions shared by the analyzer and the remote daemon."""


class InsecureListenerError(Exception):
    """Raised when a listener configuration violates the bind policy."""
