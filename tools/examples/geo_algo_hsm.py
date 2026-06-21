"""Generate 3D visualisation of HSM adaptive clearing."""

import math

import matplotlib.pyplot as plt

from raygeo.geo.algo.cleared_area import ClearedArea
from raygeo.geo.algo.hsm import (
    fillet_arc_ends,
    find_cutting_arc,
    find_safe_sweep_end,
)
from raygeo.geo.algo.offset import compute_inset_region
from raygeo.ops.assembly.hsm import adaptive_entry


def generate_find_cutting_arc():
    """Show cutting arcs from ten iterations of peeling."""
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

    # Run several iterations, collecting all cutting arcs
    all_arcs = []
    for _ in range(55):
        bites = ca.bites(step_over, va, 0.01)
        if not bites:
            break
        for bite in bites:
            arc = find_cutting_arc(bite, ca.fragments())
            if arc:
                all_arcs.append(arc)
        ca.incorporate(bites)
        if ca.total_area() >= total - 0.1:
            break

    fig, ax = plt.subplots(figsize=(7, 6))
    ax.set_aspect("equal")

    # pocket boundary
    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, alpha=0.3, label="Boundary")

    # islands
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

    # colour arcs by iteration (first → dark, later → pale)
    n = len(all_arcs)
    for idx, arc in enumerate(all_arcs):
        t = idx / max(n - 1, 1)
        r = 0.9 - 0.6 * t
        g = 0.2 + 0.5 * t
        color = (r, g, 0.2)
        ax.plot(
            [p[0] for p in arc],
            [p[1] for p in arc],
            color=color,
            linewidth=2.0,
            alpha=0.85,
        )

    ax.set_title(f"Cutting arcs from {n} passes")
    fig.tight_layout()
    return fig


def generate_fillet_arc_ends():
    """Show cutting arcs with filleted ends flowing into the frontier."""
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

    all_arcs = []
    for _ in range(55):
        bites = ca.bites(step_over, va, 0.01)
        if not bites:
            break
        for bite in bites:
            arc = find_cutting_arc(bite, ca.fragments())
            if arc and len(arc) >= 3:
                safe = find_safe_sweep_end(arc, boundary, islands, tool_radius)
                if safe:
                    fa = fillet_arc_ends(arc, boundary, islands, tool_radius)
                    all_arcs.append((arc, safe, fa))
        ca.incorporate(bites)
        if ca.total_area() >= total - 0.1:
            break

    fig, ax = plt.subplots(figsize=(7, 6))
    ax.set_aspect("equal")

    # pocket boundary
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

    raw_color = "#1f77b4"
    fillet_color = "#e41a1c"
    cross_color = "#2ca02c"
    for idx, (arc, safe, fa) in enumerate(all_arcs):
        ax.plot(
            [p[0] for p in arc],
            [p[1] for p in arc],
            color=raw_color,
            linewidth=1.5,
            alpha=0.4,
        )
        ax.plot(
            [p[0] for p in fa],
            [p[1] for p in fa],
            color=fillet_color,
            linewidth=2.5,
            alpha=0.9,
            label="Trimmed" if idx == 0 else "",
        )
        if safe is not None:
            enter, exit_pt = safe
            ax.plot(
                enter[0],
                enter[1],
                "o",
                color=cross_color,
                markersize=3,
                label="Enter" if idx == 0 else "",
            )
            ax.plot(
                exit_pt[0],
                exit_pt[1],
                "s",
                color=cross_color,
                markersize=3,
                label="Exit" if idx == 0 else "",
            )

    ax.set_title("Cutting arcs trimmed at 2×tool_radius from frontier")
    ax.legend(loc="upper right", fontsize=8)
    fig.tight_layout()
    return fig


def generate_find_cutting_arc_simple():
    """Show cutting arcs without islands."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    tool_radius = 3.0
    step_over = 2.0

    _, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=tool_radius,
        step_over=step_over,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)
    va, total = compute_inset_region(boundary, tool_radius, [])

    all_arcs = []
    for _ in range(55):
        bites = ca.bites(step_over, va, 0.01)
        if not bites:
            break
        for bite in bites:
            arc = find_cutting_arc(bite, ca.fragments())
            if arc:
                all_arcs.append(arc)
        ca.incorporate(bites)
        if ca.total_area() >= total - 0.1:
            break

    fig, ax = plt.subplots(figsize=(7, 6))
    ax.set_aspect("equal")

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, alpha=0.3, label="Boundary")

    n = len(all_arcs)
    for idx, arc in enumerate(all_arcs):
        t = idx / max(n - 1, 1)
        r = 0.9 - 0.6 * t
        g = 0.2 + 0.5 * t
        color = (r, g, 0.2)
        ax.plot(
            [p[0] for p in arc],
            [p[1] for p in arc],
            color=color,
            linewidth=2.0,
            alpha=0.85,
        )

    ax.set_title(f"Cutting arcs from {n} passes (no islands)")
    fig.tight_layout()
    return fig


def generate_fillet_arc_ends_simple():
    """Show filleted cutting arcs without islands."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    islands = []
    tool_radius = 3.0
    step_over = 2.0

    _, cp = adaptive_entry(
        pocket_boundary=boundary,
        tool_radius=tool_radius,
        step_over=step_over,
        safe_z=2.0,
        target_z=-5.0,
        plunge_pitch=1.0,
    )
    ca = ClearedArea(initial=cp)
    va, total = compute_inset_region(boundary, tool_radius, [])

    all_arcs = []
    for _ in range(55):
        bites = ca.bites(step_over, va, 0.01)
        if not bites:
            break
        for bite in bites:
            arc = find_cutting_arc(bite, ca.fragments())
            if arc and len(arc) >= 3:
                safe = find_safe_sweep_end(arc, boundary, islands, tool_radius)
                if safe:
                    fa = fillet_arc_ends(arc, boundary, islands, tool_radius)
                    all_arcs.append((arc, safe, fa))
        ca.incorporate(bites)
        if ca.total_area() >= total - 0.1:
            break

    fig, ax = plt.subplots(figsize=(7, 6))
    ax.set_aspect("equal")

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, alpha=0.3, label="Boundary")

    raw_color = "#1f77b4"
    fillet_color = "#e41a1c"
    cross_color = "#2ca02c"
    for idx, (arc, safe, fa) in enumerate(all_arcs):
        ax.plot(
            [p[0] for p in arc],
            [p[1] for p in arc],
            color=raw_color,
            linewidth=1.5,
            alpha=0.4,
        )
        ax.plot(
            [p[0] for p in fa],
            [p[1] for p in fa],
            color=fillet_color,
            linewidth=2.5,
            alpha=0.9,
            label="Trimmed" if idx == 0 else "",
        )
        if safe is not None:
            enter, exit_pt = safe
            ax.plot(
                enter[0],
                enter[1],
                "o",
                color=cross_color,
                markersize=3,
                label="Enter" if idx == 0 else "",
            )
            ax.plot(
                exit_pt[0],
                exit_pt[1],
                "s",
                color=cross_color,
                markersize=3,
                label="Exit" if idx == 0 else "",
            )

    ax.set_title("Filleted cutting arcs (no islands)")
    ax.legend(loc="upper right", fontsize=8)
    fig.tight_layout()
    return fig


def generate_find_safe_sweep_end():
    """Show cutting arcs trimmed by iterative sweep shortening."""
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

    all_crossings = []
    for _ in range(55):
        bites = ca.bites(step_over, va, 0.01)
        if not bites:
            break
        for bite in bites:
            arc = find_cutting_arc(bite, ca.fragments())
            if arc and len(arc) >= 3:
                safe = find_safe_sweep_end(arc, boundary, islands, tool_radius)
                if safe:
                    all_crossings.append((arc, safe))
        ca.incorporate(bites)
        if ca.total_area() >= total - 0.1:
            break

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

    for idx, (arc, (enter, exit_pt)) in enumerate(all_crossings):
        ax.plot(
            [p[0] for p in arc],
            [p[1] for p in arc],
            color="#1f77b4",
            linewidth=1.5,
            alpha=0.25,
        )
        ax.plot(
            enter[0],
            enter[1],
            "o",
            color="#e41a1c",
            markersize=2,
            label="Enter" if idx == 0 else "",
        )
        ax.plot(
            exit_pt[0],
            exit_pt[1],
            "s",
            color="#e41a1c",
            markersize=2,
            label="Exit" if idx == 0 else "",
        )

    ax.set_title("Cutting arcs with safe entry/exit points")
    ax.legend(loc="upper right", fontsize=8)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.hsm.md"]
__images__ = [
    {
        "heading": "find_cutting_arc",
        "caption": (
            "Bite polygons from the first peeling iteration with the"
            " cutting arc (outer edge) highlighted in red — the cleared"
            " area is shown in blue"
        ),
        "function": generate_find_cutting_arc,
    },
    {
        "heading": "find_cutting_arc",
        "caption": ("Cutting arcs from passes without islands"),
        "function": generate_find_cutting_arc_simple,
    },
    {
        "heading": "fillet_arc_ends",
        "caption": (
            "Cutting arcs (blue) with their ends rounded (red) to"
            " flow tangentially into the frontier"
        ),
        "function": generate_fillet_arc_ends,
    },
    {
        "heading": "fillet_arc_ends",
        "caption": ("Filleted cutting arcs without islands"),
        "function": generate_fillet_arc_ends_simple,
    },
    {
        "heading": "find_safe_sweep_end",
        "caption": (
            "Cutting arcs trimmed (red) by iterative sweep shortening"
            " until the tool sweep no longer collides with the boundary"
            " or islands — original arc shown in blue"
        ),
        "function": generate_find_safe_sweep_end,
    },
]
