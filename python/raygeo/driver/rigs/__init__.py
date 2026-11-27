"""Hardware-free device rigs for tests and development.

The device side of a serial or UDP link, without hardware: a rig
plays the *device* while the session under test connects over a
real transport — the PTY rig exposes a genuine tty device path, so
Rust serial code sees a port like any other. The firmware emulators
(:mod:`raygeo.driver.grbl.emulator` and friends) speak protocols; these
rigs are the dumber layer beneath them when only a byte pipe or a
packet sink is needed.
"""
