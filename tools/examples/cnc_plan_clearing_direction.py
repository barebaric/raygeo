"""Generate cut-direction visualisations: climb vs conventional clearing."""

import math

import matplotlib.pyplot as plt

from raygeo.cnc.execution.intent import create_intent, run_intent
from raygeo.cnc.plan.clearing import plan_clearing
from raygeo.ops.part import Part
from tools.plot import plot_ops_2d


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _floor_polylines(ops, target_z, tol=0.05):
    """Floor-level cutting polylines, in travel order.

    Each polyline is one continuous cut: travel moves, plunges and depth
    changes break it.  Arcs are sampled into short chords so direction
    arrows follow the arc tangent instead of the chord.
    """
    polylines = []
    current: list[tuple[float, float]] = []
    prev: tuple[float, float] | None = None
    for cmd in ops.to_dict()["commands"]:
        if cmd["type"] not in ("LINE_TO", "ARC_TO"):
            # Travel, dwell, state change: the cut is interrupted.
            if current:
                polylines.append(current)
                current = []
            prev = None
            continue
        end = cmd["end"]
        if end[2] > target_z + tol:
            # A cut above the floor (entry, retract): chain breaks.
            if current:
                polylines.append(current)
                current = []
            prev = None
            continue
        cur = (end[0], end[1])
        if prev is None:
            current = [cur]
            prev = cur
            continue
        if cmd["type"] == "ARC_TO":
            cx = prev[0] + cmd["center_offset"][0]
            cy = prev[1] + cmd["center_offset"][1]
            radius = ((cur[0] - cx) ** 2 + (cur[1] - cy) ** 2) ** 0.5
            a0 = math.atan2(prev[1] - cy, prev[0] - cx)
            a1 = math.atan2(cur[1] - cy, cur[0] - cx)
            if cmd["clockwise"]:
                sweep = a1 - a0
                while sweep > 0.0:
                    sweep -= 2.0 * math.pi
            else:
                sweep = a1 - a0
                while sweep < 0.0:
                    sweep += 2.0 * math.pi
            steps = max(2, int(radius * abs(sweep) / 1.5))
            for k in range(1, steps + 1):
                angle = a0 + sweep * k / steps
                current.append(
                    (
                        cx + radius * math.cos(angle),
                        cy + radius * math.sin(angle),
                    )
                )
        else:
            current.append(cur)
        prev = cur
    if current:
        polylines.append(current)
    return polylines


def _add_direction_arrows(ax, polylines, spacing=16.0, color="#111111"):
    """Place an arrow head every `spacing` mm along each cut polyline."""
    distance_to_next = spacing
    for polyline in polylines:
        distance_to_next = spacing
        for (x0, y0), (x1, y1) in zip(polyline, polyline[1:]):
            dx, dy = x1 - x0, y1 - y0
            length = (dx * dx + dy * dy) ** 0.5
            if length < 1e-9:
                continue
            ux, uy = dx / length, dy / length
            d = distance_to_next
            while d <= length:
                cx, cy = x0 + ux * d, y0 + uy * d
                head = 1.0
                ax.annotate(
                    "",
                    xy=(cx + ux * head, cy + uy * head),
                    xytext=(cx - ux * head, cy - uy * head),
                    arrowprops=dict(
                        arrowstyle="-|>", color=color, linewidth=1.1
                    ),
                )
                d += spacing
            distance_to_next = d - length


def _run(boundary, islands, direction):
    part = Part.from_polygons(boundary, islands, (60.0, 40.0))
    plan = plan_clearing(
        part,
        "",
        tool_radius=3.0,
        step_over=2.5,
        target_z=-3.0,
        safe_z=2.0,
        cut_direction=direction,
        feed_rate=600.0,
        plunge_feed=200.0,
    )
    return run_intent(create_intent(plan, part, 0))


def generate_cut_direction():
    """1x2 grid: the same pocket-with-island cleared climb vs conventional,
    arrows along the floor passes showing the walk direction."""
    boundary = _rect(0.0, 0.0, 60.0, 40.0)
    island = _rect(25.0, 15.0, 10.0, 10.0)

    fig, axes = plt.subplots(1, 2, figsize=(16, 11))

    for ax, direction, title in [
        (axes[0], "ccw", "cut_direction='ccw' — climb"),
        (axes[1], "cw", "cut_direction='cw' — conventional"),
    ]:
        ops = _run(boundary, [island], direction)
        plot_ops_2d(ax, ops, boundary=boundary, islands=[island])
        _add_direction_arrows(ax, _floor_polylines(ops, -3.0), spacing=16.0)
        ax.set_title(title, fontsize=11)
        ax.set_xlabel("X (mm)")
        ax.set_ylabel("Y (mm)")
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.25)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.cnc.plan.clearing.md"]

__images__ = [
    {
        "heading": "plan_clearing",
        "caption": (
            "Cut direction: climb ('ccw', left) vs conventional"
            " ('cw', right) — arrows mark the walk."
        ),
        "function": generate_cut_direction,
    },
]

if __name__ == "__main__":
    figure = generate_cut_direction()
    figure.savefig("/tmp/cut_direction.png", dpi=150, bbox_inches="tight")
    print("Saved /tmp/cut_direction.png")
