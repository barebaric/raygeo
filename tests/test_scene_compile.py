"""Tests for the Rust scene compiler (Ops::compile_scene_3d)."""

import numpy as np

from raygeo.ops import Ops
from raygeo.ops.axis import Axis
from raygeo.ops.convert import Encoder, LayerConfig, SceneSpec


def _identity_spec(layer_configs=None):
    return (
        np.eye(4, dtype=np.float32).tolist(),
        layer_configs or {},
    )


def _compile(ops, layer_configs=None):
    w2v, configs = _identity_spec(layer_configs)
    return ops.compile_scene_3d(w2v, configs)


def _flat_group(data):
    groups = data["groups"]
    assert len(groups) >= 1, f"expected >=1 group, got {len(groups)}"
    for g in groups:
        if not g["is_rotary"]:
            return g
    return groups[0]


# ── Basic structure ─────────────────────────────────────────────


def test_empty_ops():
    ops = Ops()
    data = _compile(ops)
    assert len(data["groups"]) == 0
    assert data["laser_uid_order"] == []
    assert data["layer_infos"] == []


def test_line_to():
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.set_power(0.5)
    ops.line_to(10.0, 20.0, 0.0)

    data = _compile(ops)
    g = _flat_group(data)

    pv = g["powered_verts"].reshape(-1, 3)
    assert pv.shape == (2, 3)
    assert np.allclose(pv[0], [0, 0, 0])
    assert np.allclose(pv[1], [10, 20, 0])

    pvv = g["power_values"]
    assert pvv.shape == (2,)
    assert np.allclose(pvv, [0.5, 0.5])


def test_move_to_produces_travel():
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.move_to(10.0, 20.0, 0.0)

    data = _compile(ops)
    g = _flat_group(data)

    tv = g["travel_verts"].reshape(-1, 3)
    assert tv.shape == (2, 3)
    assert np.allclose(tv[0], [0, 0, 0.01])
    assert np.allclose(tv[1], [10, 20, 0.01])


def test_zero_power_line():
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.set_power(0.0)
    ops.line_to(5.0, 5.0, 0.0)

    data = _compile(ops)
    g = _flat_group(data)

    assert g["powered_verts"].size == 0
    zpv = g["zero_power_verts"].reshape(-1, 3)
    assert zpv.shape == (2, 3)
    assert np.allclose(zpv[0], [0, 0, 0.01])
    assert np.allclose(zpv[1], [5, 5, 0.01])


# ── Power tracking ──────────────────────────────────────────────


def test_power_tracking():
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.set_power(0.3)
    ops.line_to(1.0, 0.0, 0.0)
    ops.set_power(0.8)
    ops.line_to(2.0, 0.0, 0.0)

    data = _compile(ops)
    g = _flat_group(data)

    pvv = g["power_values"]
    assert pvv.shape == (4,)
    assert np.allclose(pvv, [0.3, 0.3, 0.8, 0.8])


# ── Laser index ─────────────────────────────────────────────────


def test_laser_index():
    ops = Ops()
    ops.set_head("laser_a")
    ops.move_to(0.0, 0.0, 0.0)
    ops.set_power(1.0)
    ops.line_to(1.0, 0.0, 0.0)
    ops.set_head("laser_b")
    ops.line_to(2.0, 0.0, 0.0)

    data = _compile(ops)
    g = _flat_group(data)

    assert data["laser_uid_order"] == ["laser_a", "laser_b"]

    pvl = g["laser_indices"]
    assert pvl.shape == (4,)
    assert pvl[0] == 0 and pvl[1] == 0
    assert pvl[2] == 1 and pvl[3] == 1


# ── Arc ─────────────────────────────────────────────────────────


def test_arc_to():
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.set_power(1.0)
    ops.arc_to(10.0, 0.0, 5.0, 0.0, False, 0.0)

    data = _compile(ops)
    g = _flat_group(data)

    pv = g["powered_verts"].reshape(-1, 3)
    assert pv.shape[0] >= 4
    assert np.allclose(pv[0], [0, 0, 0])
    assert np.allclose(pv[-1], [10, 0, 0])


# ── Bezier ──────────────────────────────────────────────────────


def test_bezier_to():
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.set_power(1.0)
    ops.bezier_to(
        (2.0, 8.0, 0.0),
        (8.0, 8.0, 0.0),
        (10.0, 0.0, 0.0),
    )

    data = _compile(ops)
    g = _flat_group(data)

    pv = g["powered_verts"].reshape(-1, 3)
    assert pv.shape[0] >= 4
    assert np.allclose(pv[0], [0, 0, 0])
    assert np.allclose(pv[-1], [10, 0, 0])


# ── Scanline ────────────────────────────────────────────────────


def test_scan_line():
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.set_power(0.0)
    power_values = bytes([0, 0, 255, 255, 0, 0])
    ops.scan_to(10.0, 0.0, 0.0, power_values=power_values)

    data = _compile(ops)
    g = _flat_group(data)

    assert g["powered_verts"].size == 0

    zpv = g["zero_power_verts"].reshape(-1, 3)
    assert zpv.shape[0] >= 2

    ov_pos = g["overlay_positions"].reshape(-1, 3)
    assert ov_pos.shape[0] >= 2
    ov_pow = g["overlay_power_values"]
    assert ov_pow.shape[0] >= 2
    assert np.allclose(ov_pow, [1.0, 1.0])


def test_scan_line_all_zero():
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    power_values = bytes([0, 0, 0, 0])
    ops.scan_to(10.0, 0.0, 0.0, power_values=power_values)

    data = _compile(ops)
    g = _flat_group(data)

    assert g["overlay_positions"].size == 0


# ── Per-command offsets ─────────────────────────────────────────


def test_cmd_offsets():
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.set_power(1.0)
    ops.line_to(1.0, 0.0, 0.0)
    ops.line_to(2.0, 0.0, 0.0)

    data = _compile(ops)
    g = _flat_group(data)

    total_cmds = ops.len()
    pv_off = g["powered_cmd_offsets"]
    assert len(pv_off) == total_cmds + 1
    assert pv_off[0] == 0
    assert pv_off[-1] == 4

    tv_off = g["travel_cmd_offsets"]
    assert len(tv_off) == total_cmds + 1


# ── Layer split ─────────────────────────────────────────────────


def test_layer_split():
    ops = Ops()
    ops.job_start()

    ops.layer_start("flat_layer")
    ops.move_to(0.0, 0.0, 0.0)
    ops.set_power(1.0)
    ops.line_to(1.0, 0.0, 0.0)
    ops.layer_end("flat_layer")

    ops.layer_start("rot_layer")
    ops.move_to(0.0, 0.0, 0.0)
    ops.set_power(1.0)
    ops.line_to(2.0, 0.0, 0.0)
    ops.layer_end("rot_layer")

    ops.job_end()

    configs = {
        "flat_layer": {
            "rotary_enabled": False,
            "rotary_diameter": 0.0,
        },
        "rot_layer": {
            "rotary_enabled": True,
            "rotary_diameter": 50.0,
        },
    }
    data = _compile(ops, configs)

    assert len(data["groups"]) == 2
    flat = [g for g in data["groups"] if not g["is_rotary"]]
    rot = [g for g in data["groups"] if g["is_rotary"]]
    assert len(flat) == 1
    assert len(rot) == 1

    assert len(data["layer_infos"]) == 2


# ── World transform ─────────────────────────────────────────────


def test_world_transform():
    transform = np.eye(4, dtype=np.float32)
    transform[0, 3] = 100.0

    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.set_power(1.0)
    ops.line_to(10.0, 0.0, 0.0)

    data = ops.compile_scene_3d(transform.tolist(), {})
    g = _flat_group(data)

    pv = g["powered_verts"].reshape(-1, 3)
    assert np.allclose(pv[0], [100, 0, 0])
    assert np.allclose(pv[1], [110, 0, 0])


# ── Z offset on non-powered ─────────────────────────────────────


def test_z_offset_travel():
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.move_to(1.0, 0.0, 0.0)

    data = _compile(ops)
    g = _flat_group(data)

    tv = g["travel_verts"].reshape(-1, 3)
    assert abs(tv[0, 2]) > 0.0
    assert abs(tv[1, 2]) > 0.0


# ── Rotary ──────────────────────────────────────────────────────


def test_rotary_basic():
    ops = Ops()
    ops.layer_start("rot")
    ops.move_to(0.0, 0.0, 0.0, extra={Axis.A: 0.0})
    ops.set_power(1.0)
    ops.line_to(10.0, 0.0, 0.0, extra={Axis.A: 90.0})
    ops.layer_end("rot")

    configs = {
        "rot": {
            "rotary_enabled": True,
            "rotary_diameter": 50.0,
        },
    }
    data = _compile(ops, configs)

    rot_groups = [g for g in data["groups"] if g["is_rotary"]]
    assert len(rot_groups) == 1

    pv = rot_groups[0]["powered_verts"].reshape(-1, 3)
    assert pv.shape[0] >= 2


def test_rotary_no_diameter():
    ops = Ops()
    ops.layer_start("rot")
    ops.move_to(0.0, 0.0, 0.0, extra={Axis.A: 0.0})
    ops.set_power(1.0)
    ops.line_to(10.0, 0.0, 0.0, extra={Axis.A: 90.0})
    ops.layer_end("rot")

    configs = {
        "rot": {
            "rotary_enabled": True,
            "rotary_diameter": 0.0,
        },
    }
    data = _compile(ops, configs)
    assert len(data["groups"]) >= 1


# ── Overlay extraction (Phase 2) ────────────────────────────────


def test_overlay_all_zero():
    """All-zero scanline power → no overlay segments."""
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.scan_to(10.0, 0.0, 0.0, power_values=bytes([0, 0, 0, 0]))

    data = _compile(ops)
    g = _flat_group(data)

    assert g["overlay_positions"].size == 0
    assert g["overlay_power_values"].size == 0


def test_overlay_all_max():
    """All-255 scanline → single overlay segment spanning full range."""
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.scan_to(10.0, 0.0, 0.0, power_values=bytes([255, 255, 255, 255]))

    data = _compile(ops)
    g = _flat_group(data)

    ov_pos = g["overlay_positions"].reshape(-1, 3)
    assert ov_pos.shape == (2, 3)
    assert np.allclose(ov_pos[0], [0, 0, 0])
    assert np.allclose(ov_pos[1], [10, 0, 0])

    ov_pow = g["overlay_power_values"]
    assert ov_pow.shape == (2,)
    assert np.allclose(ov_pow, [1.0, 1.0])


def test_overlay_mixed():
    """Known pattern (0,255,255,0,0,255) → correct segment boundaries."""
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.scan_to(6.0, 0.0, 0.0, power_values=bytes([0, 255, 255, 0, 0, 255]))

    data = _compile(ops)
    g = _flat_group(data)

    ov_pos = g["overlay_positions"].reshape(-1, 3)
    assert ov_pos.shape == (4, 3)

    # First segment: pixels 1-3 (t=1/6 to t=3/6)
    assert np.allclose(ov_pos[0], [1.0, 0, 0])
    assert np.allclose(ov_pos[1], [3.0, 0, 0])

    # Second segment: pixel 5 to end (t=5/6 to t=1)
    assert np.allclose(ov_pos[2], [5.0, 0, 0])
    assert np.allclose(ov_pos[3], [6.0, 0, 0])


def test_overlay_power_values():
    """Power byte / 255.0 stored correctly in overlay."""
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.scan_to(4.0, 0.0, 0.0, power_values=bytes([128, 128, 128, 128]))

    data = _compile(ops)
    g = _flat_group(data)

    ov_pow = g["overlay_power_values"]
    assert ov_pow.shape == (2,)
    assert abs(ov_pow[0] - 128.0 / 255.0) < 0.01
    assert abs(ov_pow[1] - 128.0 / 255.0) < 0.01


def test_overlay_laser_index():
    """Laser index propagated to overlay vertices."""
    ops = Ops()
    ops.set_head("laser_x")
    ops.move_to(0.0, 0.0, 0.0)
    ops.scan_to(4.0, 0.0, 0.0, power_values=bytes([255, 255, 255, 255]))

    data = _compile(ops)
    g = _flat_group(data)

    ov_lid = g["overlay_laser_indices"]
    assert ov_lid.shape == (2,)
    assert ov_lid[0] == 0
    assert ov_lid[1] == 0


# ── PyO3 binding: PySceneSpec / LayerConfig (Phase 4) ──────────


def test_scene_spec_construction():
    """Construct SceneSpec from Python with world_to_visual and
    layer_configs."""
    w2v = np.eye(4, dtype=np.float32).tolist()
    configs = {
        "layer_a": LayerConfig(
            rotary_enabled=True,
            rotary_diameter=50.0,
            axis_position=10.0,
            reverse=True,
        ),
    }
    spec = SceneSpec(w2v, configs)
    assert spec.world_to_visual[0][0] == 1.0
    assert len(spec.layer_configs) == 1
    uid, cfg = spec.layer_configs[0]
    assert uid == "layer_a"
    assert cfg.rotary_enabled is True
    assert cfg.rotary_diameter == 50.0
    assert cfg.axis_position == 10.0
    assert cfg.reverse is True


def test_scene_spec_default_layer_config():
    """LayerConfig defaults are all false/zero."""
    cfg = LayerConfig()
    assert cfg.rotary_enabled is False
    assert cfg.rotary_diameter == 0.0
    assert cfg.axis_position == 0.0
    assert cfg.reverse is False


def test_compile_returns_numpy_float32():
    """compile_scene_3d returns numpy arrays with dtype float32."""
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.set_power(1.0)
    ops.line_to(10.0, 0.0, 0.0)

    data = _compile(ops)
    g = _flat_group(data)

    assert g["powered_verts"].dtype == np.float32
    assert g["power_values"].dtype == np.float32
    assert g["laser_indices"].dtype == np.float32
    assert g["travel_verts"].dtype == np.float32
    assert g["zero_power_verts"].dtype == np.float32
    assert g["overlay_positions"].dtype == np.float32
    assert g["overlay_power_values"].dtype == np.float32
    assert g["overlay_laser_indices"].dtype == np.float32


def test_compile_returns_int32_offsets():
    """compile_scene_3d returns offset arrays with dtype int32."""
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.set_power(1.0)
    ops.line_to(10.0, 0.0, 0.0)

    data = _compile(ops)
    g = _flat_group(data)

    assert g["powered_cmd_offsets"].dtype == np.int32
    assert g["travel_cmd_offsets"].dtype == np.int32
    assert g["overlay_cmd_offsets"].dtype == np.int32


def test_numpy_arrays_contiguous():
    """Returned numpy arrays are C-contiguous (zero-copy)."""
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.set_power(1.0)
    ops.line_to(10.0, 0.0, 0.0)

    data = _compile(ops)
    g = _flat_group(data)

    assert g["powered_verts"].flags["C_CONTIGUOUS"]
    assert g["power_values"].flags["C_CONTIGUOUS"]
    assert g["powered_cmd_offsets"].flags["C_CONTIGUOUS"]


def test_compile_array_shapes():
    """Verify returned array shapes match expected vertex counts."""
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.set_power(1.0)
    ops.line_to(1.0, 0.0, 0.0)
    ops.line_to(2.0, 0.0, 0.0)

    data = _compile(ops)
    g = _flat_group(data)

    assert g["powered_verts"].shape == (12,)
    assert g["power_values"].shape == (4,)
    assert g["laser_indices"].shape == (4,)


def test_encode_output_scene_accessible():
    """PyEncodeOutput.Scene variant is accessible through the pipeline."""
    w2v = np.eye(4, dtype=np.float32).tolist()
    spec = SceneSpec(w2v, {})
    enc = Encoder(spec)
    assert enc is not None
    assert "SceneSpec" in repr(spec)
