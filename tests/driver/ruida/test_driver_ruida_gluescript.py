"""GlueScript tests: the native job authoring and compilation model
must reproduce the Ruida Protocol Analyzer's transcript and compiled
rpascript byte-for-byte."""

from pathlib import Path

import pytest

from raygeo.driver.ruida import encode_script
from raygeo.ops.convert import GlueScript

GOLDEN = Path(__file__).parent / "golden"


def load(name):
    return (GOLDEN / name).read_text().splitlines()


class TestGoldenCompilation:
    def test_staged_transcript_compiles_to_the_golden_rpascript(self):
        script = GlueScript()
        script.stage_gluescript(load("transcript.cglu"))
        expected = load("job.rds")
        # The generator banner names the implementation; the rest of
        # the controller script is byte-identical.
        assert script.rpascript[2:] == expected[2:]

    def test_compiled_script_encodes_to_the_golden_bytes(self):
        script = GlueScript()
        script.stage_gluescript(load("transcript.cglu"))
        actual = bytes(encode_script(script.rpascript)).hex().upper()
        assert actual == (GOLDEN / "job.hex").read_text().strip()

    def test_replay_reproduces_the_transcript(self):
        transcript = load("transcript.cglu")
        script = GlueScript()
        script.stage_gluescript(transcript)
        assert script.gluescript == transcript


class TestAuthoring:
    def test_authoring_matches_replay(self):
        script = GlueScript()
        script.declare_job("Test", "MACHINE")
        script.declare_layer(
            "Cut", "#ff6600", "VECTOR", "NONE", 5.0, 20.0, 50.0, 50.0
        )
        script.move_xy_to(0.0, 0.0)
        script.power_range(50.0, 50.0)
        script.cut_xy_to(10.0, 10.0)
        script.end_job()
        script.stage_gluescript()
        authored = list(script.rpascript)

        replay = GlueScript()
        replay.stage_gluescript(script.gluescript)
        assert replay.rpascript == authored
        assert (
            bytes(encode_script(authored)).hex()
            == bytes(encode_script(replay.rpascript)).hex()
        )

    def test_cut_speed_and_power_range_flush_before_cut(self):
        script = GlueScript()
        script.declare_job("Test")
        script.declare_layer("Cut", "#ff6600")
        script.move_xy_to(0.0, 0.0)
        script.cut_speed(10.0)
        script.power_range(20.0, 60.0)
        script.cut_xy_to(1.0, 1.0)
        script.end_job()
        script.stage_gluescript()
        assert "CUT_SPEED_LASER_1 Layer:0 Speed=10.0" in script.rpascript
        assert "SELECT_LAYER Layer:0" in script.rpascript
        assert "LAYER_MAX_POWER_1 Layer:0 Power:60.0%" in script.rpascript
        index_speed = script.rpascript.index(
            "CUT_SPEED_LASER_1 Layer:0 Speed=10.0"
        )
        index_cut = next(
            index
            for index, line in enumerate(script.rpascript)
            if line.startswith(("CUT_NEAR", "CUT_FAR"))
        )
        assert index_speed < index_cut

    def test_near_and_far_forms(self):
        script = GlueScript()
        script.declare_job("Test")
        script.declare_layer("Cut", "#ff6600")
        script.move_xy_to(0.0, 0.0)
        script.cut_xy_to(1.0, 1.0)
        script.cut_xy_to(100.0, 100.0)
        script.end_job()
        script.stage_gluescript()
        assert any(
            line.startswith("CUT_NEAR_XY nearX=") for line in script.rpascript
        )
        assert any(
            line.startswith("CUT_FAR_XY X=") for line in script.rpascript
        )

    def test_image_power_emits_imd_power(self):
        script = GlueScript()
        script.declare_job("Test")
        script.declare_layer("Image", "#00ccff", "IMAGE", "X_BI")
        script.move_xy_to(0.0, 0.0)
        script.power(50.0)
        script.cut_xy_to(1.0, 0.0)
        script.end_job()
        script.stage_gluescript()
        assert "IMD_POWER_1 Power:50.0%" in script.rpascript
        assert "IMD_POWER_3 Power:50.0%" in script.rpascript

    def test_set_mode_and_overscan(self):
        script = GlueScript()
        script.declare_job("Test")
        script.declare_layer("Layer", "#ff6600")
        script.set_mode("IMAGE")
        script.set_overscan("X_BI")
        script.end_job()
        script.stage_gluescript()
        assert "OVERSCAN_H_BI" in script.rpascript

    def test_air_assist_and_laser(self):
        script = GlueScript()
        script.declare_job("Test")
        script.declare_layer("Layer", "#ff6600")
        script.select_laser(1)
        script.air_assist_on()
        script.air_assist_off()
        script.end_job()
        script.stage_gluescript()
        assert "LASER_DEVICE_1" in script.rpascript
        assert "AIR_ASSIST_ON" in script.rpascript
        assert "AIR_ASSIST_OFF" in script.rpascript

    def test_comment_only_actions(self):
        script = GlueScript()
        script.declare_job("Test")
        script.declare_layer("Layer", "#ff6600")
        script.move_speed(100.0)
        script.pwm(50.0)
        script.end_job()
        script.stage_gluescript()
        assert "# move_speed(100.0)" in script.rpascript
        assert "# pwm(50.0)" in script.rpascript


class TestValidation:
    def test_invalid_reference_point(self):
        script = GlueScript()
        with pytest.raises(ValueError, match="reference point"):
            script.declare_job("Test", "NOWHERE")

    def test_invalid_mode(self):
        script = GlueScript()
        script.declare_job("Test")
        with pytest.raises(ValueError, match="layer mode"):
            script.declare_layer("Layer", "#ff6600", "NOPE")

    def test_invalid_overscan(self):
        script = GlueScript()
        script.declare_job("Test")
        with pytest.raises(ValueError, match="overscan"):
            script.declare_layer("Layer", "#ff6600", "VECTOR", "NOPE")

    def test_stage_requires_end_job(self):
        script = GlueScript()
        script.declare_job("Test")
        with pytest.raises(RuntimeError, match="end_job"):
            script.stage_gluescript()

    def test_end_job_twice(self):
        script = GlueScript()
        script.declare_job("Test")
        script.end_job()
        with pytest.raises(ValueError, match="twice"):
            script.end_job()

    def test_power_range_requires_layer(self):
        script = GlueScript()
        script.declare_job("Test")
        with pytest.raises(ValueError, match="declared layer"):
            script.power_range(10.0, 50.0)

    def test_unknown_transcript_command(self):
        script = GlueScript()
        with pytest.raises(RuntimeError, match="unknown gluescript"):
            script.stage_gluescript(["not_a_command(1)"])
