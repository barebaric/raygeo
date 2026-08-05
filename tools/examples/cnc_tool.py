"""Generate visualisations of common CNC tool shapes."""

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import numpy as np

from raygeo.cnc.tool import Tool, ToolCategory, ToolMaterial, ToolModel

__docs_target__ = ["raygeo.cnc.tool.md"]


def _arc_points(cx, cy, r, a0, a1, n=32):
    angles = np.linspace(a0, a1, n)
    return [(cx + r * np.cos(a), cy + r * np.sin(a)) for a in angles]


def _draw_endmill(ax, diameter, ceh, overall):
    r = diameter / 2
    ax.add_patch(mpatches.Rectangle((-r, 0), diameter, ceh, fc=".8", ec="k"))
    ax.add_patch(
        mpatches.Rectangle((-r, ceh), diameter, overall - ceh, fc=".6", ec="k")
    )


def _draw_ballnose(ax, diameter, ceh, overall):
    r = diameter / 2
    ax.add_patch(mpatches.Wedge((0, r), r, 180, 360, fc=".8", ec="k"))
    ax.add_patch(
        mpatches.Rectangle((-r, r), diameter, ceh - r, fc=".8", ec="k")
    )
    ax.add_patch(
        mpatches.Rectangle((-r, ceh), diameter, overall - ceh, fc=".6", ec="k")
    )


def _draw_bullnose(ax, diameter, corner_radius, ceh, overall):
    r = diameter / 2
    cr = corner_radius
    fb = r - cr  # half-width of the flat bottom
    verts = [(-fb, 0), (fb, 0)]
    # Right corner: arc from (fb, 0) up to (r, cr), centre (fb, cr).
    verts += _arc_points(fb, cr, cr, -np.pi / 2, 0)
    verts += [(r, ceh), (-r, ceh), (-r, cr)]
    # Left corner: arc from (-r, cr) down to (-fb, 0), centre (-fb, cr).
    verts += _arc_points(-fb, cr, cr, np.pi, 3 * np.pi / 2)
    ax.add_patch(mpatches.Polygon(verts, closed=True, fc=".8", ec="k"))
    ax.add_patch(
        mpatches.Rectangle((-r, ceh), diameter, overall - ceh, fc=".6", ec="k")
    )


def _annotate(ax, diameter, ceh, overall, x_offset):
    ax.plot([-diameter / 2, diameter / 2], [ceh, ceh], "--", lw=0.8)
    ax.text(x_offset, ceh, f"CEH {ceh:g}", fontsize=8, color="tab:blue")
    ax.text(x_offset, overall, f"L {overall:g}", fontsize=8, color="tab:blue")
    ax.text(x_offset, -2, f"Ø{diameter:g}", fontsize=9, ha="center")


def _category_name(category):
    return str(category).split(".")[-1]


def generate_tool_shapes():
    """Schematic side profiles of the three common cutting-tool shapes."""
    specs = [
        (
            "End Mill",
            ToolCategory.EndMill,
            dict(
                diameter=6,
                cutting_edge_height=15,
                overall_length=50,
                shank_diameter=6,
                flute_count=3,
            ),
        ),
        (
            "Ball Nose",
            ToolCategory.BallNose,
            dict(
                diameter=6,
                cutting_edge_height=15,
                overall_length=50,
                shank_diameter=6,
                flute_count=2,
            ),
        ),
        (
            "Bull Nose",
            ToolCategory.BullNose,
            dict(
                diameter=8,
                corner_radius=1,
                cutting_edge_height=15,
                overall_length=50,
                shank_diameter=8,
                flute_count=4,
            ),
        ),
    ]

    fig, axes = plt.subplots(1, len(specs), figsize=(12, 5))
    for (title, category, params), ax in zip(specs, axes, strict=False):
        model = ToolModel(**params)
        Tool(
            label=title,
            category=category,
            model=model,
            material=ToolMaterial.Carbide,
            stickout=model.cutting_edge_height() + 3,
        )
        diameter = model.diameter()
        ceh = model.cutting_edge_height()
        overall = model.get_parameter("overall_length")
        assert overall is not None
        if category is ToolCategory.EndMill:
            _draw_endmill(ax, diameter, ceh, overall)
        elif category is ToolCategory.BallNose:
            _draw_ballnose(ax, diameter, ceh, overall)
        else:
            _draw_bullnose(ax, diameter, model.corner_radius(), ceh, overall)
        _annotate(ax, diameter, ceh, overall, x_offset=diameter)
        ax.set_title(f"{title}\ncategory={_category_name(category)}")
        ax.set_aspect("equal")
        ax.set_xlim(-diameter, 2 * diameter + 2)
        ax.set_ylim(-3, overall + 4)
        ax.axis("off")

    fig.tight_layout()
    return fig


__images__ = [
    {
        "heading": None,
        "caption": (
            "Side profiles of the three common cutting-tool categories, "
            "built as ToolModel parameter bags."
        ),
        "function": generate_tool_shapes,
    },
]


if __name__ == "__main__":
    fig = generate_tool_shapes()
    fig.savefig("cnc_tool_shapes.png", dpi=120)
    print("wrote cnc_tool_shapes.png")
