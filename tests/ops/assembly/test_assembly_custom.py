"""Tests for the custom-command assembler spec (``CustomSpec``).

The spec emits one ``Ops.custom`` command per input line, in order,
with no coordinate interpretation. These tests exercise it through
the pipeline compute stage (the way Rayforge drives it) and check
the emitted ops.
"""

from raygeo.cnc.execution.specs import ComputePayload
from raygeo.ops import Ops
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.custom import CustomSpec
from raygeo.ops.part import Part
from raygeo.pipeline.execute import clear_cache, execute_stages
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec


def _custom_node(lines: list[str], key: str = "cmd"):
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Compute(
            part=Part(size_mm=(0.0, 0.0)),
            params=ComputePayload(assembler=Assembler(CustomSpec(lines))),
        ),
    )


def _run(node):
    clear_cache()
    completed = []
    execute_stages([node], completed.append, None)
    assert len(completed) == 1
    assert completed[0].output is not None
    return completed[0].output.ops


def _custom_entries(ops):
    return [
        (i, ops.custom_text(i))
        for i in range(ops.len())
        if ops.command_type(i).name == "CUSTOM"
    ]


def test_custom_spec_emits_one_command_per_line_in_order():
    ops = _run(_custom_node(["M101", "G4 P2", "; note"]))
    entries = _custom_entries(ops)
    assert [text for _i, text in entries] == ["M101", "G4 P2", "; note"]


def test_custom_spec_lines_follow_cut_state_commands():
    ops = _run(_custom_node(["M101"]))
    types = [ops.command_type(i).name for i in range(ops.len())]
    assert types[-1] == "CUSTOM"
    # The compute stage stamps cut-state commands before the
    # assembler output; custom lines come after them.
    assert "SET_POWER" in types
    assert types.index("SET_POWER") < types.index("CUSTOM")


def test_custom_spec_preserves_text_verbatim():
    lines = ["G0 Z5 ; {machine.name}", "@include(Rotary Init)", "M8"]
    ops = _run(_custom_node(lines))
    assert [text for _i, text in _custom_entries(ops)] == lines


def test_custom_spec_empty_lines():
    ops = _run(_custom_node([]))
    assert _custom_entries(ops) == []


def test_custom_spec_ops_survive_extend():
    ops = _run(_custom_node(["M101"]))
    merged = Ops()
    merged.extend(ops)
    assert [text for _i, text in _custom_entries(merged)] == ["M101"]


def test_custom_spec_lines_are_distinct_per_node():
    ops = _run(_custom_node(["M101"], key="cmd-a"))
    ops2 = _run(_custom_node(["M102"], key="cmd-b"))
    assert [t for _i, t in _custom_entries(ops)] == ["M101"]
    assert [t for _i, t in _custom_entries(ops2)] == ["M102"]


def test_custom_spec_python_attrs():
    spec = CustomSpec(["M101"])
    assert spec.lines == ["M101"]
    assert "M101" in repr(spec)
    assert spec == CustomSpec(["M101"])
    assert spec != CustomSpec(["M102"])
