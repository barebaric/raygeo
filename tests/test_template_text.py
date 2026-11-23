from raygeo.template import resolve_text


def _resolve(template, values, functions=None, previous=None):
    return resolve_text(template, values, functions or {}, previous)


def test_resolves_values():
    resolved, entry, volatile = _resolve(
        "W={width:.1f} No. {counter:03d}",
        {"width": 50.0, "counter": 7},
    )
    assert resolved == "W=50.0 No. 007"
    assert entry[1] == resolved
    assert volatile is False


def test_resolves_string_variable():
    resolved, _entry, _volatile = _resolve("{name}", {"name": "Widget"})
    assert resolved == "Widget"


def test_resolves_host_function():
    resolved, _entry, volatile = _resolve(
        "ID {uuid8()}", {}, {"uuid8": lambda: "abc12345"}
    )
    assert resolved == "ID abc12345"
    assert volatile is True


def test_reuses_unchanged_placeholder():
    functions = {"uuid8": lambda: "abc12345"}
    first, entry, _ = _resolve("ID {uuid8()} No. {counter}", {"counter": 1}, functions)
    second, entry, _ = _resolve(
        "ID {uuid8()} No. {counter}", {"counter": 2}, functions, entry
    )
    assert first == "ID abc12345 No. 1"
    assert second == "ID abc12345 No. 2"


def test_volatile_value_changes_for_a_new_caller():
    calls = iter(["first", "second"])
    functions = {"uuid8": lambda: next(calls)}
    first, _entry, _ = _resolve("{uuid8()}", {}, functions)
    second, _entry, _ = _resolve("{uuid8()}", {}, functions)
    assert first == "first"
    assert second == "second"


def test_unknown_placeholder_is_left_verbatim():
    resolved, _entry, _volatile = _resolve("{missing}", {})
    assert resolved == "{missing}"


def test_empty_template_is_none():
    resolved, entry, volatile = _resolve("", {})
    assert resolved is None
    assert entry[1] is None
    assert volatile is False
