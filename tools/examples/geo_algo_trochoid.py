"""Generate trochoid example images."""

import matplotlib.pyplot as plt

from raygeo.geo.algo.trochoid import get_trochoid_along_3d


def generate_straight():
    # Straight carrier — compare low vs high engagement angle
    carrier = [(0, 0), (80, 0)]

    fig, axes = plt.subplots(1, 2, figsize=(14, 5))

    for ax, label, eng_deg in zip(
        axes, ["Engagement 60°", "Engagement 120°"], [60, 120]
    ):
        pts = get_trochoid_along_3d(
            carrier,
            diameter=10,
            engagement_angle_deg=eng_deg,
            step_over_ratio=0.2,
            z=0,
        )
        xs = [p[0] for p in pts]
        ys = [p[1] for p in pts]
        ax.plot(xs, ys, "steelblue", linewidth=2, label=label)
        ax.scatter(xs, ys, c=range(len(pts)), cmap="viridis", s=4, alpha=0.5)
        ax.plot(
            [p[0] for p in carrier],
            [p[1] for p in carrier],
            "r--",
            linewidth=1,
            label="Carrier",
        )
        ax.set_aspect("equal")
        ax.set_xlim(-10, 100)
        ax.set_ylim(-15, 15)
        ax.set_title(label)
        ax.set_xlabel("X")
        ax.set_ylabel("Y")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=9)

    fig.tight_layout()
    return fig


def generate_l_shaped():
    carrier_l = [(0, 0), (50, 0), (50, 50)]
    pts = get_trochoid_along_3d(
        carrier_l,
        diameter=10,
        engagement_angle_deg=90,
        step_over_ratio=0.2,
        z=0,
    )

    fig, ax = plt.subplots(figsize=(8, 8))
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    ax.plot(xs, ys, "steelblue", linewidth=2, label="Trochoid path")
    ax.scatter(xs, ys, c=range(len(pts)), cmap="viridis", s=4, alpha=0.5)
    cx = [p[0] for p in carrier_l]
    cy = [p[1] for p in carrier_l]
    ax.plot(cx, cy, "r--", linewidth=2, label="Carrier")
    ax.plot(cx, cy, "ro", markersize=6)
    ax.set_aspect("equal")
    ax.set_xlim(-10, 70)
    ax.set_ylim(-10, 60)
    ax.set_title("Trochoidal path on L-shaped carrier")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.trochoid.md"]
__images__ = [
    {
        "heading": "get_trochoid_along_3d",
        "caption": (
            "Trochoidal toolpath along a straight carrier — 60° vs 120°"
        ),
        "function": generate_straight,
    },
    {
        "heading": "get_trochoid_along_3d",
        "caption": "Trochoidal toolpath around an L-shaped corner",
        "function": generate_l_shaped,
    },
]
