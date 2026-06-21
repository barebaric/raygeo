"""Generate ClearedArea example images."""

import math

import matplotlib.pyplot as plt

from raygeo.geo.algo.cleared_area import ClearedArea
from raygeo.geo.algo.hsm import adaptive_entry
from raygeo.geo.algo.offset import compute_inset_region


def generate_raster():
    """Raster toolpath cleared area."""
    # Square pocket 80x80
    boundary = [(5, 5), (85, 5), (85, 85), (5, 85)]

    ca = ClearedArea()
    tool_radius = 3.0
    for i in range(15):
        x = 12.0 + i * 5.0
        if x > 80.0:
            break
        ca.expand([(x, 10), (x, 80)], tool_radius)

    remaining = ca.remaining([boundary])

    fig, ax = plt.subplots(figsize=(7, 7))

    # Draw cleared fragments and remaining area first (behind boundary)
    all_frags = ca.query_window((-10, -10, 100, 100))
    for frag in all_frags:
        fx, fy = zip(
            *([(p[0], p[1]) for p in frag] + [(frag[0][0], frag[0][1])])
        )
        ax.fill(fx, fy, "steelblue", alpha=0.3)
        ax.plot(fx, fy, "steelblue", linewidth=1, alpha=0.6)

    for poly in remaining:
        px, py = zip(
            *([(p[0], p[1]) for p in poly] + [(poly[0][0], poly[0][1])])
        )
        ax.fill(
            px,
            py,
            "tomato",
            alpha=0.4,
            label="Remaining" if poly is remaining[0] else None,
        )
        ax.plot(px, py, "tomato", linewidth=1.5)

    # Boundary drawn last so the black line is visible on top
    bx, by = zip(*(boundary + [boundary[0]]))
    ax.plot(bx, by, "k-", linewidth=2, label="Boundary")

    ax.set_aspect("equal")
    ax.set_xlim(0, 90)
    ax.set_ylim(0, 90)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=9)
    ax.set_title(
        f"ClearedArea: {ca.total_area():.0f} mm\u00b2 cleared, "
        f"{len(all_frags)} fragments"
    )

    fig.tight_layout()
    return fig


def generate_bulk():
    """Bulk polygon insertion cleared area."""
    ca2 = ClearedArea()
    # L-shaped pocket
    pocket = [(0, 0), (100, 0), (100, 100), (60, 100), (60, 40), (0, 40)]
    # Bulk cleared region (a large rectangle inside the pocket)
    cleared_bulk = [(10, 10), (90, 10), (90, 90), (10, 90)]
    ca2.add_cleared_polygons([cleared_bulk])
    remaining2 = ca2.remaining([pocket])

    fig2, ax2 = plt.subplots(figsize=(7, 7))

    # Cleared bulk
    cx, cy = zip(*(cleared_bulk + [cleared_bulk[0]]))
    ax2.fill(cx, cy, "steelblue", alpha=0.3, label="Cleared (bulk)")
    ax2.plot(cx, cy, "steelblue", linewidth=1.5)

    # Remaining
    for poly in remaining2:
        px, py = zip(*(poly + [poly[0]]))
        ax2.fill(
            px,
            py,
            "tomato",
            alpha=0.4,
            label="Remaining" if poly is remaining2[0] else None,
        )
        ax2.plot(px, py, "tomato", linewidth=1.5)

    # Boundary
    bx2, by2 = zip(*(pocket + [pocket[0]]))
    ax2.plot(bx2, by2, "k-", linewidth=2, label="Pocket boundary")

    ax2.set_aspect("equal")
    ax2.set_xlim(-5, 105)
    ax2.set_ylim(-5, 105)
    ax2.grid(True, alpha=0.3)
    ax2.legend(fontsize=9)
    ax2.set_title(
        f"ClearedArea.add_cleared_polygons: "
        f"{ca2.total_area():.0f} mm\u00b2 cleared"
    )

    fig2.tight_layout()
    return fig2


def generate_incorporate():
    """incorporate returns only the newly-added portion."""
    ca = ClearedArea()
    # Start with a square cleared area
    initial = [(10, 10), (90, 10), (90, 90), (10, 90)]
    ca.add_cleared_polygons([initial])

    # Incorporate a larger square — only the outer ring is new
    larger = [(0, 0), (100, 0), (100, 100), (0, 100)]
    new_ring = ca.incorporate([larger])

    fig, ax = plt.subplots(figsize=(7, 7))

    # Draw initial cleared region
    ix, iy = zip(*(initial + [initial[0]]))
    ax.fill(ix, iy, "steelblue", alpha=0.3, label="Existing cleared")
    ax.plot(ix, iy, "steelblue", linewidth=1.5)

    # Draw newly-incorporated ring
    for poly in new_ring:
        px, py = zip(*(poly + [poly[0]]))
        ax.fill(
            px,
            py,
            "limegreen",
            alpha=0.4,
            label="New (incorporate)" if poly is new_ring[0] else None,
        )
        ax.plot(px, py, "limegreen", linewidth=2)

    ax.set_aspect("equal")
    ax.set_xlim(-5, 105)
    ax.set_ylim(-5, 105)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=9)
    ax.set_title(f"incorporate: {len(new_ring)} new fragment(s)")

    fig.tight_layout()
    return fig


def generate_frontier():
    """frontier returns a simplified outer boundary."""
    ca = ClearedArea()
    # Two overlapping squares that should be merged
    poly1 = [(10, 10), (60, 10), (60, 60), (10, 60)]
    poly2 = [(40, 40), (90, 40), (90, 90), (40, 90)]
    ca.add_cleared_polygons([poly1, poly2])

    f = ca.frontier(0.5)

    fig, ax = plt.subplots(figsize=(7, 7))

    # Draw fragments
    for poly in ca.query_window((-10, -10, 110, 110)):
        px, py = zip(
            *([(p[0], p[1]) for p in poly] + [(poly[0][0], poly[0][1])])
        )
        ax.fill(px, py, "steelblue", alpha=0.2)
        ax.plot(px, py, "steelblue", linewidth=1, alpha=0.5)

    # Draw frontier in bold
    for poly in f:
        fx, fy = zip(*(poly + [poly[0]]))
        ax.plot(
            fx,
            fy,
            "crimson",
            linewidth=3,
            label="Frontier" if poly is f[0] else None,
        )

    ax.set_aspect("equal")
    ax.set_xlim(0, 100)
    ax.set_ylim(0, 100)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=9)
    ax.set_title(
        f"frontier: {len(f)} polygon(s), merged from 2 overlapping fragments"
    )

    fig.tight_layout()
    return fig


def generate_bites():
    """bites across 3 sequential expansion steps."""
    import math

    cx, cy = 50.0, 50.0
    step_over = 8.0
    pocket = [(20, 20), (80, 20), (80, 80), (20, 80)]

    def octagon(r):
        return [
            (
                cx + r * math.cos(2 * math.pi * i / 8),
                cy + r * math.sin(2 * math.pi * i / 8),
            )
            for i in range(8)
        ]

    ca = ClearedArea()
    init = octagon(12.0)
    ca.add_cleared_polygons([init])

    # Run 3 sequential bite steps, snapshotting before each
    cleared_snapshots = [ca.frontier(0.5)]
    bite_sets = []
    for _ in range(3):
        b = ca.bites(step_over, [pocket], 1.0)
        bite_sets.append(b)
        ca.incorporate(b)
        cleared_snapshots.append(ca.frontier(0.5))

    fig, axes = plt.subplots(1, 3, figsize=(18, 6))
    colors = ["#fdbe85", "#fd8d3c", "#d94701"]

    for step in range(3):
        ax = axes[step]

        # Pocket boundary
        bx, by = zip(*(pocket + [pocket[0]]))
        ax.plot(bx, by, "k-", linewidth=2, label="Pocket boundary")

        # Cleared area before this step
        for poly in cleared_snapshots[step]:
            px, py = zip(*(poly + [poly[0]]))
            ax.fill(px, py, "steelblue", alpha=0.25)
            ax.plot(px, py, "steelblue", linewidth=1, alpha=0.5)

        # Bites for this step
        label = "Bite" if step == 0 else None
        for poly in bite_sets[step]:
            px, py = zip(*(poly + [poly[0]]))
            ax.fill(px, py, colors[step], alpha=0.6, label=label)
            ax.plot(px, py, color=colors[step], linewidth=2)

        ax.set_aspect("equal")
        ax.set_xlim(10, 90)
        ax.set_ylim(10, 90)
        ax.grid(True, alpha=0.3)
        ax.set_title(f"Step {step + 1}: {len(bite_sets[step])} bite(s)")
        if step == 0:
            ax.legend(fontsize=8)

    fig.suptitle(
        "bites: 3 sequential expansions, each clipped to pocket boundary",
        fontsize=13,
    )
    fig.tight_layout()
    return fig


def generate_bite_in_direction():
    """Show directional bites coloured by pass order."""
    boundary = [(0, 0), (180, 0), (180, 120), (0, 120)]
    islands = [
        [(15, 15), (35, 15), (35, 35), (15, 35)],
        [
            (
                80 + 10 * math.cos(2 * math.pi * i / 32),
                50 + 10 * math.sin(2 * math.pi * i / 32),
            )
            for i in range(32)
        ],
        [(130, 80), (160, 80), (160, 105), (130, 105)],
    ]
    tool_radius = 3.0
    step_over = 2.0

    _, cp = adaptive_entry(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=tool_radius,
        step_over=step_over,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)
    va, total = compute_inset_region(boundary, tool_radius, islands)

    directions = {
        "east": (200, 60),
        "north": (90, 140),
        "west": (-20, 60),
        "south": (90, -20),
    }
    all_bites = []

    for label, target in directions.items():
        for _ in range(20):
            bites = ca.bite_in_direction(
                step_over,
                va,
                0.01,
                target,
                math.pi / 3,
            )
            if not bites:
                break
            for b in bites:
                all_bites.append((b, label))
            ca.incorporate(bites)

    fig, ax = plt.subplots(figsize=(7, 6))
    ax.set_aspect("equal")

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, alpha=0.3, label="Boundary")

    for isl in islands:
        ix = [p[0] for p in isl] + [isl[0][0]]
        iy = [p[1] for p in isl] + [isl[0][1]]
        ax.fill(
            ix,
            iy,
            facecolor="lightgray",
            edgecolor="gray",
            hatch="///",
            linewidth=1,
        )

    n = len(all_bites)
    for idx, (bite, label) in enumerate(all_bites):
        t = idx / max(n - 1, 1)
        r = 0.9 - 0.6 * t
        g = 0.2 + 0.5 * t
        color = (r, g, 0.2)
        xs = [p[0] for p in bite] + [bite[0][0]]
        ys = [p[1] for p in bite] + [bite[0][1]]
        ax.fill(
            xs, ys, facecolor=color, alpha=0.3, edgecolor=color, linewidth=0.5
        )

    ax.set_title(f"Directional bites ({n} passes)")

    import matplotlib.colors as mcolors

    cmap = mcolors.LinearSegmentedColormap.from_list(
        "order",
        [(0, (0.9, 0.2, 0.2)), (1, (0.3, 0.7, 0.2))],
    )
    sm = plt.cm.ScalarMappable(cmap=cmap, norm=mcolors.Normalize(0, n))
    sm.set_array([])
    cbar = fig.colorbar(
        sm, ax=ax, orientation="vertical", pad=0.02, shrink=0.7
    )
    cbar.set_label("Pass index")

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.cleared_area.md"]
__images__ = [
    {
        "heading": None,
        "caption": (
            "ClearedArea tracking a simulated raster toolpath — "
            "cleared fragments shown in blue, remaining area in red"
        ),
        "function": generate_raster,
    },
    {
        "heading": "add_cleared_polygons",
        "caption": (
            "ClearedArea with bulk polygon insertion via "
            "``add_cleared_polygons`` — cleared region in blue, "
            "remaining area in red"
        ),
        "function": generate_bulk,
    },
    {
        "heading": "incorporate",
        "caption": (
            "``incorporate`` adds polygons to the cleared state while "
            "returning only the newly-covered region (shown in green)."
        ),
        "function": generate_incorporate,
    },
    {
        "heading": "frontier",
        "caption": (
            "``frontier`` returns the outer boundary of the cleared area "
            "after merging overlapping fragments — shown in crimson."
        ),
        "function": generate_frontier,
    },
    {
        "heading": "bites",
        "caption": (
            "``bites`` computes the expansible material — the crescent-shaped "
            "regions of uncut material reachable by expanding the frontier "
            "by ``step_over``."
        ),
        "function": generate_bites,
    },
    {
        "heading": "bite_in_direction",
        "caption": (
            "Directional bites coloured by pass order"
            " (first = dark, later = pale)"
        ),
        "function": generate_bite_in_direction,
    },
]
