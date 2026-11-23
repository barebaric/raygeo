"""Generate DXF reading and writing example images."""

import math

import matplotlib.pyplot as plt

from raygeo.dxf import read, write
from raygeo.geo import Geometry
from tools.plot import plot_geometry

# tan(90 deg / 4): the bulge factor that turns a segment into a quarter
# turn, which is what a rounded corner is.
QUARTER_TURN = math.tan(math.radians(90.0) / 4.0)


def _arc_points(radius, start_deg, end_deg, center=(0.0, 0.0)):
    """Points along a circular arc, for drawing expected geometry."""
    steps = 48
    return [
        (
            center[0]
            + radius
            * math.cos(
                math.radians(start_deg + (end_deg - start_deg) * i / steps)
            ),
            center[1]
            + radius
            * math.sin(
                math.radians(start_deg + (end_deg - start_deg) * i / steps)
            ),
        )
        for i in range(steps + 1)
    ]


def _minimal_dxf(entities):
    """Wrap entity definitions in a minimal R12 DXF document."""
    lines = [
        "0",
        "SECTION",
        "2",
        "HEADER",
        "9",
        "$INSUNITS",
        "70",
        "4",
        "0",
        "ENDSEC",
    ]
    lines += ["0", "SECTION", "2", "ENTITIES"]
    for kind, layer, values in entities:
        lines += ["0", kind, "8", layer]
        for code, value in values:
            lines += [str(code), str(value)]
    lines += ["0", "ENDSEC", "0", "EOF"]
    return "\n".join(lines) + "\n"


def generate_shapes():
    """Read one of each common entity type."""
    document = _minimal_dxf(
        [
            ("LINE", "outline", [(10, 0), (20, 0), (11, 40), (21, 0)]),
            (
                "ARC",
                "outline",
                [
                    (10, 20),
                    (20, 20),
                    (40, 10),
                    (50, 0),
                    (51, 180),
                ],
            ),
            ("CIRCLE", "holes", [(10, 18), (20, 34), (40, 4)]),
            (
                "LWPOLYLINE",
                "slots",
                # A rounded rectangle: straight edges with a quarter-turn
                # bulged corner between each pair of them.
                [
                    (90, 8),
                    (70, 1),
                    (10, 17),
                    (20, 12),
                    (42, 0.0),
                    (10, 25),
                    (20, 12),
                    (42, QUARTER_TURN),
                    (10, 30),
                    (20, 17),
                    (42, 0.0),
                    (10, 30),
                    (20, 25),
                    (42, QUARTER_TURN),
                    (10, 25),
                    (20, 30),
                    (42, 0.0),
                    (10, 17),
                    (20, 30),
                    (42, QUARTER_TURN),
                    (10, 12),
                    (20, 25),
                    (42, 0.0),
                    (10, 12),
                    (20, 17),
                    (42, QUARTER_TURN),
                ],
            ),
        ]
    )
    output = read(document)

    fig, ax = plt.subplots(figsize=(8, 8))
    colors = {
        "outline": "steelblue",
        "holes": "tomato",
        "slots": "forestgreen",
    }
    for name, geometry in output.layers:
        plot_geometry(
            ax, geometry, color=colors.get(name, "gray"), linewidth=2
        )
        ax.plot([], [], color=colors.get(name, "gray"), label=f"Layer {name}")

    ax.add_patch(
        plt.Rectangle(
            (output.bounds.min_x, output.bounds.min_y),
            output.bounds.width,
            output.bounds.height,
            fill=False,
            edgecolor="gray",
            linestyle="--",
            linewidth=1,
            label="Bounds",
        )
    )
    xmin, ymin, xmax, ymax = (
        output.bounds.min_x,
        output.bounds.min_y,
        output.bounds.max_x,
        output.bounds.max_y,
    )
    ax.set_xlim(xmin - 5, xmax + 5)
    ax.set_ylim(ymin - 5, ymax + 5)
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.set_title("DXF entities read and grouped by layer", fontsize=13)
    ax.legend(fontsize=10, loc="upper right")
    fig.tight_layout()
    return fig


def generate_write_roundtrip():
    """Write geometry, read it back, and compare."""
    geometry = Geometry()
    geometry.move_to(0, 0)
    geometry.line_to(80, 0)
    geometry.line_to(80, 50)
    geometry.line_to(0, 50)
    geometry.close_path()
    geometry.move_to(110, 25)
    geometry.arc_to(150, 65, 0, 40, False, 0)
    geometry.move_to(40, 90)
    geometry.bezier_to(25, 115, 55, 115, 40, 140)

    reread = read(write(geometry)).layers[0][1]

    fig, (left, right) = plt.subplots(1, 2, figsize=(14, 6))
    plot_geometry(left, geometry, color="steelblue", linewidth=2.5)
    left.set_title("Original geometry", fontsize=13)

    plot_geometry(right, reread, color="forestgreen", linewidth=2.5)
    right.set_title("Re-read after DXF round trip", fontsize=13)

    for axis in (left, right):
        axis.set_xlim(-10, 165)
        axis.set_ylim(-10, 155)
        axis.set_aspect("equal")
        axis.grid(True, alpha=0.3)

    fig.tight_layout()
    return fig


def generate_arc_accuracy():
    """Show how accurately arcs survive the round trip."""
    start_deg, end_deg = 30.0, 200.0
    radius = 40.0
    geometry = Geometry()
    sx = radius * math.cos(math.radians(start_deg))
    sy = radius * math.sin(math.radians(start_deg))
    ex = radius * math.cos(math.radians(end_deg))
    ey = radius * math.sin(math.radians(end_deg))
    geometry.move_to(sx, sy)
    geometry.arc_to(ex, ey, -sx, -sy, False, 0)

    reread = read(write(geometry)).layers[0][1]

    fig, ax = plt.subplots(figsize=(8, 8))
    expected = _arc_points(radius, start_deg, end_deg)
    ax.plot(
        [p[0] for p in expected],
        [p[1] for p in expected],
        "--",
        color="gray",
        linewidth=3,
        label="Ideal arc",
    )
    plot_geometry(ax, reread, color="tomato", linewidth=1.5, label="Re-read")
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.set_title(
        f"Arc of {end_deg - start_deg:.0f}° survives as an ARC entity",
        fontsize=13,
    )
    ax.legend(fontsize=10)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.dxf.md"]
__images__ = [
    {
        "heading": "read",
        "caption": "Lines, arcs, circles and bulges grouped by layer",
        "function": generate_shapes,
    },
    {
        "heading": "write",
        "caption": "Geometry written as DXF and read back",
        "function": generate_write_roundtrip,
    },
    {
        "heading": "write",
        "caption": "Arcs stay as ARC entities, so a round trip is lossless",
        "function": generate_arc_accuracy,
    },
]
