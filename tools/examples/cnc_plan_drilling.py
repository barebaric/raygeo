"""Generate visualisations of drilling workplan execution."""

import math

import matplotlib.pyplot as plt

from raygeo.cnc.execution.intent import create_intent, run_intent
from raygeo.cnc.plan.drilling import plan_drilling
from raygeo.ops.part import Part
from tools.plot import plot_ops_3d


def _circle(cx, cy, r, n=96):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def _plate_with_holes():
    return Part.from_polygons(
        [(0, 0), (100, 0), (100, 60), (0, 60)],
        [_circle(30, 30, 2.5), _circle(70, 30, 2.5)],
        (100.0, 60.0),
    )


def generate_plan_drilling_peck():
    """plan_drilling with peck cycles on detected holes."""
    part = _plate_with_holes()
    plan = plan_drilling(
        part,
        cycle="peck",
        diameter=5.0,
        target_z=-12.0,
        peck_increment=4.0,
        feed_rate=1000.0,
        plunge_feed=300.0,
        spindle_rpm=4000,
    )
    ops = run_intent(create_intent(plan, part, 0))

    fig = plt.figure(figsize=(9, 8))
    ax = fig.add_subplot(111, projection="3d")
    plot_ops_3d(ax, ops, islands=[_circle(30, 30, 2.5), _circle(70, 30, 2.5)])
    ax.set_title(
        f"plan_drilling — {plan.step_count} peck cycles detected on the "
        "plate (retract per peck)"
    )
    fig.tight_layout()
    return fig


def generate_plan_drilling_explicit_centers():
    """plan_drilling with explicit centers, drilling solid material."""
    part = _plate_with_holes()
    plan = plan_drilling(
        part,
        holes=[(15.0, 15.0), (50.0, 15.0), (85.0, 15.0)],
        diameter=3.0,
        target_z=-8.0,
        feed_rate=800.0,
        plunge_feed=250.0,
    )
    ops = run_intent(create_intent(plan, part, 0))

    fig = plt.figure(figsize=(9, 8))
    ax = fig.add_subplot(111, projection="3d")
    plot_ops_3d(ax, ops)
    ax.set_title(
        f"plan_drilling — {plan.step_count} plain cycles from explicit "
        "centers, safe-Z travel between holes"
    )
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.cnc.plan.drilling.md"]

__images__ = [
    {
        "heading": "plan_drilling",
        "caption": (
            "Peck drilling of two holes detected as circular islands; "
            "each peck retracts to the rapid plane."
        ),
        "function": generate_plan_drilling_peck,
    },
    {
        "heading": "plan_drilling",
        "caption": (
            "Plain cycles at explicit centers in solid material with "
            "safe-Z travel between holes."
        ),
        "function": generate_plan_drilling_explicit_centers,
    },
]
