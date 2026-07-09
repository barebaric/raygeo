"""Tests for the CLI trace subcommand (new span/event format).

These exercise ``raygeo trace`` + ``raygeo print`` end to end.
"""

import pathlib
import subprocess
import sys


def _raygeo(args, cwd):
    cmd = [sys.executable, "-m", "raygeo.cli.main"] + args
    proc = subprocess.run(
        cmd, capture_output=True, text=True, cwd=cwd, timeout=60
    )
    return proc.returncode, proc.stdout, proc.stderr


def _trace_profile_outer(tmp_path, tp):
    return _raygeo(
        [
            "trace",
            tp,
            "--mode",
            "profile",
            "--outer",
            "--tool-radius",
            "3",
            "--scenario",
            "default",
        ],
        tmp_path,
    )


def test_cli_raygeo_trace_profile_outer_generates_valid_bin(tmp_path):
    """`raygeo trace --mode profile --outer` produces a valid v3 .bin file."""
    tp = str(tmp_path / "prof_outer.bin")
    rc, out, err = _trace_profile_outer(tmp_path, tp)
    assert rc == 0, f"stderr: {err}"
    assert pathlib.Path(tp).exists()

    from raygeo.trace import TraceFile

    trace = TraceFile(tp)
    assert trace.ver == 3
    assert trace.events
    assert "profile" in trace.sources


def test_cli_raygeo_trace_profile_inner_generates_valid_bin(tmp_path):
    """`raygeo trace --mode profile --inner` produces a valid v3 .bin file."""
    tp = str(tmp_path / "prof_inner.bin")
    rc, out, err = _raygeo(
        [
            "trace",
            tp,
            "--mode",
            "profile",
            "--inner",
            "--tool-radius",
            "3",
            "--scenario",
            "default",
        ],
        tmp_path,
    )
    assert rc == 0, f"stderr: {err}"
    assert pathlib.Path(tp).exists()

    from raygeo.trace import TraceFile

    trace = TraceFile(tp)
    assert trace.ver == 3
    assert "profile" in trace.sources


def test_cli_raygeo_print_handles_profile_trace(tmp_path):
    """`raygeo print` on a profile trace prints span/event summary lines."""
    tp = str(tmp_path / "prof_print.bin")
    rc, _, err = _trace_profile_outer(tmp_path, tp)
    assert rc == 0

    rc, out, err = _raygeo(["print", tp], tmp_path)
    assert rc == 0, f"stderr: {err}"
    # The new print output lists spans and events; a profile span should
    # be present and at least one move line.
    assert "profile" in out
    assert "move" in out


def test_cli_raygeo_unknown_profile_mode_errors_tersely(tmp_path):
    """Unrecognised --mode value produces an error."""
    tp = str(tmp_path / "bad.bin")
    rc, out, err = _raygeo(
        [
            "trace",
            tp,
            "--mode",
            "bogus",
            "--outer",
            "--tool-radius",
            "3",
            "--scenario",
            "default",
        ],
        tmp_path,
    )
    assert rc != 0
    assert "bogus" in err or "invalid choice" in err
