"""Generate ordering example images."""

import matplotlib.pyplot as plt

from raygeo.geo.algo.ordering import order_nearest_neighbor


def generate_order_nearest_neighbor():
    """Show nearest-neighbor ordering of arc-like paths."""
    arcs = [
        [(10.0, 80.0), (30.0, 80.0), (30.0, 60.0), (10.0, 60.0)],
        [(70.0, 10.0), (90.0, 10.0), (90.0, 30.0), (70.0, 30.0)],
        [(50.0, 40.0), (70.0, 40.0), (70.0, 60.0), (50.0, 60.0)],
        [(10.0, 10.0), (30.0, 10.0), (30.0, 30.0), (10.0, 30.0)],
        [(70.0, 80.0), (90.0, 80.0), (90.0, 100.0), (70.0, 100.0)],
    ]

    order = order_nearest_neighbor(arcs)

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5))

    ax1.set_title("Input paths (coloured by index)")
    ax1.set_aspect("equal")
    colors = ["red", "blue", "green", "orange", "purple"]
    for i, arc in enumerate(arcs):
        xs = [p[0] for p in arc]
        ys = [p[1] for p in arc]
        ax1.plot(xs, ys, "o-", color=colors[i], linewidth=2, markersize=5)
        mid = arc[len(arc) // 2]
        ax1.text(
            mid[0],
            mid[1],
            str(i),
            fontsize=10,
            ha="center",
            va="center",
            fontweight="bold",
        )
    ax1.grid(True, alpha=0.3)

    ax2.set_title(f"Visit order: {order}")
    ax2.set_aspect("equal")
    for idx_in_order, arc_idx in enumerate(order):
        arc = arcs[arc_idx]
        xs = [p[0] for p in arc]
        ys = [p[1] for p in arc]
        t = idx_in_order / max(len(order) - 1, 1)
        color = (t, 0.2, 1.0 - t)
        ax2.plot(xs, ys, "o-", color=color, linewidth=2.5, markersize=5)
        mid = arc[len(arc) // 2]
        ax2.text(
            mid[0],
            mid[1],
            str(arc_idx),
            fontsize=10,
            ha="center",
            va="center",
            fontweight="bold",
        )
    # Draw arrows showing the NN connections
    for i in range(len(order) - 1):
        from_arc = arcs[order[i]]
        to_arc = arcs[order[i + 1]]
        fx, fy = from_arc[-1]
        tx, ty = to_arc[0]
        ax2.annotate(
            "",
            xy=(tx, ty),
            xytext=(fx, fy),
            arrowprops=dict(arrowstyle="->", color="gray", lw=1.5, alpha=0.7),
        )
    ax2.grid(True, alpha=0.3)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.ordering.md"]

__images__ = [
    {
        "heading": "order_nearest_neighbor",
        "caption": (
            "Nearest-neighbor ordering of arc-like paths —"
            " starts with the longest, then chains by proximity"
        ),
        "function": generate_order_nearest_neighbor,
    },
]
