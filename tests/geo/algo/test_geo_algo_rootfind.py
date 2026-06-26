"""Tests for rootfind module."""

import math

from raygeo.geo.algo.rootfind import bisect, bracket_grid, illinois, secant


def _sq_err(x):
    return x * x - 2


def _sq_err_plus(x):
    return x * x + 1


def _lin(x):
    return 2 * x - 5


def test_bisect_sqrt_2():
    root, status, iters = bisect(_sq_err, 0.0, 2.0, 1e-10, 100)
    assert status == "Converged"
    assert abs(root - math.sqrt(2)) < 1e-8
    assert iters > 0


def test_bisect_no_bracket():
    _, status, _ = bisect(_sq_err_plus, 0.0, 2.0, 1e-10, 100)
    assert status == "NoBracket"


def test_secant_sqrt_2():
    root, status, iters = secant(_sq_err, 1.0, 2.0, 1e-10, 100)
    assert status == "Converged"
    assert abs(root - math.sqrt(2)) < 1e-8
    assert iters > 0


def test_illinois_sqrt_2():
    root, status, iters = illinois(_sq_err, 0.0, 2.0, 1e-10, 100)
    assert status == "Converged"
    assert abs(root - math.sqrt(2)) < 1e-8
    assert iters > 0


def test_illinois_linear():
    root, status, _ = illinois(_lin, 0.0, 10.0, 1e-10, 100)
    assert status == "Converged"
    assert abs(root - 2.5) < 1e-8


def test_all_methods_agree():
    rb, sb, _ = bisect(_sq_err, 0.0, 2.0, 1e-8, 200)
    rs, ss, _ = secant(_sq_err, 1.0, 2.0, 1e-8, 200)
    ri, si, _ = illinois(_sq_err, 0.0, 2.0, 1e-8, 200)
    assert sb == "Converged"
    assert ss == "Converged"
    assert si == "Converged"
    assert abs(rb - rs) < 1e-6
    assert abs(rb - ri) < 1e-6


def test_secant_fewer_iters():
    _, _, ib = bisect(_sq_err, 0.0, 2.0, 1e-8, 200)
    _, _, isec = secant(_sq_err, 1.0, 2.0, 1e-8, 200)
    assert isec <= ib, "secant should converge in fewer iters"


# ── bracket_grid ──────────────────────────────────────────────────────


def _cubic_err(x):
    return x**3 - 2 * x - 5


def test_bracket_grid_finds_root():
    root, status, iters = bracket_grid(_sq_err, 1.5, 1.0)
    assert status == "Converged"
    assert abs(root - math.sqrt(2)) < 0.5
    assert iters == 7


def test_bracket_grid_converged_status():
    root, status, _ = bracket_grid(
        lambda x: math.sin(x - 3.0), heading=3.0, max_deflection=1.5
    )
    assert status == "Converged"
    assert abs(root - 3.0) < 0.5


def test_bracket_grid_no_bracket():
    root, status, iters = bracket_grid(
        lambda x: x * x + 1.0, heading=0.0, max_deflection=1.0
    )
    assert status == "NoBracket"
    assert iters == 7


def test_bracket_grid_returns_seven_samples():
    _, _, iters = bracket_grid(_sq_err, 1.0, 0.5)
    assert iters == 7


def test_bracket_grid_near_heading():
    root, status, _ = bracket_grid(
        lambda x: x - 2.0, heading=2.0, max_deflection=0.5
    )
    assert status == "Converged"
    assert abs(root - 2.0) < 1e-6


def test_bracket_grid_interpolates_at_boundary():
    """f crosses zero between the -0.2 and 0.0 ratio samples."""
    root, status, _ = bracket_grid(
        lambda x: x - 2.1, heading=2.0, max_deflection=1.0
    )
    assert status == "Converged"
    assert abs(root - 2.1) < 1e-6


def test_bracket_grid_falls_back_on_monotonic():
    root, status, iters = bracket_grid(
        lambda x: abs(x - 5.0) + 0.5, heading=5.0, max_deflection=2.0
    )
    assert status == "NoBracket"
    assert iters == 7
    assert abs(root - 5.0) < 2.5
