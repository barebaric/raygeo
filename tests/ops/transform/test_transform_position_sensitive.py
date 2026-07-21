"""Tests for the position_sensitive flag on transformer specs (R4)."""

from raygeo.ops.transform import is_position_sensitive
from raygeo.ops.transform.clip import CropSpec
from raygeo.ops.transform.merge_lines import MergeLinesSpec
from raygeo.ops.transform.optimize import OptimizeSpec
from raygeo.ops.transform.overscan import OverscanSpec
from raygeo.ops.transform.smooth import SmoothSpec

_REGIONS = [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]]


def test_crop_is_position_sensitive():
    assert is_position_sensitive(CropSpec(0.01, 0.0, _REGIONS))


def test_smooth_is_not_position_sensitive():
    assert not is_position_sensitive(SmoothSpec(50, 30.0))


def test_overscan_is_not_position_sensitive():
    assert not is_position_sensitive(OverscanSpec(2.0))


def test_optimize_is_not_position_sensitive():
    assert not is_position_sensitive(OptimizeSpec(True, True, []))


def test_merge_lines_is_not_position_sensitive():
    assert not is_position_sensitive(MergeLinesSpec(0.1))


def test_unknown_object_returns_false():
    assert not is_position_sensitive(42)
    assert not is_position_sensitive("not a spec")
