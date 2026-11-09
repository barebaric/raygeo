"""Generate visualisations of drill cycle assembly.

Drill cycles are purely axial, so a 3D toolpath plot collapses into a
vertical line. These figures instead show (a) the *emitted* cycle as a
sawtooth Z profile over cumulative axial travel — feed plunges vs.
rapid retracts — and (b) an XZ cross-section of the hole with ghosted
tool silhouettes, proving depth, tip compensation and (for the
countersink) the chamfer diameter.
"""

import math

import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.patches import Polygon, Rectangle

from raygeo.ops.assembly.drill import (
    PeckParams,
    generate_countersink,
    generate_drill,
)
from raygeo.ops.part import Part
from raygeo.ops.state import State

FEED = "#d62728"
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


def _plot_cycle_profile(ax, moves, z_surface, rapid_z, safe_z):
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
            color=RAPID if rapid else FEED,
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
        (z_surface, "surface", "-"),
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
            Line2D([], [], color=FEED, lw=2, label="feed plunge"),
            Line2D([], [], color=RAPID, lw=2, ls="--", label="rapid"),
        ],
        loc="lower left",
        fontsize=8,
    )


def _peck_bottoms(z_surface, target_z, tip_angle_deg, diameter, increment):
    """Peck bottoms as the assembler computes them."""
    tip_len = diameter / 2.0 / math.tan(math.radians(tip_angle_deg) / 2.0)
    tip_z = target_z - tip_len
    n = math.ceil((z_surface - tip_z) / increment)
    return [
        max(z_surface - increment * (k + 1), tip_z) for k in range(n)
    ], tip_z


def _draw_tool_ghost(ax, diameter, tip_angle_deg, tip_z, shaft_top):
    """Translucent tool silhouette: shaft + conical tip at ``tip_z``."""
    r = diameter / 2.0
    tip_len = r / math.tan(math.radians(tip_angle_deg) / 2.0)
    cone_start = tip_z + tip_len
    ax.add_patch(
        Polygon(
            [
                (-r, cone_start),
                (0.0, tip_z),
                (r, cone_start),
                (r, shaft_top),
                (-r, shaft_top),
            ],
            closed=True,
            facecolor=FEED,
            alpha=0.10,
            edgecolor=TOOL_EDGE,
            lw=1.0,
        )
    )


def _draw_drill_section(
    ax,
    diameter,
    tip_angle_deg,
    target_z,
    bottoms,
    z_surface=0.0,
    shaft_top=7.0,
):
    """Hole cross-section: walls, conical floor, tool ghosts per peck."""
    r = diameter / 2.0
    tip_len = r / math.tan(math.radians(tip_angle_deg) / 2.0)
    tip_z = target_z - tip_len
    x_lim = r + 4.0
    z_floor = target_z - tip_len - 1.0

    # Material (grey) with the hole cross-section (white) punched out.
    ax.add_patch(
        Rectangle(
            (-x_lim, z_surface),
            2 * x_lim,
            z_floor - z_surface,
            facecolor="#e8e8e8",
            edgecolor="none",
        )
    )
    ax.add_patch(
        Polygon(
            [
                (-r, z_surface),
                (r, z_surface),
                (r, target_z),
                (0.0, tip_z),
                (-r, target_z),
            ],
            closed=True,
            facecolor="white",
            edgecolor="none",
        )
    )

    ax.plot([-r, -r], [z_surface, target_z], color="black", lw=1.2)
    ax.plot([r, r], [z_surface, target_z], color="black", lw=1.2)
    ax.plot([-r, 0.0], [target_z, tip_z], color="black", lw=1.2)
    ax.plot([r, 0.0], [target_z, tip_z], color="black", lw=1.2)
    ax.axhline(z_surface, color="black", lw=0.8, alpha=0.5)

    for i, bottom in enumerate(bottoms):
        _draw_tool_ghost(ax, diameter, tip_angle_deg, bottom, shaft_top)
        ax.annotate(
            f"peck {i + 1}",
            (0.0, bottom),
            xytext=(r + 1.2, bottom),
            fontsize=8,
            va="center",
            color=TOOL_EDGE,
            arrowprops=dict(arrowstyle="-", alpha=0.5, lw=0.7),
        )

    ax.set_xlim(-x_lim, x_lim)
    ax.set_ylim(z_floor, shaft_top + 0.5)
    ax.set_aspect("equal")
    ax.set_title("cross-section at the peck bottoms")
    ax.set_xlabel("x (mm)")
    ax.set_ylabel("Z (mm)")


def generate_drill_peck_cycle():
    """generate_drill — full-retract pecking: profile and section."""
    part = Part.from_polygons([])
    state = State(feed_rate=1000, plunge_feed=300, spindle_rpm=4000)
    result = generate_drill(
        part,
        center=(0.0, 0.0),
        diameter=5.0,
        target_z=-12.0,
        peck=PeckParams(increment=5.0),
        dwell_ms=100.0,
        state=state,
    )
    bottoms, _ = _peck_bottoms(0.0, -12.0, 118.0, 5.0, 5.0)

    fig, (axp, axs) = plt.subplots(1, 2, figsize=(12, 6.5))
    _plot_cycle_profile(
        axp, _axial_moves(result.ops), 0.0, rapid_z=1.0, safe_z=5.0
    )
    _draw_drill_section(axs, 5.0, 118.0, -12.0, bottoms)
    fig.suptitle(
        "generate_drill — peck cycle: every peck retracts to the rapid "
        "plane and re-enters 0.5 mm above the previous bottom",
        fontsize=10,
    )
    fig.tight_layout()
    return fig


def generate_drill_chip_break_cycle():
    """generate_drill — chip-break pecking: profile and section."""
    part = Part.from_polygons([])
    state = State(feed_rate=1000, plunge_feed=300, spindle_rpm=4000)
    result = generate_drill(
        part,
        center=(0.0, 0.0),
        diameter=5.0,
        target_z=-12.0,
        tip_angle=90.0,
        peck=PeckParams(increment=4.0, mode="CHIP_BREAK", retract_amount=0.5),
        state=state,
    )
    bottoms, _ = _peck_bottoms(0.0, -12.0, 90.0, 5.0, 4.0)

    fig, (axp, axs) = plt.subplots(1, 2, figsize=(12, 6.5))
    _plot_cycle_profile(
        axp, _axial_moves(result.ops), 0.0, rapid_z=1.0, safe_z=5.0
    )
    _draw_drill_section(axs, 5.0, 90.0, -12.0, bottoms)
    fig.suptitle(
        "generate_drill — chip-break cycle: the tool only lifts 0.5 mm "
        "between pecks and never leaves the hole",
        fontsize=10,
    )
    fig.tight_layout()
    return fig


def _draw_countersink_section(
    ax,
    tip_angle_deg,
    target_diameter,
    bore_diameter,
    z_surface=0.0,
    shaft_top=5.0,
):
    """Cross-section proving the chamfer: the cone flank meets the
    surface exactly at ``target_diameter``; the removed wedge is
    shaded."""
    half = math.radians(tip_angle_deg) / 2.0
    depth = (target_diameter / 2.0) / math.tan(half)
    r_cham = target_diameter / 2.0
    r_bore = bore_diameter / 2.0
    x_lim = r_cham + 4.5
    z_floor = -1.6 * depth
    # Depth where the cone reaches the bore wall.
    z_int = z_surface - (depth - r_bore / math.tan(half))

    # Material (grey) with the machined cross-section (white) punched
    # out: chamfer flanks opening to the target diameter at the
    # surface, bore walls below.
    ax.add_patch(
        Rectangle(
            (-x_lim, z_surface),
            2 * x_lim,
            z_floor - z_surface,
            facecolor="#e8e8e8",
            edgecolor="none",
        )
    )
    ax.add_patch(
        Polygon(
            [
                (-r_cham, z_surface),
                (r_cham, z_surface),
                (r_bore, z_int),
                (r_bore, z_floor),
                (-r_bore, z_floor),
                (-r_bore, z_int),
            ],
            closed=True,
            facecolor="white",
            edgecolor="none",
        )
    )

    # Stock surface, existing bore walls, chamfer walls.
    ax.plot(
        [-x_lim, x_lim],
        [z_surface, z_surface],
        color="black",
        lw=0.8,
    )
    for side in (-1.0, 1.0):
        ax.plot(
            [side * r_bore, side * r_bore],
            [z_int, z_floor],
            color="black",
            lw=1.2,
        )
        ax.plot(
            [side * r_bore, side * r_cham],
            [z_int, z_surface],
            color="black",
            lw=1.4,
        )
        # Removed wedge between old bore wall and chamfer flank.
        ax.add_patch(
            Polygon(
                [
                    (side * r_bore, z_surface),
                    (side * r_cham, z_surface),
                    (side * r_bore, z_int),
                ],
                closed=True,
                facecolor=FEED,
                alpha=0.25,
                lw=0,
            )
        )

    # Tool silhouette at final depth: flanks pass through (±r_cham, 0).
    ax.add_patch(
        Polygon(
            [
                (-r_cham, z_surface),
                (0.0, z_surface - depth),
                (r_cham, z_surface),
                (r_cham, shaft_top),
                (-r_cham, shaft_top),
            ],
            closed=True,
            facecolor=FEED,
            alpha=0.10,
            edgecolor=TOOL_EDGE,
            lw=1.0,
        )
    )

    # Diameter dimension at the surface.
    y_dim = 0.6
    ax.annotate(
        "",
        xy=(-r_cham, y_dim),
        xytext=(r_cham, y_dim),
        arrowprops=dict(arrowstyle="<->", lw=1.0),
    )
    ax.text(
        0.0,
        y_dim + 0.35,
        f"⌀{target_diameter:g} at the surface",
        ha="center",
        fontsize=9,
    )
    ax.annotate(
        f"⌀{bore_diameter:g} bore",
        (r_bore, z_int),
        xytext=(r_cham + 1.2, -depth / 2.0),
        fontsize=8,
        color=TOOL_EDGE,
        arrowprops=dict(arrowstyle="-", alpha=0.5, lw=0.7),
    )

    ax.set_xlim(-x_lim, x_lim)
    ax.set_ylim(z_floor, shaft_top + 1.2)
    ax.set_aspect("equal")
    ax.set_xlabel("x (mm)")
    ax.set_ylabel("Z (mm)")
    ax.set_title(f"cross-section, cone tip at Z = {z_surface - depth:.1f}")


def generate_countersink_cycle():
    """generate_countersink — the chamfer geometry, in section."""
    part = Part.from_polygons([])
    state = State(feed_rate=800, plunge_feed=250, spindle_rpm=3000)
    result = generate_countersink(
        part,
        center=(0.0, 0.0),
        tip_angle=90.0,
        target_diameter=8.0,
        state=state,
    )
    # Placed on a Ø5 bore, as in real use: the tool is shown at the
    # depth the emitted cycle plunged to.
    plunge_z = [z for kind, z in _axial_moves(result.ops) if kind == "feed"][0]

    fig, ax = plt.subplots(figsize=(8.5, 6.5))
    _draw_countersink_section(ax, 90.0, 8.0, bore_diameter=5.0)
    fig.suptitle(
        "generate_countersink — a 90° cone plunged to Z="
        f"{plunge_z:.1f} opens the Ø5 bore to Ø8 at the surface",
        fontsize=10,
    )
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.drill.md"]

__images__ = [
    {
        "heading": "generate_drill",
        "caption": (
            "Peck cycle: emitted plunge/retract profile (left) and "
            "cross-section with tool ghosts (right)."
        ),
        "function": generate_drill_peck_cycle,
    },
    {
        "heading": "generate_drill",
        "caption": (
            "Chip-break cycle: profile and cross-section; the tool "
            "never leaves the hole between pecks."
        ),
        "function": generate_drill_chip_break_cycle,
    },
    {
        "heading": "generate_countersink",
        "caption": (
            "Countersink cross-section: the cone meets the surface "
            "exactly at the target diameter."
        ),
        "function": generate_countersink_cycle,
    },
]
