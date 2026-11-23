from raygeo.template import (
    names_in_expression,
    placeholder_names,
    referenced_names,
)


def test_names_in_simple_expression():
    assert names_in_expression("width * 2") == {"width"}


def test_names_in_expression_with_function():
    assert names_in_expression("sqrt(width)") == {"sqrt", "width"}


def test_names_in_expression_skips_string_literals():
    assert names_in_expression("'width' + height") == {"height"}


def test_names_in_expression_skips_keywords():
    assert names_in_expression("a if b else c") == {"a", "b", "c"}


def test_names_in_expression_skips_attributes():
    assert names_in_expression("date.today()") == {"date"}


def test_referenced_names_ignores_format_spec():
    assert referenced_names("No. {counter:04d}") == {"counter"}


def test_referenced_names_collects_multiple():
    names = referenced_names("{width} x {height} {counter}")
    assert names == {"counter", "height", "width"}


def test_referenced_names_plain_text():
    assert referenced_names("Hello") == set()


def test_placeholder_names_include_dotted_paths():
    names = placeholder_names("{workpiece.counter:03d}")
    assert names == {"workpiece.counter"}


def test_placeholder_names_multiple():
    names = placeholder_names("{counter} {job.counter}")
    assert names == {"counter", "job.counter"}
