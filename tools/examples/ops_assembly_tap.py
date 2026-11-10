"""Generate visualisations of the rigid-tapping assembly.

Tapping is purely axial, so a 3D toolpath plot collapses into a
vertical line. These figures instead show (a) the *emitted* cycle as a
sawtooth Z profile over cumulative axial travel — synchronized descents
and retracts vs. the rapid re-entries of pecked tapping — and (b) an
XZ cross-section of the pilot bore with the thread flanks the tap
forms, proving depth, pitch and the bottom reversal.
"""

import math

import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.patches import Polygon, Rectangle

from raygeo.ops.assembly.tap import generate_tap
from raygeo.ops.part import Part
from raygeo.ops.state import State

FEED = "#d62728"
RETRACT = "#9467bd"
RAPID = "#1f77b4"
TOOL_EDGE = "#444444"


def _axial_moves(ops):
    """``(kind, z)`` for every axial move of a cycle.

    Extracted from the emitted ops, so the figures show what the
    assembler actually produced.
    """
    return [
        ("rapid" if c["type"] == "MOVE_TO" else "feed", c["end"][2])
        for c in ops.to_dict()["commands"]
        if c["type"] in ("MOVE_TO", "LINE_TO")
    ]


def _plot_cycle_profile(ax, moves, rapid_z, safe_z):
    """Sawtooth Z profile over cumulative axial travel.

    Synchronized moves (descents and retracts under an active sync
    pitch) are solid; rapid re-entries and the final lift are dashed.
    """
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
        # A synchronized retract climbs, a descent goes down.
        color = RAPID if rapid else (RETRACT if zs[i + 1] > zs[i] else FEED)
        ax.plot(
            xs[i : i + 2],
            zs[i : i + 2],
            color=color,
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
                color=color,
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
    ax.set_title("emitted cycle profile")
    ax.legend(
        handles=[
            Line2D([], [], color=FEED, lw=2, label="sync descent"),
            Line2D([], [], color=RETRACT, lw=2, label="sync retract"),
            Line2D([], [], color=RAPID, lw=2, ls="--", label="rapid"),
        ],
        loc="lower left",
        fontsize=8,
    )


def _peck_bottoms(z_surface, target_z, increment):
    """Peck bottoms as the assembler computes them."""
    n = math.ceil((z_surface - target_z) / increment)
    return [max(z_surface - increment * (k + 1), target_z) for k in range(n)]


def _draw_tap_section(
    ax,
    nominal_diameter,
    tap_drill_diameter,
    target_z,
    pitch,
    bottoms=(),
    z_surface=0.0,
    shaft_top=7.0,
):
    """Cross-section: pilot bore, formed thread flanks, tap ghost."""
    r_major = nominal_diameter / 2.0
    r_bore = tap_drill_diameter / 2.0
    thread_depth = r_major - r_bore
    x_lim = r_major + 4.0
    z_floor = target_z - 1.5

    # Material (grey) with the pilot bore punched out.
    ax.add_patch(
        Rectangle(
            (-x_lim, z_floor),
            2 * x_lim,
            shaft_top - z_floor,
            facecolor="#e8e8e8",
            edgecolor="none",
        )
    )
    ax.add_patch(
        Rectangle(
            (-r_bore, target_z),
            2 * r_bore,
            z_surface - target_z,
            facecolor="white",
            edgecolor="none",
        )
    )

    # Thread flanks: V-grooves of one pitch engaged with the walls.
    n_starts = max(1, int((z_surface - target_z) / pitch))
    for side in (-1.0, 1.0):
        sign = 1.0 if side > 0 else -1.0
        ax.plot(
            [side * r_bore, side * r_bore],
            [target_z, z_surface],
            color="black",
            lw=1.2,
        )
        for k in range(n_starts):
            z0 = z_surface - k * pitch
            ax.plot(
                [side * r_bore, side * r_major],
                [z0, z0 - pitch / 2.0],
                color="black",
                lw=0.9,
            )
            ax.plot(
                [side * r_major, side * r_bore],
                [z0 - pitch / 2.0, z0 - pitch],
                color="black",
                lw=0.9,
            )
        if thread_depth > 0:
            ax.annotate(
                f"thread ⌀{nominal_diameter:g}",
                (side * r_major, z_surface - 1.5 * pitch),
                xytext=(sign * (x_lim - 0.2), z_surface - 1.5 * pitch),
                fontsize=8,
                ha="right" if sign > 0 else "left",
                va="center",
                color=TOOL_EDGE,
                arrowprops=dict(arrowstyle="-", alpha=0.5, lw=0.7),
            )

    ax.plot([-x_lim, x_lim], [z_surface, z_surface], color="black", lw=0.8)

    # Tap ghost per peck bottom: a full-diameter cylinder with a small
    # chamfered lead — a tap has no conical drill tip.
    for i, bottom in enumerate(bottoms):
        ax.add_patch(
            Polygon(
                [
                    (-r_major * 0.9, bottom + 0.6),
                    (0.0, bottom),
                    (r_major * 0.9, bottom + 0.6),
                    (r_major, bottom + 0.6),
                    (r_major, shaft_top),
                    (-r_major, shaft_top),
                    (-r_major, bottom + 0.6),
                ],
                closed=True,
                facecolor=FEED,
                alpha=0.10,
                edgecolor=TOOL_EDGE,
                lw=1.0,
            )
        )
        if len(bottoms) > 1:
            ax.annotate(
                f"peck {i + 1}",
                (0.0, bottom),
                xytext=(r_major + 1.2, bottom),
                fontsize=8,
                va="center",
                color=TOOL_EDGE,
                arrowprops=dict(arrowstyle="-", alpha=0.5, lw=0.7),
            )

    ax.set_xlim(-x_lim, x_lim)
    ax.set_ylim(z_floor, shaft_top + 0.5)
    ax.set_aspect("equal")
    title = (
        "cross-section at the tap bottom"
        if bottoms
        else ("cross-section at the tap bottom (single pass)")
    )
    ax.set_title(title)
    ax.set_xlabel("x (mm)")
    ax.set_ylabel("Z (mm)")


def generate_tap_rigid_cycle():
    """generate_tap — plain rigid cycle: profile and section."""
    part = Part.from_polygons([])
    state = State(feed_rate=1000, spindle_rpm=500)
    result = generate_tap(
        part,
        center=(0.0, 0.0),
        nominal_diameter=6.0,
        pitch=1.0,
        target_z=-12.0,
        dwell_ms=50.0,
        state=state,
    )

    fig, (axp, axs) = plt.subplots(1, 2, figsize=(12, 6.5))
    _plot_cycle_profile(axp, _axial_moves(result.ops), rapid_z=1.0, safe_z=5.0)
    _draw_tap_section(axs, 6.0, 5.0, -12.0, pitch=1.0, bottoms=[-12.0])
    fig.suptitle(
        "generate_tap — rigid cycle: descents and retracts run at "
        "pitch × rpm; the spindle reverses at the bottom",
        fontsize=10,
    )
    fig.tight_layout()
    return fig


def generate_tap_peck_cycle():
    """generate_tap — pecked rigid cycle: profile and section."""
    part = Part.from_polygons([])
    state = State(feed_rate=1000, spindle_rpm=500)
    result = generate_tap(
        part,
        center=(0.0, 0.0),
        nominal_diameter=6.0,
        pitch=1.0,
        target_z=-12.0,
        peck=5.0,
        state=state,
    )
    bottoms = _peck_bottoms(0.0, -12.0, 5.0)

    fig, (axp, axs) = plt.subplots(1, 2, figsize=(12, 6.5))
    _plot_cycle_profile(axp, _axial_moves(result.ops), rapid_z=1.0, safe_z=5.0)
    _draw_tap_section(axs, 6.0, 5.0, -12.0, pitch=1.0, bottoms=bottoms)
    fig.suptitle(
        "generate_tap — pecked cycle: between pecks the tap retracts "
        "fully to the rapid plane and re-enters 0.5 mm above the bottom",
        fontsize=10,
    )
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.tap.md"]

__images__ = [
    {
        "heading": "generate_tap",
        "caption": (
            "Plain rigid cycle: synchronized profile and thread "
            "cross-section; reversal at the bottom."
        ),
        "function": generate_tap_rigid_cycle,
    },
    {
        "heading": "generate_tap",
        "caption": (
            "Pecked cycle: full synchronized retracts between pecks "
            "clear the flutes."
        ),
        "function": generate_tap_peck_cycle,
    },
]
