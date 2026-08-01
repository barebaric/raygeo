"""Tests for typed AssemblyWarning infrastructure (Step 1).

These tests cover the warning types introduced in Step 1 of the
multi-face-regions plan:

* ``AssemblyWarningKind`` is importable from ``raygeo.ops.assembly`` and
  exposes the ``FACE_FAILED`` / ``REGION_FAILED`` variants.
* ``AssemblyWarning`` is importable and exposes the
  ``kind`` / ``face_id`` / ``region`` / ``detail`` attributes.
* A successful Compute stage yields an ``AssemblyOutput`` whose
  ``warnings`` list defaults to empty (no warnings are emitted yet;
  Steps 3 and 4 populate it).
"""

from raygeo.ops import Ops
from raygeo.ops.assembly import (
    AssemblyOutput,
    AssemblyWarning,
    AssemblyWarningKind,
)


def test_assembly_warning_kind_importable():
    assert AssemblyWarningKind is not None


def test_assembly_warning_kind_variants():
    assert hasattr(AssemblyWarningKind, "FACE_FAILED")
    assert hasattr(AssemblyWarningKind, "REGION_FAILED")


def test_assembly_warning_kind_value():
    assert AssemblyWarningKind.FACE_FAILED.value == "face_failed"
    assert AssemblyWarningKind.REGION_FAILED.value == "region_failed"


def test_assembly_warning_kind_repr():
    assert (
        repr(AssemblyWarningKind.FACE_FAILED)
        == "AssemblyWarningKind.FACE_FAILED"
    )
    assert (
        repr(AssemblyWarningKind.REGION_FAILED)
        == "AssemblyWarningKind.REGION_FAILED"
    )
    assert str(AssemblyWarningKind.FACE_FAILED) == repr(
        AssemblyWarningKind.FACE_FAILED
    )


def test_assembly_warning_importable():
    assert AssemblyWarning is not None


def test_assembly_warning_has_expected_fields():
    expected = {"kind", "face_id", "region", "detail"}
    actual = {name for name, _ in AssemblyWarning.__dict__.items()}
    missing = expected - actual
    assert not missing, f"AssemblyWarning missing fields: {missing}"


def test_assembly_output_defaults_warnings_empty():
    out = AssemblyOutput(ops=Ops())
    assert out.warnings == []


def test_assembly_output_warnings_is_list():
    out = AssemblyOutput(ops=Ops())
    assert isinstance(out.warnings, list)
