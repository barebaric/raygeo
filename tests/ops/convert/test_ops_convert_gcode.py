"""Tests for the Rust G-code encoder (ops.to_gcode).

Replaces the inline Rust unit tests that were removed from gcode.rs.
Each test builds an Ops sequence, calls ops.to_gcode(), and asserts
on the emitted G-code text and op-map.
"""

from raygeo.ops import Ops
from raygeo.ops.axis import Axis
from raygeo.ops.convert import GcodeDialectSpec
from raygeo.ops.state import AirAssistMode, CoolantMode

# ── Helpers ──────────────────────────────────────────────────────


def _ctx(**overrides) -> dict:
    defaults = dict(
        gcode_precision=3,
        max_travel_speed=6000.0,
        default_head_uid="h0",
        heads=[{"uid": "h0", "tool_number": 0, "max_power": 100.0}],
        active_wcs="",
        layer_wcs={},
        macros={},
        path_vars={},
        layer_path_vars={},
        workpiece_path_vars={},
    )
    defaults.update(overrides)
    return defaults


def _encode(ops, dialect=None, ctx=None):
    return ops.to_gcode(dialect or GcodeDialectSpec(), ctx or _ctx())


# ── Coordinate formatting ───────────────────────────────────────


def test_format_coord_trailing_zeros_stripped():
    ops = Ops()
    ops.job_start()
    ops.move_to(1.500, 2.0, 3.100)
    ops.job_end()
    text = _encode(ops)["text"]
    assert "X1.5" in text
    assert "Y2" in text
    assert "Z3.1" in text


def test_format_coord_near_zero_snapped():
    ops = Ops()
    ops.job_start()
    ops.move_to(1e-10, 0.0, 0.0)
    ops.job_end()
    text = _encode(ops)["text"]
    assert "X0" in text


def test_format_precision_respected():
    ops = Ops()
    ops.job_start()
    ops.move_to(1.23456789, 0.0, 0.0)
    ops.job_end()
    text = _encode(ops, GcodeDialectSpec(gcode_precision=2))["text"]
    assert "X1.23" in text


# ── Omit unchanged coords ───────────────────────────────────────


def test_omit_unchanged_coords_enabled():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.job_end()
    text = _encode(ops)["text"]
    lines = [line for line in text.splitlines() if line.startswith("G1 X10")]
    assert len(lines) >= 1
    assert "Y" not in lines[0]


def test_omit_unchanged_coords_disabled():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.job_end()
    text = _encode(ops, GcodeDialectSpec(omit_unchanged_coords=False))["text"]
    lines = [line for line in text.splitlines() if line.startswith("G1 X10")]
    assert any("Y0" in line for line in lines)


def test_none_changed_emits_at_least_x():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(5.0, 5.0, 0.0)
    ops.line_to(5.0, 5.0, 0.0)
    ops.job_end()
    text = _encode(ops)["text"]
    lines = [line for line in text.splitlines() if line.startswith("G1")]
    assert any("X5" in line for line in lines)


# ── Laser state machine ─────────────────────────────────────────


def test_laser_on_emitted_for_cut():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.job_end()
    text = _encode(ops)["text"]
    assert "M4 S100" in text


def test_laser_off_before_rapid():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.move_to(20.0, 0.0, 0.0)
    ops.line_to(30.0, 0.0, 0.0)
    ops.job_end()
    text = _encode(ops)["text"]
    lines = text.splitlines()
    m5_idx = lines.index("M5")
    g0_idx = next(
        i for i, line in enumerate(lines) if line.startswith("G0 X20")
    )
    assert m5_idx < g0_idx


def test_zero_power_no_laser():
    ops = Ops()
    ops.job_start()
    ops.set_power(0.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(5.0, 5.0, 0.0)
    ops.job_end()
    text = _encode(ops)["text"]
    assert "M4 S0" not in text


def test_laser_off_at_job_end():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.job_end()
    text = _encode(ops)["text"]
    lines = text.splitlines()
    assert lines[-2] == "M5"
    assert lines[-1] == "M30"


# ── Power tracking ──────────────────────────────────────────────


def test_power_scales_by_head_max():
    ops = Ops()
    ops.job_start()
    ops.set_power(0.5)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.job_end()
    ctx = _ctx(heads=[{"uid": "h0", "tool_number": 0, "max_power": 200.0}])
    text = _encode(ops, ctx=ctx)["text"]
    assert "M4 S100" in text


def test_redundant_power_suppressed():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(5.0, 0.0, 0.0)
    ops.set_power(1.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.job_end()
    text = _encode(ops)["text"]
    assert text.count("M4 S100") == 1


# ── Speed / modal feedrate ──────────────────────────────────────


def test_modal_speed_emitted():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(3000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.job_end()
    text = _encode(ops)["text"]
    assert "G1 F3000" in text


def test_modal_speed_suppressed_when_unchanged():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(3000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(5.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.job_end()
    text = _encode(ops)["text"]
    assert text.count("G1 F3000") == 1


def test_non_modal_always_emits_feed():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(3000)
    ops.set_rapid_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.move_to(20.0, 0.0, 0.0)
    ops.line_to(30.0, 0.0, 0.0)
    ops.job_end()
    text = _encode(ops, GcodeDialectSpec(modal_feedrate=False))["text"]
    g1_lines = [line for line in text.splitlines() if line.startswith("G1")]
    assert all("F3000" in line for line in g1_lines)


def test_g0_with_speed():
    ops = Ops()
    ops.job_start()
    ops.set_rapid_rate(5000)
    ops.move_to(10.0, 0.0, 0.0)
    ops.job_end()
    text = _encode(ops, GcodeDialectSpec(can_g0_with_speed=True))["text"]
    assert "F5000" in text


# ── Travel moves ────────────────────────────────────────────────


def test_rapid_move_basic():
    ops = Ops()
    ops.job_start()
    ops.move_to(10.0, 20.0, 30.0)
    ops.job_end()
    text = _encode(ops)["text"]
    assert "G0 X10 Y20 Z30" in text


# ── Cut moves ───────────────────────────────────────────────────


def test_line_to_basic():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 20.0, 0.0)
    ops.job_end()
    text = _encode(ops)["text"]
    assert "G1 X10 Y20 F1000" in text


# ── Arc moves ───────────────────────────────────────────────────


def test_arc_cw():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.arc_to(10.0, 10.0, 5.0, 0.0, clockwise=True)
    ops.job_end()
    text = _encode(ops)["text"]
    assert "G2 X10 Y10 I5 J0" in text


def test_arc_ccw():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.arc_to(10.0, 10.0, 5.0, 0.0, clockwise=False)
    ops.job_end()
    text = _encode(ops)["text"]
    assert "G3 X10 Y10 I5 J0" in text


# ── Bezier moves ────────────────────────────────────────────────


def test_bezier_with_g5_template():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.bezier_to(
        (3.0, 0.0, 0.0),
        (7.0, 10.0, 0.0),
        (10.0, 10.0, 0.0),
    )
    ops.job_end()
    d = GcodeDialectSpec(bezier_cubic="G5 X{x} Y{y} I{i} J{j} P{p} Q{q}")
    text = _encode(ops, d)["text"]
    assert "G5" in text
    assert "X10" in text
    assert "Y10" in text


def test_bezier_empty_template_linearized():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.bezier_to(
        (3.0, 0.0, 0.0),
        (7.0, 10.0, 0.0),
        (10.0, 10.0, 0.0),
    )
    ops.job_end()
    text = _encode(ops, GcodeDialectSpec(bezier_cubic=""))["text"]
    g1_lines = [line for line in text.splitlines() if line.startswith("G1")]
    assert len(g1_lines) > 1


# ── Dwell ───────────────────────────────────────────────────────


def test_dwell():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.dwell(500.0)
    ops.job_end()
    text = _encode(ops)["text"]
    assert "G4 P0.500" in text


def test_dwell_empty_template():
    ops = Ops()
    ops.job_start()
    ops.dwell(100.0)
    ops.job_end()
    text = _encode(ops, GcodeDialectSpec(dwell=""))["text"]
    assert "G4" not in text


# ── Air assist ──────────────────────────────────────────────────


def test_air_assist_on_off():
    ops = Ops()
    ops.job_start()
    ops.set_air_assist(AirAssistMode.ON)
    ops.set_power(1.0)
    ops.set_feed_rate(500)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(5.0, 5.0, 0.0)
    ops.set_air_assist(AirAssistMode.OFF)
    ops.job_end()
    text = _encode(ops)["text"]
    assert "M8" in text
    assert "M9" in text


def test_air_assist_noop_when_same():
    ops = Ops()
    ops.job_start()
    ops.set_air_assist(AirAssistMode.ON)
    ops.set_air_assist(AirAssistMode.ON)
    ops.job_end()
    text = _encode(ops)["text"]
    assert text.count("M8") == 1


def test_air_assist_off_at_job_end():
    ops = Ops()
    ops.job_start()
    ops.set_air_assist(AirAssistMode.ON)
    ops.job_end()
    text = _encode(ops)["text"]
    lines = text.splitlines()
    assert lines[-2] == "M9"


# ── Coolant ─────────────────────────────────────────────────────


def test_coolant_flood():
    ops = Ops()
    ops.job_start()
    ops.set_coolant(CoolantMode.FLOOD)
    ops.job_end()
    assert "M8" in _encode(ops)["text"]


def test_coolant_mist():
    ops = Ops()
    ops.job_start()
    ops.set_coolant(CoolantMode.MIST)
    ops.job_end()
    assert "M7" in _encode(ops)["text"]


def test_coolant_off():
    ops = Ops()
    ops.job_start()
    ops.set_coolant(CoolantMode.FLOOD)
    ops.set_coolant(CoolantMode.OFF)
    ops.job_end()
    assert "M9" in _encode(ops)["text"]


def test_coolant_noop_when_same():
    ops = Ops()
    ops.job_start()
    ops.set_coolant(CoolantMode.FLOOD)
    ops.set_coolant(CoolantMode.FLOOD)
    ops.job_end()
    assert _encode(ops)["text"].count("M8") == 1


def test_coolant_off_at_job_end():
    ops = Ops()
    ops.job_start()
    ops.set_coolant(CoolantMode.FLOOD)
    ops.job_end()
    text = _encode(ops)["text"]
    lines = text.splitlines()
    assert lines[-1] == "M30"
    assert "M9" in lines


# ── Spindle ─────────────────────────────────────────────────────


def test_spindle_on():
    ops = Ops()
    ops.job_start()
    ops.set_spindle_rpm(12000)
    ops.job_end()
    assert "M3 S12000" in _encode(ops)["text"]


def test_spindle_off():
    ops = Ops()
    ops.job_start()
    ops.set_spindle_rpm(12000)
    ops.set_spindle_rpm(0)
    ops.job_end()
    assert "M5" in _encode(ops)["text"]


def test_spindle_change():
    ops = Ops()
    ops.job_start()
    ops.set_spindle_rpm(8000)
    ops.set_spindle_rpm(12000)
    ops.job_end()
    lines = _encode(ops)["text"].splitlines()
    m5_idx = next(i for i, line in enumerate(lines) if line == "M5")
    m3_idx = next(i for i, line in enumerate(lines) if line == "M3 S12000")
    assert m5_idx < m3_idx


def test_spindle_off_at_job_end():
    ops = Ops()
    ops.job_start()
    ops.set_spindle_rpm(12000)
    ops.job_end()
    text = _encode(ops)["text"]
    lines = text.splitlines()
    assert lines[-1] == "M30"
    assert "M5" in lines


# ── Tool change ─────────────────────────────────────────────────


def test_tool_change():
    ops = Ops()
    ops.job_start()
    ops.set_head("h1")
    ops.job_end()
    ctx = _ctx(heads=[{"uid": "h1", "tool_number": 2, "max_power": 50.0}])
    assert "T2" in _encode(ops, ctx=ctx)["text"]


def test_tool_change_noop_when_same():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.set_head("h0")
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.set_head("h0")
    ops.line_to(20.0, 0.0, 0.0)
    ops.job_end()
    text = _encode(ops)["text"]
    assert text.count("T0") == 1


# ── Preamble / postscript ───────────────────────────────────────


def test_preamble_emitted():
    ops = Ops()
    ops.job_start()
    ops.job_end()
    lines = _encode(ops)["text"].splitlines()
    assert lines[0] == "G90"


def test_postscript_emitted():
    ops = Ops()
    ops.job_start()
    ops.job_end()
    lines = _encode(ops)["text"].splitlines()
    assert lines[-1] == "M30"


def test_wcs_injected_after_preamble():
    ops = Ops()
    ops.job_start()
    ops.job_end()
    ctx = _ctx(active_wcs="G55")
    d = GcodeDialectSpec(inject_wcs_after_preamble=True)
    lines = _encode(ops, d, ctx)["text"].splitlines()
    assert lines[0] == "G90"
    assert lines[1] == "G55"


# ── Op-map ──────────────────────────────────────────────────────


def test_op_map_job_start():
    ops = Ops()
    ops.job_start()
    ops.job_end()
    result = _encode(ops)
    assert result["op_to_machine_code"][0][1] >= 1


def test_op_map_move():
    ops = Ops()
    ops.job_start()
    ops.move_to(10.0, 0.0, 0.0)
    ops.job_end()
    result = _encode(ops)
    assert result["op_to_machine_code"][1][1] >= 1


def test_machine_code_to_op():
    ops = Ops()
    ops.job_start()
    ops.move_to(10.0, 0.0, 0.0)
    ops.job_end()
    result = _encode(ops)
    assert len(result["machine_code_to_op"]) > 0


# ── Macros ──────────────────────────────────────────────────────


def test_layer_start_macro():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.layer_start("L1")
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.layer_end("L1")
    ops.job_end()
    macros = {
        "layer_start": {
            "name": "ls",
            "code": ["; layer start"],
            "enabled": True,
        },
        "all_macros": {
            "ls": {
                "name": "ls",
                "code": ["; layer start"],
                "enabled": True,
            },
        },
    }
    text = _encode(ops, ctx=_ctx(macros=macros))["text"]
    assert "; layer start" in text


def test_disabled_macro_not_emitted():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.layer_start("L1")
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.layer_end("L1")
    ops.job_end()
    macros = {
        "layer_start": {
            "name": "ls",
            "code": ["; should not appear"],
            "enabled": False,
        },
        "all_macros": {},
    }
    text = _encode(ops, ctx=_ctx(macros=macros))["text"]
    assert "should not appear" not in text


def test_macro_include_expansion():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.layer_start("L1")
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.layer_end("L1")
    ops.job_end()
    macros = {
        "layer_start": {
            "name": "outer",
            "code": ["@include(inner)"],
            "enabled": True,
        },
        "all_macros": {
            "outer": {
                "name": "outer",
                "code": ["@include(inner)"],
                "enabled": True,
            },
            "inner": {
                "name": "inner",
                "code": ["; expanded"],
                "enabled": True,
            },
        },
    }
    text = _encode(ops, ctx=_ctx(macros=macros))["text"]
    assert "; expanded" in text


def test_macro_cycle_detection():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.layer_start("L1")
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.layer_end("L1")
    ops.job_end()
    macros = {
        "layer_start": {
            "name": "a",
            "code": ["@include(b)"],
            "enabled": True,
        },
        "all_macros": {
            "a": {"name": "a", "code": ["@include(b)"], "enabled": True},
            "b": {"name": "b", "code": ["@include(a)"], "enabled": True},
        },
    }
    result = _encode(ops, ctx=_ctx(macros=macros))
    assert "text" in result


def test_macro_path_vars():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.layer_start("L1")
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.layer_end("L1")
    ops.job_end()
    macros = {
        "layer_start": {
            "name": "preamble",
            "code": ["; machine={machine.name}"],
            "enabled": True,
        },
        "all_macros": {
            "preamble": {
                "name": "preamble",
                "code": ["; machine={machine.name}"],
                "enabled": True,
            },
        },
    }
    ctx = _ctx(
        path_vars={"machine.name": "MyLaser"},
        macros=macros,
    )
    text = _encode(ops, ctx=ctx)["text"]
    assert "; machine=MyLaser" in text


# ── Preamble path vars ──────────────────────────────────────────


def test_preamble_path_vars():
    ops = Ops()
    ops.job_start()
    ops.job_end()
    ctx = _ctx(path_vars={"machine.name": "TestM"})
    d = GcodeDialectSpec(preamble=["; {machine.name}"])
    text = _encode(ops, d, ctx)["text"]
    assert "; TestM" in text


# ── Extra axes ──────────────────────────────────────────────────


def test_extra_axes():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 10.0, 0.0, extra={Axis.A: 45.0})
    ops.job_end()
    text = _encode(ops)["text"]
    assert "A45" in text


def test_extra_axes_omit_unchanged():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0, extra={Axis.A: 45.0})
    ops.line_to(20.0, 0.0, 0.0, extra={Axis.A: 45.0})
    ops.job_end()
    text = _encode(ops)["text"]
    g1_lines = [
        line for line in text.splitlines() if line.startswith("G1 X20")
    ]
    assert all("A" not in line for line in g1_lines)


# ── Continuous laser mode ───────────────────────────────────────


def test_continuous_laser_no_m5_before_rapid():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.move_to(20.0, 0.0, 0.0)
    ops.line_to(30.0, 0.0, 0.0)
    ops.job_end()
    text = _encode(ops, GcodeDialectSpec(continuous_laser_mode=True))["text"]
    assert "M5" not in text


def test_continuous_laser_s0_on_rapid():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.move_to(20.0, 0.0, 0.0)
    ops.job_end()
    text = _encode(ops, GcodeDialectSpec(continuous_laser_mode=True))["text"]
    rapid_lines = [line for line in text.splitlines() if line.startswith("G0")]
    assert any("S0" in line for line in rapid_lines)


# ── WCS injection ───────────────────────────────────────────────


def test_layer_wcs_change():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.layer_start("L1")
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.layer_end("L1")
    ops.job_end()
    ctx = _ctx(
        active_wcs="G54",
        layer_wcs={"L1": "G55"},
    )
    d = GcodeDialectSpec(inject_wcs_after_preamble=True)
    text = _encode(ops, d, ctx)["text"]
    assert "G55" in text


def test_layer_wcs_no_change_no_emit():
    ops = Ops()
    ops.job_start()
    ops.set_power(1.0)
    ops.set_feed_rate(1000)
    ops.layer_start("L1")
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 0.0, 0.0)
    ops.layer_end("L1")
    ops.job_end()
    ctx = _ctx(
        active_wcs="G54",
        layer_wcs={"L1": "G54"},
    )
    d = GcodeDialectSpec(inject_wcs_after_preamble=True)
    text = _encode(ops, d, ctx)["text"]
    lines = text.splitlines()
    wcs_lines = [line for line in lines if line in ("G54", "G55")]
    assert len(wcs_lines) == 1


# ── Empty ops ───────────────────────────────────────────────────


def test_empty_job():
    ops = Ops()
    ops.job_start()
    ops.job_end()
    result = _encode(ops)
    assert "text" in result
    assert "G90" in result["text"]
    assert "M30" in result["text"]


# ── Frequency / pulse width (state-only, no gcode) ─────────────


def test_frequency_no_gcode():
    ops = Ops()
    ops.job_start()
    ops.set_frequency(1000)
    ops.job_end()
    text = _encode(ops)["text"]
    lines = [line for line in text.splitlines() if "F1000" in line]
    assert len(lines) == 0


def test_pulse_width_no_gcode():
    ops = Ops()
    ops.job_start()
    ops.set_pulse_width(50)
    ops.job_end()
    text = _encode(ops)["text"]
    lines = [line for line in text.splitlines() if "50" in line]
    assert len(lines) == 0
