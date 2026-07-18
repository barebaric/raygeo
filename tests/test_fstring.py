"""Tests for the Rust fstring module (raygeo.fstring).

Replaces the inline Rust unit tests that were removed from fstring.rs.
"""

from raygeo.fstring import (
    parse_include_directive,
    render_named,
    resolve_path_vars,
)

# ── render_named ─────────────────────────────────────────────────


def test_render_named_basic():
    assert render_named("hello {x}", {"x": "world"}) == "hello world"


def test_render_named_multiple_vars():
    result = render_named("{a} and {b}", {"a": "X", "b": "Y"})
    assert result == "X and Y"


def test_render_named_no_placeholders():
    assert render_named("plain text", {}) == "plain text"


def test_render_named_unknown_left_verbatim():
    assert render_named("{unknown}", {}) == "{unknown}"


def test_render_named_path_style_deferred():
    assert render_named("{machine.name}", {}) == "{machine.name}"


def test_render_named_format_spec_float():
    result = render_named("{val:.3f}", {"val": "3.14159"})
    assert result == "3.142"


def test_render_named_format_spec_integer():
    result = render_named("{val:d}", {"val": "7.9"})
    assert result == "7"


def test_render_named_format_spec_general():
    result = render_named("{val:.2}", {"val": "3.14159"})
    assert result == "3.14"


def test_render_named_format_spec_non_numeric():
    result = render_named("{val:.3f}", {"val": "abc"})
    assert result == "abc"


def test_render_named_empty_value():
    assert render_named("a{x}b", {"x": ""}) == "ab"


def test_render_named_mixed_path_and_named():
    result = render_named("{x_cmd} {machine.name}", {"x_cmd": "X10"})
    assert result == "X10 {machine.name}"


# ── resolve_path_vars ────────────────────────────────────────────


def test_resolve_path_vars_basic():
    assert (
        resolve_path_vars("{machine.name}", {"machine.name": "Laser"})
        == "Laser"
    )


def test_resolve_path_vars_indexed():
    assert (
        resolve_path_vars("{wcs_offset[2]}", {"wcs_offset[2]": "5.0"}) == "5.0"
    )


def test_resolve_path_vars_unknown_left_verbatim():
    assert resolve_path_vars("{unknown}", {}) == "{unknown}"


def test_resolve_path_vars_named_not_resolved():
    assert resolve_path_vars("{x_cmd}", {}) == "{x_cmd}"


def test_resolve_path_vars_multiple():
    result = resolve_path_vars(
        "{a.x} {b[0]}",
        {"a.x": "1", "b[0]": "2"},
    )
    assert result == "1 2"


def test_resolve_path_vars_no_braces():
    assert resolve_path_vars("plain", {}) == "plain"


def test_resolve_path_vars_empty_dict():
    assert resolve_path_vars("{machine.x}", {}) == "{machine.x}"


# ── parse_include_directive ──────────────────────────────────────


def test_parse_include_directive_basic():
    assert parse_include_directive("@include(MyMacro)") == "MyMacro"


def test_parse_include_directive_with_whitespace():
    assert parse_include_directive("  @include( MyMacro )  ") == "MyMacro"


def test_parse_include_directive_not_an_include():
    assert parse_include_directive("G1 X10") is None


def test_parse_include_directive_empty_macro_name():
    assert parse_include_directive("@include()") == ""


def test_parse_include_directive_nested_parens():
    assert parse_include_directive("@include(foo(bar))") == "foo(bar)"
