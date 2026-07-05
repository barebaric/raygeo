"""Generate visualisations of toroid entry motion assembly."""

import matplotlib.pyplot as plt

from raygeo.ops.assembly.toroid import generate_toroid


def _all_moving_pts(result):
    pts = []
    for i in range(result.ops.len()):
        if result.ops.is_travel(i) or result.ops.is_cutting(i):
            ep = result.ops.endpoint(i)
            pts.append((ep[0], ep[1], ep[2]))
    return pts


def generate_toroid_example():
    """Toroid to ops."""
    carrier = [(0.0, 0.0), (80.0, 0.0)]
    result = generate_toroid(
        carrier=carrier,
        tool_radius=3.0,
        step_distance=2.0,
        z=-5.0,
    )

    pts = _all_moving_pts(result)
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]

    fig, ax = plt.subplots(figsize=(10, 6))
    ax.plot(xs, ys, color="steelblue", linewidth=1.0)

    # Draw carrier
    carrier_x = [p[0] for p in carrier]
    carrier_y = [p[1] for p in carrier]
    ax.plot(
        carrier_x, carrier_y, "k--", alpha=0.3, linewidth=2, label="Carrier"
    )

    ax.scatter(xs[0], ys[0], c="forestgreen", s=80, label="Start")
    ax.scatter(xs[-1], ys[-1], c="crimson", s=80, label="End")

    ax.set_aspect("equal")
    ax.set_title("Toroidal (Trochoidal) Path")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.legend()
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.toroid.md"]
__images__ = [
    {
        "heading": "generate_toroid",
        "caption": "Trochoidal slot path along a carrier polyline",
        "function": generate_toroid_example,
    },
]
