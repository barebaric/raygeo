"""Generate visualisations of profile workplan execution."""

import matplotlib.pyplot as plt

from raygeo.cnc.execution.intent import create_intent, run_intent
from raygeo.cnc.plan.profile import plan_profile
from raygeo.geo import Geometry
from raygeo.ops.part import Part
from tools.plot import plot_ops_2d, plot_ops_3d


def _circle(r, cx=0.0, cy=0.0):
    g = Geometry()
    g.move_to(cx + r, cy, 0)
    g.arc_to(cx - r, cy, -r, 0, False, 0)
    g.arc_to(cx + r, cy, r, 0, False, 0)
    return g


def _build_and_execute(part, **kwargs):
    plan = plan_profile(part, **kwargs)
    intent = create_intent(plan, part, 0)
    ops = run_intent(intent)
    return plan, ops


def generate_plan_profile():
    """plan_profile on a circular part — exact inner contour."""
    part = Part(geometry=_circle(30.0), size_mm=(80.0, 80.0))
    plan, ops = _build_and_execute(
        part,
        side="inner",
        tool_radius=3.0,
        feed_rate=1200.0,
    )

    fig, ax = plt.subplots(figsize=(8, 8))
    boundary = [(p[0], p[1]) for p in _circle(30.0).to_polygons(0.05)[0]]
    plot_ops_2d(ax, ops, boundary=boundary)
    ax.set_title(
        f"plan_profile — {plan.step_count} step, exact inner contour "
        "(arcs at R − tool radius)"
    )
    fig.tight_layout()
    return fig


def generate_plan_profile_ramp_entry():
    """plan_profile with a ramp entry — the tool descends along the
    profile loop (helical arcs) before the wall pass."""
    part = Part(geometry=_circle(30.0), size_mm=(80.0, 80.0))
    plan, ops = _build_and_execute(
        part,
        side="inner",
        tool_radius=3.0,
        ramp_angle_deg=15.0,
        feed_rate=1200.0,
        plunge_feed=400.0,
    )
    # The 3D plot draws straight segments; linearize so the helical arcs
    # show as the curves they are.
    ops.linearize_all()

    fig = plt.figure(figsize=(9, 8))
    ax = fig.add_subplot(111, projection="3d")
    plot_ops_3d(ax, ops)
    ax.set_title(
        f"plan_profile — {plan.step_count} steps: ramp entry along the "
        "loop, then the exact wall pass"
    )
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.cnc.plan.profile.md"]

__images__ = [
    {
        "heading": "plan_profile",
        "caption": (
            "Profile workplan: one exact inner contour step on a "
            "circular part, cut in true arcs."
        ),
        "function": generate_plan_profile,
    },
    {
        "heading": "plan_profile",
        "caption": (
            "Profile workplan with a ramp entry: the tool descends along "
            "the loop before the wall pass."
        ),
        "function": generate_plan_profile_ramp_entry,
    },
]
