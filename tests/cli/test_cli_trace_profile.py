"""Tests for CLI profile-tracing subcommand (--mode profile)."""

import pathlib
import subprocess
import sys

import pytest


def _raygeo(args, cwd):
    """Run `raygeo <args...>` and return (returncode, stdout, stderr)."""
    cmd = [sys.executable, "-m", "raygeo.cli.main"] + args
    proc = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        cwd=cwd,
        timeout=60,
    )
    return proc.returncode, proc.stdout, proc.stderr


def test_cli_raygeo_trace_profile_outer_generates_valid_bin(tmp_path):
    """`raygeo trace --mode profile --outer` produces a valid .bin file."""
    tp = str(tmp_path / "prof_outer.bin")
    rc, out, err = _raygeo(
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
    assert rc == 0, f"stderr: {err}"
    assert pathlib.Path(tp).exists(), f"trace file not created: {tp}"
    from raygeo.trace import TraceFile

    trace = TraceFile(tp)
    assert len(trace) > 0
    geo = trace.geometry
    assert "offset_polys" in geo
    assert "walk_order" in geo


def test_cli_raygeo_trace_profile_inner_generates_valid_bin(tmp_path):
    """`raygeo trace --mode profile --inner` produces a valid .bin file."""
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
    assert len(trace) > 0
    geo = trace.geometry
    assert "offset_polys" in geo


def test_cli_raygeo_print_handles_profile_trace(tmp_path):
    """`raygeo print` on a profile trace prints profile-specific fields."""
    tp = str(tmp_path / "prof_print.bin")
    rc, out, _ = _raygeo(
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
    assert rc == 0

    rc, out, err = _raygeo(["print", tp], tmp_path)
    assert rc == 0, f"stderr: {err}"
    # Profile-specific fields should appear
    assert "offset_polys" in out
    assert "walk_order" in out
    # Individual record lines should show profile fields
    for line in out.splitlines():
        if "cut" in line and "\ttarget=" in line:
            break
    else:
        pytest.fail("no profile cut line with target= found in print output")


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
