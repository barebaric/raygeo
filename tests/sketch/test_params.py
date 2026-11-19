import math

import pytest

from raygeo.sketch.params import ParameterContext


@pytest.fixture
def params():
    return ParameterContext()


def test_set_get_simple(params):
    params.set("width", 100)
    assert params.get("width") == 100.0


def test_expression_evaluation(params):
    params.set("a", 10)
    params.set("b", 20)
    params.set("c", "a + b")
    assert params.get("c") == 30.0


def test_math_functions(params):
    params.set("x", "sqrt(16)")
    params.set("y", "pi")
    assert params.get("x") == 4.0
    assert params.get("y") == pytest.approx(math.pi)


def test_power_forms(params):
    params.set("a", "2 ** 3")
    params.set("b", "2 ^ 3")
    assert params.get("a") == 8.0
    assert params.get("b") == 8.0


def test_dependency_resolution_order(params):
    params.set("b", "a * 2")
    params.set("a", 10)
    assert params.get("b") == 20.0


def test_chained_dependencies(params):
    params.set("val1", 10)
    params.set("val2", "val1 + 5")
    params.set("val3", "val2 * 2")
    assert params.get("val3") == 30.0


def test_dirty_flag_logic(params):
    params.set("x", 10)
    assert params.get("x") == 10.0
    params.set("x", 20)
    params.set("y", "x + 5")
    assert params.get("y") == 25.0


def test_missing_dependency_safe_fail(params):
    params.set("z", "non_existent + 5")
    assert params.get("z") == 0.0
    assert "z" in params.unresolved_errors()


def test_evaluate_arbitrary_string(params):
    params.set("w", 50)
    assert params.evaluate("w / 2") == 25.0


def test_circular_dependency_protection(params):
    params.set("a", "b")
    params.set("b", "a")
    assert params.get("a") == 0.0
    assert params.get("b") == 0.0
    assert set(params.unresolved_errors()) == {"a", "b"}


def test_parameter_syntax_error(params):
    params.set("bad", "sqrt(")
    assert params.get("bad") == 0.0


def test_parameter_overwrite(params):
    params.set("base", 10)
    params.set("res", "base * 2")
    assert params.get("res") == 20.0
    params.set("base", 5)
    assert params.get("res") == 10.0


def test_parameter_context_serialization_round_trip(params):
    params.set("width", 100)
    params.set("height", "width / 2")
    params.set("depth", "sqrt(width)")

    data = params.to_dict()
    assert data == {
        "expressions": {
            "width": "100",
            "height": "width / 2",
            "depth": "sqrt(width)",
        }
    }

    new_params = ParameterContext.from_dict(data)
    assert new_params.get("width") == 100.0
    assert new_params.get("height") == 50.0
    assert new_params.get("depth") == 10.0


def test_get_all_values(params):
    params.set("a", 10)
    params.set("b", "a * 2")
    params.set("c", "sqrt(a + 6)")

    expected = {"a": 10.0, "b": 20.0, "c": 4.0}
    result = params.get_all_values()
    assert result == expected

    result["a"] = 999
    assert params.get("a") == 10.0


def test_get_with_default(params):
    params.set("exists", 42)
    assert params.get("does_not_exist") == 0.0
    assert params.get("does_not_exist_either", default=-1.0) == -1.0
    assert params.get("exists", default=99.0) == 42.0


def test_evaluate_raises_on_typo(params):
    params.set("width", 100)
    with pytest.raises(ValueError):
        params.evaluate("widht / 2")


def test_evaluate_numeric_does_not_raise(params):
    assert params.evaluate(10) == 10.0
    assert params.evaluate(3.5) == 3.5


def test_evaluate_rejects_attribute_access(params):
    with pytest.raises(ValueError):
        params.evaluate("().__class__.__bases__")


def test_forward_reference_not_reported(params):
    params.set("b", "a * 2")
    params.set("a", 10)
    params.evaluate_all()
    assert params.get("b") == 20.0
    assert params.unresolved_errors() == {}


def test_invalidate_re_evaluates(params):
    params.set("a", 10)
    assert params.get("a") == 10.0
    params.set("a", "5 + 5")
    params.invalidate()
    assert params.get("a") == 10.0


def test_string_initial_values_are_preserved(params):
    params.set("width", 10)
    params.evaluate_all({"label": "Widget", "width": 20.0})
    values = params.get_all_values()
    assert values["label"] == "Widget"
    assert values["width"] == 20.0
    assert params.get("label") == 0.0


def test_string_values_do_not_break_math(params):
    params.set("b", "a * 2")
    params.evaluate_all({"a": 10.0, "name": "Widget"})
    assert params.get("b") == 20.0
    assert params.unresolved_errors() == {}
