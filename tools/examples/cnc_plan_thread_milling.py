"""Generate visualisations of thread-milling workplan execution."""

import math

import matplotlib.pyplot as plt

from raygeo.cnc.execution.intent import create_intent, run_intent
from raygeo.cnc.plan.thread_milling import plan_thread_milling
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


def _plate_with_pilot_bores():
    # Two M6 pilot bores (4.2 mm diameter).
    return Part.from_polygons(
        [(0, 0), (100, 0), (100, 60), (0, 60)],
        [_circle(30, 30, 2.1), _circle(70, 30, 2.1)],
        (100.0, 60.0),
    )


def generate_plan_thread_milling():
    """plan_thread_milling on two detected pilot bores."""
    part = _plate_with_pilot_bores()
    plan = plan_thread_milling(
        part,
        nominal_diameter=6.0,
        pitch=1.0,
        tool_diameter=4.0,
        z_top=0.0,
        z_bottom=-12.0,
        feed_rate=800.0,
        plunge_feed=200.0,
        spindle_rpm=3000,
    )
    ops = run_intent(create_intent(plan, part, 0))
    # Linearize so the helical arcs render as the curves they are.
    ops.linearize_all()

    fig = plt.figure(figsize=(9, 8))
    ax = fig.add_subplot(111, projection="3d")
    plot_ops_3d(ax, ops, islands=[_circle(30, 30, 2.1), _circle(70, 30, 2.1)])
    ax.set_title(
        f"plan_thread_milling — M6x1 on {plan.step_count} detected pilot "
        "bores: lead-in arc, helix, lead-out"
    )
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.cnc.plan.thread_milling.md"]

__images__ = [
    {
        "heading": "plan_thread_milling",
        "caption": (
            "Thread milling M6x1 threads on two detected pilot bores; "
            "the helix climbs one pitch per revolution."
        ),
        "function": generate_plan_thread_milling,
    },
]
