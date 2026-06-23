"""Generate examples for fillet operations (create, append, trim)."""

import math

import matplotlib.pyplot as plt

from raygeo.geo.algo.fillet import (
    append_end_fillets,
    create_fillet_polyline,
    fillet_arc_ends,
    find_safe_sweep_end,
    trim_to_safe_fillet_span,
    try_fillet_one_end,
)
from raygeo.geo.shape.arc import get_polyline_turn_sign
from raygeo.geo.shape.polygon import trim_polyline_at


def generate_create_fillet_polyline():
    """Circular fillet arcs at different sweep angles."""
    p = (50, 50)
    dir_ = (1.0, 0.0)
    radius = 20.0
    angles = [math.pi / 4, math.pi / 2, 3 * math.pi / 4, math.pi]
    labels = ["45°", "90°", "135°", "180°"]
    colors = ["#e41a1c", "#377eb8", "#4daf4a", "#984ea3"]

    fig, ax = plt.subplots(figsize=(7, 7))
    for angle, label, color in zip(angles, labels, colors):
        _, polyline = create_fillet_polyline(
            p, dir_, radius, angle, 1.0, False
        )
        xs = [pt[0] for pt in polyline]
        ys = [pt[1] for pt in polyline]
        ax.plot(xs, ys, "-", color=color, linewidth=2, label=f"Sweep {label}")
        ax.plot(xs[0], ys[0], "o", color=color, markersize=6)
        ax.plot(xs[-1], ys[-1], "s", color=color, markersize=6)

    # direction arrow
    ax.arrow(
        p[0],
        p[1],
        dir_[0] * 25,
        dir_[1] * 25,
        head_width=3,
        head_length=3,
        fc="gray",
        ec="gray",
        linestyle="--",
        label="Direction",
    )
    ax.plot(p[0], p[1], "k+", markersize=10)
    ax.set_xlim(10, 100)
    ax.set_ylim(10, 100)
    ax.set_aspect("equal")
    ax.set_title("create_fillet_polyline — sweep angles")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.legend(fontsize=9)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_create_fillet_polyline_side():
    """Fillet arcs on left vs right side of direction."""
    p = (50, 50)
    dir_ = (1.0, 0.0)
    radius = 20.0
    sweep = math.pi / 2

    fig, ax = plt.subplots(figsize=(7, 7))
    _, left = create_fillet_polyline(p, dir_, radius, sweep, 1.0, False)
    _, right = create_fillet_polyline(p, dir_, radius, sweep, -1.0, False)

    lx = [pt[0] for pt in left]
    ly = [pt[1] for pt in left]
    rx = [pt[0] for pt in right]
    ry = [pt[1] for pt in right]
    ax.plot(lx, ly, "-", color="#377eb8", linewidth=2, label="Side +1 (left)")
    ax.plot(rx, ry, "-", color="#e41a1c", linewidth=2, label="Side -1 (right)")

    ax.arrow(
        p[0],
        p[1],
        dir_[0] * 25,
        dir_[1] * 25,
        head_width=3,
        head_length=3,
        fc="gray",
        ec="gray",
        linestyle="--",
        label="Direction",
    )
    ax.plot(p[0], p[1], "k+", markersize=10)
    ax.set_xlim(10, 100)
    ax.set_ylim(10, 100)
    ax.set_aspect("equal")
    ax.set_title("create_fillet_polyline — left vs right side")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.legend(fontsize=9)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_append_end_fillets():
    """Fillets appended to both ends of a polyline."""
    polyline = [(20, 30), (50, 30), (80, 50), (100, 50)]
    radius = 8.0
    sweep = math.pi / 2
    filleted = append_end_fillets(polyline, radius, sweep, 1.0)

    fig, ax = plt.subplots(figsize=(7, 5))
    # original polyline (thick and visible underneath)
    ox = [pt[0] for pt in polyline]
    oy = [pt[1] for pt in polyline]
    ax.plot(
        ox,
        oy,
        "-",
        color="#e41a1c",
        linewidth=3.0,
        alpha=0.7,
        label="Original",
    )
    # filleted result (semi-transparent so original shows through)
    fx = [pt[0] for pt in filleted]
    fy = [pt[1] for pt in filleted]
    ax.plot(
        fx,
        fy,
        "-",
        color="#377eb8",
        linewidth=2.5,
        alpha=0.7,
        label="With fillets",
    )

    ax.plot(polyline[0][0], polyline[0][1], "o", color="gray", markersize=6)
    ax.plot(polyline[-1][0], polyline[-1][1], "s", color="gray", markersize=6)
    ax.set_xlim(0, 120)
    ax.set_ylim(10, 80)
    ax.set_aspect("equal")
    ax.set_title("append_end_fillets — rounded ends on a polyline")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.legend(fontsize=9)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_trim_to_safe_fillet_span():
    """Trimmed fillet span avoiding obstacles."""
    # A bent polyline with an obstacle blocking the start-side fillet
    polyline = [(20, 30), (55, 30), (80, 60)]
    outer = [(0, 0), (100, 0), (100, 80), (0, 80)]
    obstacles = [[(10, 10), (28, 10), (28, 55), (10, 55)]]
    radius = 8.0

    result = trim_to_safe_fillet_span(polyline, outer, obstacles, radius, 0.0)

    fig, ax = plt.subplots(figsize=(7, 5))

    # outer boundary
    ox = [pt[0] for pt in outer] + [outer[0][0]]
    oy = [pt[1] for pt in outer] + [outer[0][1]]
    ax.plot(
        ox, oy, "-", color="lightgray", linewidth=1, label="Outer boundary"
    )

    # obstacles
    for obs in obstacles:
        obsx = [pt[0] for pt in obs] + [obs[0][0]]
        obsy = [pt[1] for pt in obs] + [obs[0][1]]
        ax.fill(obsx, obsy, facecolor="tomato", alpha=0.3, edgecolor="tomato")

    # original polyline (underneath, semi-transparent)
    px = [pt[0] for pt in polyline]
    py = [pt[1] for pt in polyline]
    ax.plot(
        px,
        py,
        "-",
        color="#e41a1c",
        linewidth=3.0,
        alpha=0.5,
        label="Original polyline",
    )

    if result is not None:
        enter, exit_ = result
        # trim the polyline to the safe span and add end fillets
        trimmed = trim_polyline_at(polyline, enter, exit_)
        filleted = append_end_fillets(trimmed, radius, math.pi / 2, 1.0)
        fx = [pt[0] for pt in filleted]
        fy = [pt[1] for pt in filleted]
        ax.plot(
            fx,
            fy,
            "-",
            color="#377eb8",
            linewidth=2.5,
            alpha=0.9,
            label="Safe filleted span",
        )
        ax.plot(
            enter[0],
            enter[1],
            "o",
            color="#4daf4a",
            markersize=8,
            label="Enter",
        )
        ax.plot(
            exit_[0],
            exit_[1],
            "s",
            color="#e41a1c",
            markersize=8,
            label="Exit",
        )

    ax.set_xlim(-5, 105)
    ax.set_ylim(-5, 85)
    ax.set_aspect("equal")
    ax.set_title("trim_to_safe_fillet_span — obstacle blocks start fillet")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_fillet_arc_ends():
    """Filleted arc inside a pocket with an obstacle."""
    arc = [(20, 20), (50, 50), (80, 20)]
    pocket = [(0, 0), (100, 0), (100, 80), (0, 80)]
    islands = [[(5, 5), (28, 5), (28, 38), (5, 38)]]
    tool_radius = 8.0

    result = fillet_arc_ends(arc, pocket, islands, tool_radius, 0.0)

    fig, ax = plt.subplots(figsize=(7, 5))

    # pocket boundary
    px = [pt[0] for pt in pocket] + [pocket[0][0]]
    py = [pt[1] for pt in pocket] + [pocket[0][1]]
    ax.plot(
        px, py, "-", color="lightgray", linewidth=1, label="Pocket boundary"
    )

    # islands
    for obs in islands:
        ox = [pt[0] for pt in obs] + [obs[0][0]]
        oy = [pt[1] for pt in obs] + [obs[0][1]]
        ax.fill(
            ox,
            oy,
            facecolor="gold",
            alpha=0.3,
            edgecolor="gold",
            label="Island",
        )

    # original polyline
    ax.plot(
        [pt[0] for pt in arc],
        [pt[1] for pt in arc],
        "-",
        color="#e41a1c",
        linewidth=3.0,
        alpha=0.5,
        label="Original",
    )

    # filleted result
    if result is not None:
        ax.plot(
            [pt[0] for pt in result],
            [pt[1] for pt in result],
            "-",
            color="#377eb8",
            linewidth=2.5,
            alpha=0.9,
            label="Filleted",
        )

    ax.set_xlim(-5, 105)
    ax.set_ylim(-5, 85)
    ax.set_aspect("equal")
    ax.set_title("fillet_arc_ends — trimmed path with quarter-circle fillets")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_find_safe_sweep_end():
    """Safe sweep end points inside a pocket with an obstacle."""
    arc = [(20, 20), (50, 50), (80, 20)]
    pocket = [(0, 0), (100, 0), (100, 80), (0, 80)]
    islands = [[(5, 5), (28, 5), (28, 38), (5, 38)]]
    tool_radius = 8.0

    result = find_safe_sweep_end(arc, pocket, islands, tool_radius, 0.0)

    fig, ax = plt.subplots(figsize=(7, 5))

    # pocket boundary
    px = [pt[0] for pt in pocket] + [pocket[0][0]]
    py = [pt[1] for pt in pocket] + [pocket[0][1]]
    ax.plot(
        px, py, "-", color="lightgray", linewidth=1, label="Pocket boundary"
    )

    # islands
    for obs in islands:
        ox = [pt[0] for pt in obs] + [obs[0][0]]
        oy = [pt[1] for pt in obs] + [obs[0][1]]
        ax.fill(
            ox,
            oy,
            facecolor="gold",
            alpha=0.3,
            edgecolor="gold",
            label="Island",
        )

    # original polyline
    ax.plot(
        [pt[0] for pt in arc],
        [pt[1] for pt in arc],
        "-",
        color="#e41a1c",
        linewidth=3.0,
        alpha=0.5,
        label="Original",
    )

    if result is not None:
        enter, exit_ = result
        # trimmed arc
        trimmed = trim_polyline_at(arc, enter, exit_)
        ax.plot(
            [pt[0] for pt in trimmed],
            [pt[1] for pt in trimmed],
            "-",
            color="#377eb8",
            linewidth=2.5,
            alpha=0.9,
            label="Safe sub-span",
        )
        ax.plot(
            enter[0],
            enter[1],
            "o",
            color="#4daf4a",
            markersize=8,
            label="Enter",
        )
        ax.plot(
            exit_[0],
            exit_[1],
            "s",
            color="#e41a1c",
            markersize=8,
            label="Exit",
        )

    ax.set_xlim(-5, 105)
    ax.set_ylim(-5, 85)
    ax.set_aspect("equal")
    ax.set_title("find_safe_sweep_end — longest collision-free sub-arc")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_try_fillet_one_end():
    """Single-end fillet fallback when the start fillet would collide.

    The obstacle sits below-left of the start point — the original arc
    never enters it, but the start fillet (which curls backward) would
    sweep through it.  The function rejects the start fillet and falls
    back to the end fillet instead.
    """
    arc = [(20, 50), (50, 80), (80, 50)]
    outer = [(0, 0), (100, 0), (100, 100), (0, 100)]
    obstacle = [(12, 32), (24, 32), (24, 47), (12, 47)]
    obstacles = [obstacle]
    radius = 10.0

    result = try_fillet_one_end(arc, outer, obstacles, radius, 0.0)

    # Ghost: what the start fillet would have been (blocked by obstacle)
    side = get_polyline_turn_sign(arc)
    start_dir = (arc[1][0] - arc[0][0], arc[1][1] - arc[0][1])
    _, ghost_start = create_fillet_polyline(
        arc[0], start_dir, radius, math.pi / 2, side, True
    )

    fig, ax = plt.subplots(figsize=(8, 6))

    ox = [pt[0] for pt in outer] + [outer[0][0]]
    oy = [pt[1] for pt in outer] + [outer[0][1]]
    ax.plot(ox, oy, "-", color="lightgray", linewidth=1, label="Boundary")

    obsx = [pt[0] for pt in obstacle] + [obstacle[0][0]]
    obsy = [pt[1] for pt in obstacle] + [obstacle[0][1]]
    ax.fill(obsx, obsy, facecolor="tomato", alpha=0.3, edgecolor="tomato")

    ax.plot(
        [pt[0] for pt in ghost_start],
        [pt[1] for pt in ghost_start],
        "--",
        color="gray",
        linewidth=1.5,
        alpha=0.6,
        label="Start fillet (blocked)",
    )

    ax.plot(
        [pt[0] for pt in arc],
        [pt[1] for pt in arc],
        "-",
        color="#e41a1c",
        linewidth=3.0,
        alpha=0.5,
        label="Original",
    )
    ax.plot(
        [pt[0] for pt in result],
        [pt[1] for pt in result],
        "-",
        color="#377eb8",
        linewidth=2.5,
        alpha=0.9,
        label="Result (end fillet only)",
    )

    ax.set_xlim(-5, 105)
    ax.set_ylim(20, 95)
    ax.set_aspect("equal")
    ax.set_title("try_fillet_one_end — start fillet blocked, end fillet used")
    ax.legend(fontsize=9, loc="upper center")
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.fillet.md"]
__images__ = [
    {
        "heading": "create_fillet_polyline",
        "caption": (
            "``create_fillet_polyline`` generates circular fillet arcs of"
            " arbitrary sweep angle, tangent to a direction at a point"
        ),
        "function": generate_create_fillet_polyline,
    },
    {
        "heading": "create_fillet_polyline",
        "caption": (
            "``create_fillet_polyline`` with ``side=+1`` (left) and"
            " ``side=-1`` (right) of the direction vector"
        ),
        "function": generate_create_fillet_polyline_side,
    },
    {
        "heading": "append_end_fillets",
        "caption": (
            "``append_end_fillets`` rounds both ends of an open polyline"
            " with reversed-start / forward-end fillet arcs"
        ),
        "function": generate_append_end_fillets,
    },
    {
        "heading": "trim_to_safe_fillet_span",
        "caption": (
            "``trim_to_safe_fillet_span`` finds the longest sub-span whose"
            " end fillets do not collide with obstacles (red)"
        ),
        "function": generate_trim_to_safe_fillet_span,
    },
    {
        "heading": "fillet_arc_ends",
        "caption": (
            "``fillet_arc_ends`` trims the arc to the longest safe sub-arc"
            " and appends quarter-circle fillets at each end"
        ),
        "function": generate_fillet_arc_ends,
    },
    {
        "heading": "find_safe_sweep_end",
        "caption": (
            "``find_safe_sweep_end`` returns the ``(enter, exit)`` points"
            " delimiting the longest sub-arc whose tool sweep avoids islands"
        ),
        "function": generate_find_safe_sweep_end,
    },
    {
        "heading": "try_fillet_one_end",
        "caption": (
            "``try_fillet_one_end`` tests the start fillet first; when it"
            " collides with the obstacle (red), falls back to the end fillet"
        ),
        "function": generate_try_fillet_one_end,
    },
]
