"""Generate exact line-and-arc offset example images."""

import matplotlib.pyplot as plt

from raygeo.geo import Geometry
from raygeo.geo.algo.offset_exact import offset_exact
from tools.plot import plot_geometry


def _rect(x0, y0, x1, y1, ccw=True):
    g = Geometry()
    pts = (
        [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
        if ccw
        else [(x0, y1), (x1, y1), (x1, y0), (x0, y0)]
    )
    g.move_to(*pts[0], 0)
    for p in pts[1:]:
        g.line_to(*p, 0)
    g.close_path()
    return g


def _circle(r, cx=0.0, cy=0.0):
    g = Geometry()
    g.move_to(cx + r, cy, 0)
    g.arc_to(cx - r, cy, -r, 0.0, False, 0)
    g.arc_to(cx + r, cy, r, 0.0, False, 0)
    return g


def generate_round_joins():
    """Round joins."""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 7))

    rect = _rect(0, 0, 60, 40)
    grown = offset_exact(rect, 8.0, 0.1)
    plot_geometry(ax1, rect, color="black", linewidth=2, label="Boundary")
    plot_geometry(
        ax1, grown[0], color="steelblue", linewidth=2.5, label="offset +8"
    )
    ax1.set_title("Growing: corners roll on exact arcs")
    ax1.set_aspect("equal")
    ax1.legend(fontsize=9)
    ax1.grid(True, alpha=0.3)

    shrunk = offset_exact(rect, -8.0, 0.1)
    plot_geometry(ax2, rect, color="black", linewidth=2, label="Boundary")
    if shrunk:
        plot_geometry(
            ax2, shrunk[0], color="tomato", linewidth=2.5, label="offset -8"
        )
    ax2.set_title("Shrinking: walls stay lines, corners are trimmed")
    ax2.set_aspect("equal")
    ax2.legend(fontsize=9)
    ax2.grid(True, alpha=0.3)

    fig.tight_layout()
    return fig


def generate_island():
    """Island."""
    region = _rect(0, 0, 100, 70)
    region.extend(_rect(35, 25, 65, 45, ccw=False))
    grown = offset_exact(region, 6.0, 0.1)

    fig, ax = plt.subplots(figsize=(8, 7))
    plot_geometry(ax, region, color="black", linewidth=2, label="Boundary")
    colors = ("steelblue", "tomato")
    names = ("Outer +6", "Island -6")
    for i, lp in enumerate(grown):
        plot_geometry(ax, lp, color=colors[i], linewidth=2.5, label=names[i])
    ax.set_title("Islands take part: growing the region shrinks its holes")
    ax.set_aspect("equal")
    ax.legend(fontsize=9, loc="upper left")
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_spline():
    """Spline."""
    b = Geometry()
    b.move_to(0, 0, 0)
    b.line_to(40, 0, 0)
    b.bezier_to(40, 30, 50, 0, 50, 30)
    b.line_to(0, 30, 0)
    b.close_path()
    grown = offset_exact(b, 3.0, 0.05)

    fig, ax = plt.subplots(figsize=(8, 6))
    plot_geometry(ax, b, color="black", linewidth=2, label="Boundary")
    plot_geometry(
        ax, grown[0], color="steelblue", linewidth=2.5, label="offset +3"
    )
    ax.set_title("Spline boundaries: line chains mitered within tolerance")
    ax.set_aspect("equal")
    ax.legend(fontsize=9)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.offset_exact.md"]
__images__ = [
    {
        "heading": "offset_exact",
        "caption": (
            "Exact offsets of a rectangle: growing rolls the corners on "
            "arcs, shrinking keeps the walls straight."
        ),
        "function": generate_round_joins,
    },
    {
        "heading": "offset_exact",
        "caption": (
            "Every closed subpath takes part: growing the region moves the "
            "island's boundary inwards."
        ),
        "function": generate_island,
    },
    {
        "heading": "offset_exact",
        "caption": (
            "A spline boundary is followed as a line chain, mitered while "
            "that stays within the tolerance."
        ),
        "function": generate_spline,
    },
]
