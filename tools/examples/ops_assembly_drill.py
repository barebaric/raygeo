"""Generate visualisations of drill cycle assembly."""

import matplotlib.pyplot as plt

from raygeo.ops.assembly.drill import (
    PeckParams,
    generate_countersink,
    generate_drill,
)
from raygeo.ops.part import Part
from raygeo.ops.state import State
from tools.plot import plot_ops_3d


def generate_drill_peck_cycle():
    """generate_drill — a full-retract peck cycle as explicit moves."""
    part = Part.from_polygons([])
    state = State(feed_rate=1000, plunge_feed=300, spindle_rpm=4000)
    result = generate_drill(
        part,
        center=(0.0, 0.0),
        diameter=5.0,
        target_z=-12.0,
        peck=PeckParams(increment=4.0),
        dwell_ms=100.0,
        state=state,
    )

    fig = plt.figure(figsize=(9, 8))
    ax = fig.add_subplot(111, projection="3d")
    plot_ops_3d(ax, result.ops)
    ax.set_title(
        "generate_drill — peck cycle: rapid to the plane, plunge with "
        "dwell, retract per peck"
    )
    fig.tight_layout()
    return fig


def generate_drill_chip_break_cycle():
    """generate_drill — a chip-break peck cycle stays in the hole."""
    part = Part.from_polygons([])
    state = State(feed_rate=1000, plunge_feed=300, spindle_rpm=4000)
    result = generate_drill(
        part,
        center=(0.0, 0.0),
        diameter=5.0,
        target_z=-12.0,
        tip_angle=90.0,
        peck=PeckParams(increment=4.0, mode="CHIP_BREAK", retract_amount=0.5),
        state=state,
    )

    fig = plt.figure(figsize=(9, 8))
    ax = fig.add_subplot(111, projection="3d")
    plot_ops_3d(ax, result.ops)
    ax.set_title(
        "generate_drill — chip-break cycle: small retracts break the "
        "chip without clearing the flutes"
    )
    fig.tight_layout()
    return fig


def generate_countersink_cycle():
    """generate_countersink — one conical plunge to the chamfer depth."""
    part = Part.from_polygons([])
    state = State(feed_rate=800, plunge_feed=250, spindle_rpm=3000)
    result = generate_countersink(
        part,
        center=(0.0, 0.0),
        tip_angle=90.0,
        target_diameter=8.0,
        state=state,
    )

    fig = plt.figure(figsize=(9, 8))
    ax = fig.add_subplot(111, projection="3d")
    plot_ops_3d(ax, result.ops)
    ax.set_title(
        "generate_countersink — a 90° cone plunged to produce an 8 mm "
        "chamfer at the surface"
    )
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.drill.md"]

__images__ = [
    {
        "heading": "generate_drill",
        "caption": (
            "Peck cycle as explicit moves: rapid to the rapid plane, "
            "feed plunges, full retracts, dwell."
        ),
        "function": generate_drill_peck_cycle,
    },
    {
        "heading": "generate_drill",
        "caption": (
            "Chip-break cycle: the tool only lifts a fraction of a "
            "millimetre between pecks."
        ),
        "function": generate_drill_chip_break_cycle,
    },
    {
        "heading": "generate_countersink",
        "caption": (
            "Countersink cycle: the cone depth derives from the target "
            "diameter and the tip angle."
        ),
        "function": generate_countersink_cycle,
    },
]
