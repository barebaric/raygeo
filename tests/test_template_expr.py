import pytest

from raygeo.template import evaluate, evaluate_number, format_value


def test_evaluate_arithmetic():
    assert evaluate_number("1 + 2 * 3", {}) == 7.0


def test_evaluate_variables():
    assert evaluate_number("width * 2", {"width": 5.0}) == 10.0


def test_evaluate_math_functions_and_constants():
    assert evaluate_number("sqrt(16) + sin(pi / 2)", {}) == pytest.approx(5.0)


def test_evaluate_strings():
    assert evaluate("'a' + 'b'", {}) == "ab"


def test_evaluate_comparisons():
    assert evaluate("3 < 4 and 5 >= 5", {}) is True


def test_evaluate_host_function():
    assert evaluate("today()", {}, {"today": lambda: "2026-11-23"}) == (
        "2026-11-23"
    )


def test_evaluate_dotted_host_function_falls_back():
    assert evaluate("date.today()", {}, {"today": lambda: "2026-11-23"}) == (
        "2026-11-23"
    )


def test_evaluate_unknown_name_raises():
    with pytest.raises(Exception):
        evaluate("missing + 1", {})


def test_evaluate_rejects_attribute_access():
    with pytest.raises(Exception):
        evaluate("foo.__class__", {"foo": 1})


def test_format_int_padding():
    assert format_value(7, "03d") == "007"


def test_format_negative_padding():
    assert format_value(-7, "04d") == "-007"


def test_format_float_precision():
    assert format_value(3.14159, ".2f") == "3.14"


def test_format_string_width():
    assert format_value("42", ">8") == "      42"
