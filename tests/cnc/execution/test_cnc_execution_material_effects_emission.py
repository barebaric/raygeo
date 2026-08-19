"""Material-effect emission tests through the execution pipeline.

Verifies that the adaptive, profile, and contour assemblers emit
vector effects into AssemblyOutput.material_effects, that cache
store/restore preserves them, and that multi-face parts concatenate
effects across faces.
"""

from conftest import compute_result, make_square_part

from raygeo.cnc.execution.specs import ComputePayload
from raygeo.geo import Geometry
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.adaptive import AdaptiveClearingSpec
from raygeo.ops.assembly.contour import ContourSpec
from raygeo.ops.assembly.profile import ProfileSpec
from raygeo.ops.material import VectorEffect
from raygeo.ops.part import Part
from raygeo.pipeline.completed import CompletedNode
from raygeo.pipeline.execute import Pipeline, execute_stages
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec


def _node(key, assembler, part, face_id=""):
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Compute(
            part=part,
            params=ComputePayload(assembler=Assembler(assembler)),
            face_id=face_id,
        ),
    )


def _adaptive_part():
    boundary = [(-20.0, -20.0), (20.0, -20.0), (20.0, 20.0), (-20.0, 20.0)]
    seed = [[(-5.0, -5.0), (5.0, -5.0), (5.0, 5.0), (-5.0, 5.0)]]
    return Part.from_polygons(boundary, initial=seed)


def _run_once(nodes):
    completed: list[CompletedNode] = []
    execute_stages(nodes, completed.append, None)
    return {c.key: c for c in completed}


def test_adaptive_emits_vector_effect_with_target_z():
    spec = AdaptiveClearingSpec(tool_radius=1.0, target_z=-2.0)
    node = _node("adaptive", spec, _adaptive_part())
    output = compute_result(_run_once([node])["adaptive"])
    assert output.material_effects is not None
    effects = output.material_effects
    assert len(effects) >= 1
    assert all(isinstance(e, VectorEffect) for e in effects)
    assert all(e.z_to == -2.0 for e in effects)
    assert all(e.z_from is None for e in effects)
    assert any(len(e.polygons) > 0 for e in effects)


def test_contour_emits_full_through_effect():
    node = _node("contour", ContourSpec(), make_square_part())
    output = compute_result(_run_once([node])["contour"])
    assert output.material_effects is not None
    effects = output.material_effects
    assert len(effects) >= 1
    assert all(isinstance(e, VectorEffect) for e in effects)
    assert all(e.z_from is None and e.z_to is None for e in effects)
    assert all(len(e.polygons) > 0 for e in effects)


def test_profile_emits_vector_effect_with_target_z():
    spec = ProfileSpec(kind="outer", tool_radius=1.0, target_z=-3.0)
    node = _node("profile", spec, _adaptive_part())
    output = compute_result(_run_once([node])["profile"])
    assert output.material_effects is not None
    effects = output.material_effects
    assert len(effects) >= 1
    assert all(isinstance(e, VectorEffect) for e in effects)
    assert all(e.z_to == -3.0 for e in effects)


def test_cache_restore_preserves_effects():
    p = Pipeline()
    spec = AdaptiveClearingSpec(tool_radius=1.0, target_z=-2.0)
    node = _node("adaptive", spec, _adaptive_part())

    completed: list[CompletedNode] = []
    p.execute([node], completed.append, None)
    first = compute_result(next(c for c in completed if c.key == "adaptive"))
    assert first.material_effects is not None
    n_first = len(first.material_effects)

    completed.clear()
    node_again = _node("adaptive", spec, _adaptive_part())
    p.execute([node_again], completed.append, None)
    second = compute_result(next(c for c in completed if c.key == "adaptive"))
    assert second.material_effects is not None
    assert len(second.material_effects) == n_first
    assert all(e.z_to == -2.0 for e in second.material_effects)


def _two_face_part():
    g = Geometry()
    g.move_to(0, 0)
    g.line_to(10, 0)
    g.line_to(10, 10)
    g.line_to(0, 10)
    g.close_path()
    g.move_to(50, 0)
    g.line_to(60, 0)
    g.line_to(60, 10)
    g.line_to(50, 10)
    g.close_path()
    return Part.from_geometry_multi_face(g, size_mm=(70.0, 10.0))


def test_multi_face_part_concatenates_effects():
    node = _node("contour", ContourSpec(), _two_face_part())
    output = compute_result(_run_once([node])["contour"])
    assert output.material_effects is not None
    effects = output.material_effects
    assert len(effects) >= 2
    assert all(isinstance(e, VectorEffect) for e in effects)
