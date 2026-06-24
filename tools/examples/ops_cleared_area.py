"""Generate ClearedArea example images."""

import math

import matplotlib.colors as mcolors
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.lines import Line2D
from matplotlib.patches import Circle

from raygeo.geo.algo.engagement import compute_engagement
from raygeo.geo.algo.medial_axis import MedialAxis
from raygeo.geo.algo.offset import compute_inset_region
from raygeo.ops.assembly.hsm import adaptive_entry
from raygeo.ops.cleared_area import ClearedArea, StepperOptions


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


def generate_expand_step():
    """show a single expand_step: swept disk enlarges the cleared area."""
    tool_radius = 5.0
    prev = (10, 10)
    nxt = (40, 40)

    # Initial cleared area: a small square the segment does NOT fully cover.
    initial_poly = [(15, 15), (25, 15), (25, 25), (15, 25)]

    ca = ClearedArea()
    ca.add_cleared_polygons([initial_poly])

    ca_final = ClearedArea()
    ca_final.add_cleared_polygons([initial_poly])
    ca_final.expand_step(prev, nxt, tool_radius)

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(12, 5))

    for ax, label, ca_state in [
        (ax1, "Before expand_step", ca),
        (ax2, "After expand_step", ca_final),
    ]:
        for frag in ca_state.query_window((-10, -10, 80, 80)):
            fx = [p[0] for p in frag] + [frag[0][0]]
            fy = [p[1] for p in frag] + [frag[0][1]]
            ax.fill(fx, fy, "steelblue", alpha=0.3)
            ax.plot(fx, fy, "steelblue", linewidth=1.5)

        ax.set_aspect("equal")
        ax.set_xlim(-5, 55)
        ax.set_ylim(-5, 55)
        ax.set_title(label)
        ax.grid(True, alpha=0.3)

    # Draw the segment and tool circles on both panels
    for ax in (ax1, ax2):
        ax.annotate(
            "",
            xy=nxt,
            xytext=prev,
            arrowprops=dict(arrowstyle="->", color="red", lw=2),
        )
        for pt in (prev, nxt):
            c = Circle(
                pt,
                tool_radius,
                fill=False,
                edgecolor="red",
                linewidth=1.5,
                linestyle="--",
            )
            ax.add_patch(c)
        ax.plot(prev[0], prev[1], "ro", markersize=6)
        ax.plot(nxt[0], nxt[1], "ro", markersize=6)

    fig.suptitle(
        "expand_step: swept disk merges into the cleared area",
        fontsize=11,
    )
    fig.tight_layout()
    return fig


def generate_bites():
    """bites across 3 sequential expansion steps."""
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
        for _ in range(40):
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

    for poly in cp:
        px = [p[0] for p in poly] + [poly[0][0]]
        py = [p[1] for p in poly] + [poly[0][1]]
        ax.fill(px, py, "white", zorder=2)
        ax.plot(px, py, "steelblue", linewidth=1, alpha=0.4, zorder=2)

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


def generate_signed_boundary_distance():
    """Heatmap of signed distance around a cleared rectangle."""
    ca = ClearedArea()
    ca.add_cleared_polygons([[(-20, -20), (20, -20), (20, 20), (-20, 20)]])

    xs = np.linspace(-40, 40, 80)
    ys = np.linspace(-40, 40, 80)
    dists = np.zeros((80, 80))
    for i, x in enumerate(xs):
        for j, y in enumerate(ys):
            dists[j, i] = ca.signed_boundary_distance(x, y)

    fig, ax = plt.subplots(figsize=(8, 7))
    im = ax.pcolormesh(xs, ys, dists, shading="auto", cmap="RdYlGn_r")
    cbar = fig.colorbar(im, ax=ax)
    cbar.set_label("Signed distance (mm)")

    for frag in ca.query_window((-50, -50, 50, 50)):
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        ax.plot(fx, fy, "k-", linewidth=2, label="Boundary")

    ax.set_aspect("equal")
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_title("Signed Boundary Distance (green = inside, red = outside)")
    fig.tight_layout()
    return fig


def generate_find_next_resume():
    """Resume point found by walking the cleared-area frontier."""
    boundary = [(0, 0), (100, 0), (100, 80), (0, 80)]
    radius = 3.0

    n = 32
    cx, cy = 50.0, 40.0
    cr = 15.0
    circle_poly = [
        (
            cx + cr * math.cos(2 * math.pi * i / n),
            cy + cr * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]

    ca = ClearedArea()
    ca.add_cleared_polygons([circle_poly])

    mat = MedialAxis.compute(
        boundary, holes=[], min_clearance=1.0, sampling_spacing=6.0
    )

    end_pos = (50.0, 55.0)
    result = ca.find_next_resume(
        mat=mat,
        end_pos=end_pos,
        radius=radius,
        min_engagement=math.pi * 0.3,
    )

    fig, ax = plt.subplots(figsize=(8, 6))

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, label="Boundary")

    for frag in ca.query_window((-10, -10, 120, 100)):
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        ax.fill(fx, fy, "steelblue", alpha=0.3)
        ax.plot(fx, fy, "steelblue", linewidth=1.5, alpha=0.5)

    ax.plot(end_pos[0], end_pos[1], "rv", markersize=10, label="End position")
    c = Circle(
        end_pos,
        radius,
        fill=False,
        edgecolor="red",
        linewidth=1.5,
        linestyle="--",
    )
    ax.add_patch(c)

    if result is not None:
        ax.plot(
            result.pos[0],
            result.pos[1],
            "g*",
            markersize=14,
            label="Resume point",
        )
        dx = math.cos(result.heading) * 12
        dy = math.sin(result.heading) * 12
        ax.annotate(
            "",
            xy=(result.pos[0] + dx, result.pos[1] + dy),
            xytext=(result.pos[0], result.pos[1]),
            arrowprops=dict(arrowstyle="->", color="green", lw=2),
        )
        lx = [p[0] for p in result.link_path]
        ly = [p[1] for p in result.link_path]
        ax.plot(lx, ly, "g-", linewidth=2, alpha=0.7)
        ax.plot(lx, ly, "go", markersize=3, alpha=0.7)

    ax.set_aspect("equal")
    ax.set_xlim(0, 100)
    ax.set_ylim(0, 80)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_title("find_next_resume: Resume Point After Path Termination")
    handles, labels = ax.get_legend_handles_labels()
    handles.append(
        Line2D(
            [0, 1],
            [0, 0],
            color="green",
            linewidth=2,
            marker=">",
            markersize=10,
        )
    )
    labels.append("Heading")
    ax.legend(handles, labels, fontsize=8)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_step_batch():
    """Visualise multiple segments batched then committed at once."""
    ca = ClearedArea()
    ca.add_cleared_polygons([[(15, 15), (25, 15), (25, 25), (15, 25)]])
    ca.begin_step_batch()

    # Queue three segments.
    segments = [
        ((10, 10), (20, 30)),
        ((20, 30), (35, 35)),
        ((35, 35), (45, 25)),
    ]
    r = 4.0
    for prev, nxt in segments:
        ca.expand_step_batched(prev, nxt, r)
    ca.commit_step_batch()

    fig, ax = plt.subplots(figsize=(8, 7))

    for frag in ca.query_window((-10, -10, 70, 70)):
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        ax.fill(fx, fy, "steelblue", alpha=0.3)
        ax.plot(fx, fy, "steelblue", linewidth=1.5)

    # Draw the initial seed as a dashed outline.
    ax.plot(
        [15, 25, 25, 15, 15],
        [15, 15, 25, 25, 15],
        "k--",
        linewidth=1.5,
        alpha=0.5,
        label="Initial seed",
    )

    for prev, nxt in segments:
        ax.annotate(
            "",
            xy=nxt,
            xytext=prev,
            arrowprops=dict(arrowstyle="->", color="red", lw=2),
        )
        c = Circle(
            nxt, r, fill=False, edgecolor="red", linewidth=1, linestyle=":"
        )
        ax.add_patch(c)

    ax.set_aspect("equal")
    ax.set_xlim(0, 60)
    ax.set_ylim(0, 50)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_title("step_batch: 3 Segments Queued → Single Union")
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_expand():
    """Sweep a multi-segment path with a disk radius."""
    path = [(10, 10), (20, 30), (40, 25), (55, 40)]
    r = 4.0

    ca = ClearedArea()
    ca.add_cleared_polygons([[(15, 15), (25, 15), (25, 25), (15, 25)]])
    ca.expand(path, r)

    fig, ax = plt.subplots(figsize=(8, 7))

    for frag in ca.query_window((-10, -10, 80, 80)):
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        ax.fill(fx, fy, "steelblue", alpha=0.3)
        ax.plot(fx, fy, "steelblue", linewidth=1.5)

    # Draw the path and disks.
    for i in range(len(path) - 1):
        p0, p1 = path[i], path[i + 1]
        ax.annotate(
            "",
            xy=p1,
            xytext=p0,
            arrowprops=dict(arrowstyle="->", color="red", lw=2),
        )
    for pt in path:
        c = Circle(
            pt, r, fill=False, edgecolor="red", linewidth=1, linestyle=":"
        )
        ax.add_patch(c)

    ax.plot([p[0] for p in path], [p[1] for p in path], "ro", markersize=4)

    ax.set_aspect("equal")
    ax.set_xlim(0, 70)
    ax.set_ylim(0, 60)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_title("expand: Sweep Disk Along a Polyline Path")
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_remaining():
    """Show the remaining area after subtracting cleared fragments."""
    pocket = [(0, 0), (100, 0), (100, 100), (0, 100)]
    cleared = [(20, 20), (80, 20), (80, 80), (20, 80)]

    ca = ClearedArea()
    ca.add_cleared_polygons([cleared])
    remaining = ca.remaining([pocket])

    fig, ax = plt.subplots(figsize=(8, 7))

    bx = [p[0] for p in pocket] + [pocket[0][0]]
    by = [p[1] for p in pocket] + [pocket[0][1]]
    ax.plot(bx, by, "k-", linewidth=2, label="Boundary")

    # Fill the outer boundary (remaining area) in blue.
    for i, poly in enumerate(remaining):
        px = [p[0] for p in poly] + [poly[0][0]]
        py = [p[1] for p in poly] + [poly[0][1]]
        if i == 0:
            ax.fill(px, py, "steelblue", alpha=0.25, label="Remaining")
        ax.plot(px, py, "steelblue", linewidth=1.5)

    # Draw subtracted (cleared) on top — opaque red with hatch
    # so it fully covers the blue fill underneath.
    subtracted_frags = ca.query_window((-10, -10, 110, 110))
    for idx, frag in enumerate(subtracted_frags):
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        ax.fill(
            fx,
            fy,
            facecolor="white",
            zorder=3,
        )
        ax.fill(
            fx,
            fy,
            facecolor="tomato",
            edgecolor="tomato",
            hatch="///",
            alpha=0.3,
            linewidth=0.5,
            zorder=4,
            label="Subtracted" if idx == 0 else "",
        )

    ax.set_aspect("equal")
    ax.set_xlim(-5, 105)
    ax.set_ylim(-5, 105)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_title("remaining: Uncut Area After Subtraction")
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_query_window():
    """Show fragments returned by query_window within a bbox."""
    ca = ClearedArea()
    ca.add_cleared_polygons(
        [
            [(10, 10), (40, 10), (40, 40), (10, 40)],
            [(60, 60), (90, 60), (90, 90), (60, 90)],
        ]
    )
    bbox = (0, 0, 50, 50)
    result = ca.query_window(bbox)

    fig, ax = plt.subplots(figsize=(8, 7))

    # Draw all fragments.
    for frag in ca.query_window((-10, -10, 110, 110)):
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        ax.fill(fx, fy, "steelblue", alpha=0.15)
        ax.plot(fx, fy, "steelblue", linewidth=1, alpha=0.4)

    # Draw the query bounding box.
    bx = [bbox[0], bbox[2], bbox[2], bbox[0], bbox[0]]
    by = [bbox[1], bbox[1], bbox[3], bbox[3], bbox[1]]
    ax.plot(bx, by, "g-", linewidth=2, label="Query bbox")

    # Highlight the returned fragments.
    for frag in result:
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        first_qr = frag is result[0]
        ax.fill(
            fx,
            fy,
            "tomato",
            alpha=0.4,
            label="Query result" if first_qr else "",
        )
        ax.plot(fx, fy, "tomato", linewidth=2)

    ax.set_aspect("equal")
    ax.set_xlim(0, 100)
    ax.set_ylim(0, 100)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_title("query_window: Fragments Inside a Bounding Box")
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def _engagement_at(pt, ca, tool_radius):
    """Compute engagement by measuring signed distance to cleared boundary."""
    d = ca.signed_boundary_distance(pt[0], pt[1])
    angle, _, _ = compute_engagement(d, tool_radius)
    return angle


def generate_wall_following():
    """Tool stepping along a curved wall, maintaining constant engagement."""
    tool_radius = 3.0
    step_len = 1.0
    target_eng = math.pi

    # Cleared area with a curved top edge (sine wave).
    n = 100
    poly = [(-100.0, -100.0), (100.0, -100.0), (100.0, 20.0)]
    for i in range(n + 1):
        x = 100.0 - 200.0 * i / n
        y = 20.0 + 2.0 * math.sin(x * math.pi / 40.0)
        poly.append((x, y))

    ca = ClearedArea()
    ca.add_cleared_polygons([poly])

    opts = StepperOptions()
    opts.radius = tool_radius
    opts.step_length = step_len
    opts.target_engagement = target_eng
    opts.max_deflection = 0.8
    path, _status = ca.run_segment((0, 20), 0.0, opts, 80)

    fig, (ax1, ax2) = plt.subplots(
        1, 2, figsize=(14, 5), gridspec_kw={"width_ratios": [3, 1]}
    )

    # ── Left: path overlaid on geometry ──
    engagements = [_engagement_at(p, ca, tool_radius) for p in path]
    max_eng = max(engagements) if engagements else 1.0
    norm_eng = [e / max_eng for e in engagements]

    for frag in ca.query_window((-110, -110, 110, 40)):
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        ax1.fill(fx, fy, "steelblue", alpha=0.2)
        ax1.plot(fx, fy, "steelblue", linewidth=1, alpha=0.5)

    for i in range(len(path) - 1):
        seg_xs = [path[i][0], path[i + 1][0]]
        seg_ys = [path[i][1], path[i + 1][1]]
        c = plt.cm.RdYlGn(norm_eng[i])
        ax1.plot(seg_xs, seg_ys, color=c, linewidth=2)

    # Every 10th tool position.
    for i in range(0, len(path), 10):
        c = Circle(
            path[i],
            tool_radius,
            fill=False,
            edgecolor=plt.cm.RdYlGn(norm_eng[i]),
            linewidth=1,
            linestyle="--",
        )
        ax1.add_patch(c)

    ax1.set_aspect("equal")
    ax1.set_xlim(-10, 85)
    ax1.set_ylim(12, 32)
    ax1.set_xlabel("X (mm)")
    ax1.set_ylabel("Y (mm)")
    ax1.set_title(
        "Wall Following: Curved Boundary\n"
        "(path colour = engagement, green = on-target)"
    )
    ax1.grid(True, alpha=0.3)

    # ── Right: engagement histogram ──
    target_line_label = f"Target ({target_eng:.2f})"
    ax2.hist(engagements, bins=20, color="steelblue", alpha=0.7)
    ax2.axvline(
        target_eng,
        color="red",
        linestyle="--",
        linewidth=2,
        label=target_line_label,
    )
    ax2.set_xlabel("Engagement angle (rad)")
    ax2.set_ylabel("Count")
    mean_eng = np.mean(engagements)
    std_eng = np.std(engagements)
    ax2.set_title(f"Engagement  |  μ = {mean_eng:.3f}  σ = {std_eng:.3f}")
    ax2.legend(fontsize=8)
    ax2.grid(True, alpha=0.3)

    fig.tight_layout()
    return fig

    fig.tight_layout()
    return fig


def generate_pocket_corner():
    """Stepping through a 90° corner, showing deflection."""
    tool_radius = 3.0
    step_len = 0.6
    target_eng = math.pi * 0.85

    # Cleared area: an L-shaped region.
    ca = ClearedArea()
    ca.add_cleared_polygons(
        [
            [
                (-10, -10),
                (30, -10),
                (30, 20),
                (10, 20),
                (10, 30),
                (-10, 30),
            ]
        ]
    )

    opts = StepperOptions()
    opts.radius = tool_radius
    opts.step_length = step_len
    opts.target_engagement = target_eng
    opts.max_deflection = 0.8
    path, _status = ca.run_segment((25, 20), -math.pi / 2, opts, 60)

    fig, ax = plt.subplots(figsize=(8, 7))

    engagements = [_engagement_at(p, ca, tool_radius) for p in path]
    max_eng = max(engagements) if engagements else 1.0
    norm_eng = [e / max_eng for e in engagements]

    for i in range(len(path) - 1):
        seg_xs = [path[i][0], path[i + 1][0]]
        seg_ys = [path[i][1], path[i + 1][1]]
        c = plt.cm.RdYlGn(norm_eng[i])
        ax.plot(seg_xs, seg_ys, color=c, linewidth=2)

    frags = ca.query_window((-20, -20, 50, 50))
    for frag in frags:
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        ax.fill(fx, fy, "steelblue", alpha=0.2)

    ax.plot(path[0][0], path[0][1], "o", color="green", markersize=8, zorder=5)
    ax.plot(path[-1][0], path[-1][1], "x", color="red", markersize=8, zorder=5)

    ax.set_aspect("equal")
    ax.set_xlim(-5, 35)
    ax.set_ylim(-5, 35)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_title("Corner Navigation: Solver Deflects to Maintain Engagement")
    ax.grid(True, alpha=0.3)

    fig.tight_layout()
    return fig


def generate_engagement_histogram():
    """Histogram of engagement variance along a curved wall."""
    tool_radius = 3.0
    step_len = 0.5
    target_eng = math.pi * 0.85

    # Cleared area with a curved top edge (sine wave).
    n = 100
    poly = [(-100.0, -100.0), (100.0, -100.0), (100.0, 20.0)]
    for i in range(n + 1):
        t = i / n
        x = 100.0 - 200.0 * t
        y = 20.0 + 5.0 * math.sin(x * math.pi / 50.0)
        poly.append((x, y))

    ca = ClearedArea()
    ca.add_cleared_polygons([poly])

    opts = StepperOptions()
    opts.radius = tool_radius
    opts.step_length = step_len
    opts.target_engagement = target_eng
    opts.max_deflection = 0.8
    path, _status = ca.run_segment((0, 20), 0.0, opts, 200)

    engagements = [_engagement_at(p, ca, tool_radius) for p in path]

    n_bins = min(30, len(set(round(e, 6) for e in engagements)))
    fig, ax = plt.subplots(figsize=(8, 4))
    ax.hist(engagements, bins=max(n_bins, 5), color="steelblue", alpha=0.7)
    ax.axvline(
        target_eng,
        color="red",
        linestyle="--",
        linewidth=2,
        label=f"Target ({target_eng:.2f} rad)",
    )
    ax.set_xlabel("Engagement angle (rad)")
    ax.set_ylabel("Count")
    ax.set_title(
        f"Engagement Distribution: σ = {np.std(engagements):.3f} rad, "
        f"mean = {np.mean(engagements):.3f} rad"
    )
    ax.legend(fontsize=9)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.cleared_area.md"]
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
        "heading": "signed_boundary_distance",
        "caption": (
            "Signed boundary distance around a cleared square: "
            "green = inside cleared, red = outside."
        ),
        "function": generate_signed_boundary_distance,
    },
    {
        "heading": "expand",
        "caption": (
            "``expand``: sweeping a disk along a multi-segment "
            "path enlarges the cleared area."
        ),
        "function": generate_expand,
    },
    {
        "heading": "remaining",
        "caption": (
            "``remaining`` subtracts cleared fragments from the "
            "boundary polygon, returning the uncut region (red)."
        ),
        "function": generate_remaining,
    },
    {
        "heading": "query_window",
        "caption": (
            "``query_window`` returns only the cleared fragments "
            "whose bounding box overlaps the query (green box)."
        ),
        "function": generate_query_window,
    },
    {
        "heading": "expand_step",
        "caption": (
            "``expand_step``: sweeping a disk (dashed circle) of radius "
            "*radius* from *prev* to *next* (red arrow) enlarges "
            "the cleared area (right) vs the initial state (left)."
        ),
        "function": generate_expand_step,
    },
    {
        "heading": "find_next_resume",
        "caption": (
            "``find_next_resume`` walks the cleared-area frontier from "
            "the end position (red triangle) and returns the first "
            "position with sufficient engagement (green star)."
        ),
        "function": generate_find_next_resume,
    },
    {
        "heading": "begin_step_batch",
        "caption": (
            "Three segments queued via ``begin_step_batch`` / "
            "``expand_step_batched`` then unioned in a single "
            "``commit_step_batch`` pass."
        ),
        "function": generate_step_batch,
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
    {
        "heading": None,
        "caption": "Tool stepping parallel to a straight wall. "
        "Path colour = engagement (green = on target).",
        "function": generate_wall_following,
    },
    {
        "heading": "step",
        "caption": "90° corner: the solver deflects the heading to keep "
        "engagement constant around the turn.",
        "function": generate_pocket_corner,
    },
    {
        "heading": "step",
        "caption": "Engagement histogram for 200 steps along a straight "
        "wall. Tight peak near target indicates stable behaviour.",
        "function": generate_engagement_histogram,
    },
]
