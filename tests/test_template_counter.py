from raygeo.template import CounterSettings


def test_defaults():
    settings = CounterSettings()
    assert settings.current == 1
    assert settings.start == 1
    assert settings.step == 1
    assert settings.end is None
    assert settings.auto_advance is True
    assert settings.advance_on_export is False


def test_value_at_uses_step():
    settings = CounterSettings(current=10, start=1, step=5)
    assert settings.value_at(0) == 10
    assert settings.value_at(1) == 15
    assert settings.value_at(3) == 25


def test_value_at_wraps_after_end():
    settings = CounterSettings(current=4, start=1, step=1, end=5)
    assert [settings.value_at(i) for i in range(4)] == [4, 5, 1, 2]


def test_value_at_wraps_with_step():
    settings = CounterSettings(current=90, start=10, step=40, end=95)
    assert [settings.value_at(i) for i in range(3)] == [90, 10, 50]


def test_negative_offset_wraps_backwards():
    settings = CounterSettings(current=1, start=1, step=1, end=3)
    assert settings.value_at(-1) == 3


def test_zero_step_keeps_value():
    settings = CounterSettings(current=7, step=0)
    assert settings.value_at(5) == 7


def test_advanced_moves_current():
    settings = CounterSettings(current=3, step=2)
    assert settings.advanced(4).current == 11
    assert settings.advanced().current == 5
    assert settings.current == 3


def test_previous_moves_back_one_step():
    settings = CounterSettings(current=3, step=2)
    assert settings.advanced(-1).current == 1


def test_reset_returns_to_start():
    settings = CounterSettings(current=42, start=7)
    assert settings.reset().current == 7


def test_values_for_copies():
    settings = CounterSettings(current=8, step=1, end=9, start=1)
    assert settings.values_for(3) == [8, 9, 1]


def test_round_trip():
    settings = CounterSettings(
        current=12,
        start=10,
        step=2,
        end=30,
        auto_advance=False,
        advance_on_export=True,
    )
    assert CounterSettings.from_dict(settings.to_dict()) == settings


def test_from_dict_ignores_missing_keys():
    assert CounterSettings.from_dict({}) == CounterSettings()


def test_from_dict_rejects_end_below_start():
    settings = CounterSettings.from_dict({"start": 10, "end": 5})
    assert settings.end is None


def test_equality():
    assert CounterSettings(current=3) == CounterSettings(current=3)
    assert CounterSettings(current=3) != CounterSettings(current=4)
