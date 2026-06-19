"""Generate geometry playground example images."""

__images__ = [
    {
        "stem": "geometry-playground",
        "caption": "Various geometry shapes and operations",
        "doc": ["raygeo.md", "raygeo.geo.md"],
        "heading": None,
    },
]

import math

import matplotlib.pyplot as plt

from raygeo.geo import Geometry
from tools.plot import auto_limits, plot_geometry


def generate_examples(output_dir):
    images = []

    fig, axes = plt.subplots(2, 3, figsize=(14, 9))
    axes_flat = axes.flatten()

    cases = [
        ("Rectangle", _make_rect()),
        ("Circle", _make_circle()),
        ("Polygon (regular)", _make_polygon()),
        ("Star", _make_star()),
        ("Grown (offset)", _make_offset()),
        ("Simplified", _make_simplified()),
    ]

    for ax, (title, geom) in zip(axes_flat, cases):
        plot_geometry(ax, geom, color="steelblue", show_points=False)
        xmin, xmax, ymin, ymax = auto_limits([geom])
        ax.set_xlim(xmin, xmax)
        ax.set_ylim(ymin, ymax)
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.set_title(title)

    fig.tight_layout()
    path = output_dir / "geometry-playground.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "geometry-playground.png",
            "caption": "Various geometry shapes and operations",
        }
    )

    return {
        "title": "Geometry Playground",
        "description": (
            "Build, transform, and analyze geometric shapes including "
            "rectangles, circles, polygons, stars, and more."
        ),
        "images": images,
    }


def _make_rect():
    return Geometry.from_points([(0, 0), (10, 0), (10, 10), (0, 10)])


def _make_circle():
    geom = Geometry()
    r = 10
    geom.move_to(r, 0, 0)
    geom.arc_to(-r, 0, -r, 0, True, 0)
    geom.arc_to(r, 0, r, 0, True, 0)
    return geom


def _make_polygon():
    n = 6
    r = 10
    return Geometry.from_points(
        [
            (
                r * math.cos(2 * math.pi * i / n),
                r * math.sin(2 * math.pi * i / n),
            )
            for i in range(n)
        ]
    )


def _make_star():
    outer_r = 10
    inner_r = 4
    points = 5
    coords = []
    for i in range(points * 2):
        a = math.pi / 2 + math.pi * i / points
        rd = outer_r if i % 2 == 0 else inner_r
        coords.append((rd * math.cos(a), rd * math.sin(a)))
    return Geometry.from_points(coords)


def _make_offset():
    g = Geometry.from_points([(0, 0), (10, 0), (10, 10), (0, 10)])
    return g.grow(2)


def _make_simplified():
    g = Geometry.from_points(
        [
            (0, 0),
            (0.5, 0.01),
            (1, 0),
            (10, 0),
            (10, 10),
            (0, 10),
        ]
    )
    return g.simplify(0.5)
