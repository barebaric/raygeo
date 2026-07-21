"""Generate visualisations of clearing workplan execution."""

import matplotlib.pyplot as plt
from matplotlib.patches import Polygon as PolygonPatch

from raygeo.cnc.execution.intent import create_intent, run_intent
from raygeo.cnc.plan.clearing import plan_clearing
from raygeo.ops.feature.narrow import analyze_pocket
from raygeo.ops.feature.region import find_regions
from raygeo.ops.part import Part
from tools.plot import plot_ops_2d


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def _hshape(corridor_w):
    """Left lobe + dead-end corridor of given width."""
    cy = 20 - corridor_w / 2
    return [
        (0, 0),
        (30, 0),
        (30, cy),
        (50, cy),
        (50, cy + corridor_w),
        (30, cy + corridor_w),
        (30, 40),
        (0, 40),
    ]


def _dumbbell(corridor_w):
    """Two lobes connected by a corridor of given width."""
    cy = 20 - corridor_w / 2
    return [
        (0, 0),
        (30, 0),
        (30, cy),
        (50, cy),
        (50, 0),
        (80, 0),
        (80, 40),
        (50, 40),
        (50, cy + corridor_w),
        (30, cy + corridor_w),
        (30, 40),
        (0, 40),
    ]


def _build_and_execute(boundary, islands=None, **kwargs):
    islands = islands or []
    part = Part.from_polygons(boundary, islands, (0.0, 0.0))
    plan = plan_clearing(part, "", **kwargs)
    intent = create_intent(plan, part, 0)
    ops = run_intent(intent)
    return plan, ops


def _step_summary(steps):
    seen = []
    for s in steps:
        k = s.kind
        if k not in seen:
            seen.append(k)
    return " + ".join(seen)


def _annotate_regions(ax, boundary, islands, tool_radius):
    """Draw region/passage overlays with labels."""
    regions = find_regions(
        boundary=boundary, islands=islands, tool_radius=tool_radius
    )
    for i, reg in enumerate(regions):
        poly, area, entry_pt, r_max = reg
        patch = PolygonPatch(
            poly,
            facecolor="#4C72B0",
            edgecolor="#4C72B0",
            alpha=0.10,
            linewidth=0.8,
            zorder=2,
        )
        ax.add_patch(patch)
        ax.annotate(
            f"Region {i + 1}\n(r={r_max:.0f})",
            entry_pt,
            fontsize=7,
            ha="center",
            va="center",
            color="#4C72B0",
            fontweight="bold",
        )

    passages = analyze_pocket(
        polygon=boundary,
        holes=islands,
        tool_radius=tool_radius,
    )
    passage_colors = {
        "narrow": "#DD8452",
        "slot": "#C44E52",
        "unreachable": "#999999",
    }
    passage_labels = {
        "narrow": "Narrow",
        "slot": "Slot",
        "unreachable": "Unreachable",
    }
    for poly, cls, _w, _edges in passages:
        color = passage_colors.get(cls, "#888888")
        label = passage_labels.get(cls, cls)
        patch = PolygonPatch(
            poly,
            facecolor=color,
            edgecolor=color,
            alpha=0.15,
            linewidth=0.8,
            zorder=2,
        )
        ax.add_patch(patch)
        xs = [p[0] for p in poly]
        ys = [p[1] for p in poly]
        cx = sum(xs) / len(xs)
        cy = sum(ys) / len(ys)
        ax.annotate(
            label,
            (cx, cy),
            fontsize=6,
            ha="center",
            va="center",
            color=color,
            fontweight="bold",
        )


def generate_clearing():
    """2x3 grid: dumbbells (row 1), dead-end passages (row 2)."""
    common = dict(
        tool_radius=3.0,
        step_over=2.0,
        step_length=0.6,
        target_z=-5.0,
        safe_z=2.0,
        area_tolerance=5.0,
        wall_margin=0.5,
        finishing=True,
    )

    nar_db = _dumbbell(8.5)
    nar_db_plan, nar_db_ops = _build_and_execute(nar_db, **common)

    slot_db = _dumbbell(6.2)
    slot_db_plan, slot_db_ops = _build_and_execute(slot_db, **common)

    db_un = _dumbbell(5.0)
    db_un_plan, db_un_ops = _build_and_execute(db_un, **common)

    nar_de = _hshape(8.5)
    nar_de_plan, nar_de_ops = _build_and_execute(nar_de, **common)

    slot_de = _hshape(6.2)
    slot_de_plan, slot_de_ops = _build_and_execute(slot_de, **common)

    un_de = _hshape(5.0)
    un_de_plan, un_de_ops = _build_and_execute(un_de, **common)

    panels = [
        (nar_db, nar_db_plan, nar_db_ops, "Dumbbell narrow (8.5 mm)"),
        (slot_db, slot_db_plan, slot_db_ops, "Dumbbell slot (6.2 mm)"),
        (db_un, db_un_plan, db_un_ops, "Dumbbell unreachable (5 mm)"),
        (nar_de, nar_de_plan, nar_de_ops, "Dead-end narrow (8.5 mm)"),
        (slot_de, slot_de_plan, slot_de_ops, "Dead-end slot (6.2 mm)"),
        (un_de, un_de_plan, un_de_ops, "Dead-end unreachable (5 mm)"),
    ]

    fig, axes = plt.subplots(2, 3, figsize=(18, 12))

    for idx, (bnd, plan, ops, title) in enumerate(panels):
        row, col = divmod(idx, 3)
        ax = axes[row][col]
        if ops is not None:
            plot_ops_2d(ax, ops, boundary=bnd)
        _annotate_regions(ax, bnd, [], common["tool_radius"])
        ax.set_title(f"{title}\n{_step_summary(plan.steps)}", fontsize=9)

    for row in axes:
        for ax in row:
            if not ax.get_visible():
                continue
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
            "Clearing workplan: narrow passage (ToroidalClear),"
            " slot (Slot), dual-entry dumbbell (Unreachable)."
        ),
        "function": generate_clearing,
    },
]

if __name__ == "__main__":
    fig = generate_clearing()
    fig.savefig(
        "/tmp/cnc_machining_adaptive.png", dpi=150, bbox_inches="tight"
    )
    print("Saved /tmp/cnc_machining_adaptive.png")
