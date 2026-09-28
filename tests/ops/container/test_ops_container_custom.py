"""Tests for the Custom command (raw machine-code line).

A Custom command carries one user-provided machine-code text line
through the ops pipeline. It is a State command: transform stages
leave it untouched and it reaches the encoder verbatim (aside from
path-variable expansion, which happens in the G-code encoder).
"""

import pytest

from raygeo.ops import Ops
from raygeo.ops.types import CommandCategory, CommandType, category


def test_custom_builder_and_type():
    ops = Ops()
    ops.custom("M101")
    assert len(ops) == 1
    assert ops.command_type(0) == CommandType.CUSTOM
    assert category(ops.command_type(0)) == CommandCategory.STATE


def test_custom_text_accessor():
    ops = Ops()
    ops.set_power(1.0)
    ops.custom("G4 P2 ; settle")
    assert ops.custom_text(1) == "G4 P2 ; settle"


def test_custom_text_accessor_wrong_type():
    ops = Ops()
    ops.dwell(100.0)
    with pytest.raises(TypeError):
        ops.custom_text(0)


def test_custom_text_accessor_out_of_range():
    ops = Ops()
    ops.custom("M101")
    with pytest.raises(IndexError):
        ops.custom_text(1)


def test_custom_text_stored_unexpanded():
    ops = Ops()
    ops.custom("; head {machine.name} layer {layer.name}")
    assert ops.custom_text(0) == "; head {machine.name} layer {layer.name}"


def test_custom_inspect():
    ops = Ops()
    ops.custom("M101")
    info = ops.inspect(0)
    assert info.type_ == CommandType.CUSTOM
    assert info.custom_text == "M101"


def test_custom_ordering_preserved_by_extend():
    first = Ops()
    first.set_power(1.0)
    first.custom("M101")
    first.dwell(50.0)
    second = Ops()
    second.custom("M102")
    second.set_feed_rate(1000)
    second.custom("M103")

    merged = Ops()
    merged.extend(first)
    merged.extend(second)

    types = [merged.command_type(i).name for i in range(merged.len())]
    assert types == [
        "SET_POWER",
        "CUSTOM",
        "DWELL",
        "CUSTOM",
        "SET_FEED_RATE",
        "CUSTOM",
    ]
    assert merged.custom_text(1) == "M101"
    assert merged.custom_text(3) == "M102"
    assert merged.custom_text(5) == "M103"


def test_custom_does_not_track_state():
    ops = Ops()
    ops.set_power(0.5)
    ops.custom("M101")
    state = ops.state_at(1)
    assert state.power == 0.5
    assert state.feed_rate is None
    assert state.active_head_uid is None
