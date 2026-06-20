"""Generate geometry example images — playground + arc-to-bezier."""

import math

import matplotlib.pyplot as plt

from raygeo.geo import Bezier, Geometry
from tools.plot import auto_limits, plot_geometry


def generate_playground():
    # ── Geometry playground (6-panel grid) ───────────────────────────
    fig, axes = plt.subplots(2, 3, figsize=(14, 9))
    axes_flat = axes.flatten()

    cases = [
        ("Rectangle", _make_rect()),
        ("Circle", _make_circle()),
        ("Polygon (regular)", _make_polygon()),
        ("Star", _make_star()),
        ("Grown (offset)", _make_offset()),
        ("Simplified", _make_simplified()),
    ]

    for ax, (title, geom) in zip(axes_flat, cases):
        plot_geometry(ax, geom, color="steelblue", show_points=False)
        xmin, xmax, ymin, ymax = auto_limits([geom])
        ax.set_xlim(xmin, xmax)
        ax.set_ylim(ymin, ymax)
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.set_title(title)

    fig.tight_layout()
    return fig


def generate_arc_to_bezier():
    # ── Arc to Bezier overlay comparison ─────────────────────────────
    arch = _make_arch()
    arch_beziers = arch.copy()
    arch_beziers.convert_arcs_to_beziers()

    circle = _make_circle_geom()
    circle_beziers = circle.copy()
    circle_beziers.convert_arcs_to_beziers()

    fig2, (ax_a, ax_b) = plt.subplots(1, 2, figsize=(16, 7))

    plot_geometry(
        ax_a, arch, color="steelblue", linewidth=3, label="Original arc"
    )
    plot_geometry(
        ax_a,
        arch_beziers,
        color="crimson",
        linewidth=1.5,
        label="Bezier approx",
    )
    _plot_bezier_controls(ax_a, arch_beziers, color="crimson", ms=5)
    ax_a.set_aspect("equal")
    ax_a.grid(True, alpha=0.3)
    ax_a.legend(fontsize=10)
    ax_a.set_title("Arch: arc (blue) + bezier (red) overlay", fontsize=13)
    ax_a.set_xlim(-22, 22)
    ax_a.set_ylim(-27, 5)

    plot_geometry(
        ax_b, circle, color="steelblue", linewidth=3, label="Original arc"
    )
    plot_geometry(
        ax_b,
        circle_beziers,
        color="crimson",
        linewidth=1.5,
        label="Bezier approx",
    )
    _plot_bezier_controls(ax_b, circle_beziers, color="crimson", ms=5)
    ax_b.set_aspect("equal")
    ax_b.grid(True, alpha=0.3)
    ax_b.legend(fontsize=10)
    ax_b.set_title("Circle: arc (blue) + bezier (red) overlay", fontsize=13)
    ax_b.set_xlim(-15, 15)
    ax_b.set_ylim(20, 40)

    fig2.tight_layout()
    return fig2


# ── Geometry playground helpers ──────────────────────────────────────


def _make_rect():
    return Geometry.from_points([(0, 0), (10, 0), (10, 10), (0, 10)])


def _make_circle():
    geom = Geometry()
    r = 10
    geom.move_to(r, 0, 0)
    geom.arc_to(-r, 0, -r, 0, True, 0)
    geom.arc_to(r, 0, r, 0, True, 0)
    return geom


def _make_polygon():
    n = 6
    r = 10
    return Geometry.from_points(
        [
            (
                r * math.cos(2 * math.pi * i / n),
                r * math.sin(2 * math.pi * i / n),
            )
            for i in range(n)
        ]
    )


def _make_star():
    outer_r = 10
    inner_r = 4
    points = 5
    coords = []
    for i in range(points * 2):
        a = math.pi / 2 + math.pi * i / points
        rd = outer_r if i % 2 == 0 else inner_r
        coords.append((rd * math.cos(a), rd * math.sin(a)))
    return Geometry.from_points(coords)


def _make_offset():
    g = Geometry.from_points([(0, 0), (10, 0), (10, 10), (0, 10)])
    return g.grow(2)


def _make_simplified():
    g = Geometry.from_points(
        [
            (0, 0),
            (0.5, 0.01),
            (1, 0),
            (10, 0),
            (10, 10),
            (0, 10),
        ]
    )
    return g.simplify(0.5)


# ── Arc-to-Bezier helpers ────────────────────────────────────────────


def _plot_bezier_controls(ax, geom, color="crimson", ms=4):
    """Draw handle lines and markers for Bezier control points."""
    prev_end = None
    for cmd in geom.iter_typed_commands():
        if isinstance(cmd, Bezier):
            c1, c2 = cmd.control1, cmd.control2
            end = cmd.end
            if prev_end is not None:
                ax.plot(
                    [prev_end[0], c1[0]],
                    [prev_end[1], c1[1]],
                    color=color,
                    linewidth=0.8,
                    linestyle=":",
                )
                ax.scatter(
                    prev_end[0],
                    prev_end[1],
                    color=color,
                    s=ms**2,
                    marker="o",
                    zorder=5,
                )
            ax.plot(
                [c2[0], end[0]],
                [c2[1], end[1]],
                color=color,
                linewidth=0.8,
                linestyle=":",
            )
            ax.scatter(
                [c1[0], c2[0]],
                [c1[1], c2[1]],
                color=color,
                s=(ms + 2) ** 2,
                marker="x",
                linewidths=1.2,
                zorder=6,
            )
            prev_end = end
        elif hasattr(cmd, "end"):
            prev_end = cmd.end


def _make_arch():
    """Continuous path: line + arc + arc + line forming an arch."""
    g = Geometry()
    g.move_to(0, -20, 0)
    g.line_to(-15, -20, 0)
    g.arc_to(0, -5, 0, 15, False, 0)
    g.arc_to(15, -20, 15, 0, False, 0)
    g.line_to(0, -20, 0)
    return g


def _make_circle_geom():
    """Full circle from two semicircles."""
    g = Geometry()
    r = 8
    g.move_to(r, 30, 0)
    g.arc_to(-r, 30, -r, 0, True, 0)
    g.arc_to(r, 30, r, 0, True, 0)
    return g


__docs_target__ = ["raygeo.md", "raygeo.geo.md"]
__images__ = [
    # geometry.py
    {
        "heading": None,
        "caption": "Various geometry shapes and operations",
        "function": generate_playground,
    },
    {
        "heading": "convert_arcs_to_beziers",
        "caption": (
            "Overlay showing Bezier curves (with control points)"
            " closely matching the original arcs"
        ),
        "function": generate_arc_to_bezier,
    },
]
