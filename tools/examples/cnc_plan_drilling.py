"""Generate visualisations of drilling workplan execution."""

import math

import matplotlib.pyplot as plt
from matplotlib.lines import Line2D

from raygeo.cnc.execution.intent import create_intent, run_intent
from raygeo.cnc.plan.drilling import plan_drilling
from raygeo.ops.part import Part
from tools.plot import plot_ops_3d

FEED = "#d62728"
RAPID = "#1f77b4"


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


def _first_cycle_moves(ops):
    """``(kind, z)`` axial moves inside the plan's first drill cycle."""
    moves, inside = [], False
    for c in ops.to_dict()["commands"]:
        if c["type"] == "OPS_SECTION_START":
            inside = True
        elif c["type"] == "OPS_SECTION_END":
            if inside:
                break
        elif inside and c["type"] in ("MOVE_TO", "LINE_TO"):
            kind = "rapid" if c["type"] == "MOVE_TO" else "feed"
            moves.append((kind, c["end"][2]))
    return moves


def _plot_cycle_profile(ax, moves, rapid_z, safe_z):
    """Sawtooth Z profile over cumulative axial travel."""
    xs, zs, kinds = [0.0], [moves[0][1]], ["rapid"]
    dist, prev_z = 0.0, moves[0][1]
    for kind, z in moves[1:]:
        dist += abs(z - prev_z)
        xs.append(dist)
        zs.append(z)
        kinds.append(kind)
        prev_z = z
    for i in range(len(xs) - 1):
        rapid = kinds[i + 1] == "rapid"
        ax.plot(
            xs[i : i + 2],
            zs[i : i + 2],
            color=FEED if not rapid else RAPID,
            ls="--" if rapid else "-",
            lw=2,
        )
        if not rapid:
            ax.annotate(
                f"{zs[i + 1]:.1f}",
                (xs[i + 1], zs[i + 1]),
                xytext=(5, 0),
                textcoords="offset points",
                fontsize=8,
                color=FEED,
                va="center",
            )
    for z, label, style in (
        (safe_z, "safe_z", ":"),
        (rapid_z, "rapid_z", "--"),
        (0.0, "surface", "-"),
    ):
        ax.axhline(z, color="black", ls=style, lw=0.8, alpha=0.5)
        ax.text(
            0.99,
            z,
            label + " ",
            transform=ax.get_yaxis_transform(),
            ha="right",
            va="bottom",
            fontsize=8,
            alpha=0.7,
        )
    ax.set_xlabel("cumulative axial travel (mm)")
    ax.set_ylabel("Z (mm)")
    ax.set_title("first hole: emitted cycle profile")
    ax.legend(
        handles=[
            Line2D([], [], color=FEED, lw=2, label="feed plunge"),
            Line2D([], [], color=RAPID, lw=2, ls="--", label="rapid"),
        ],
        loc="lower left",
        fontsize=8,
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

    fig = plt.figure(figsize=(13, 6.5))
    ax3 = fig.add_subplot(1, 2, 1, projection="3d")
    plot_ops_3d(ax3, ops, islands=[_circle(30, 30, 2.5), _circle(70, 30, 2.5)])
    ax3.set_title(
        f"plan_drilling — {plan.step_count} peck cycles detected on the plate"
    )
    axp = fig.add_subplot(1, 2, 2)
    _plot_cycle_profile(axp, _first_cycle_moves(ops), rapid_z=1.0, safe_z=5.0)
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
            "Peck drilling of two detected holes; right: the "
            "plunge/retract profile of the first cycle."
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
