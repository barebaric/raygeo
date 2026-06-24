"""Profile adaptive_peeling using the multi-island example case."""

import math

from raygeo.ops.assembly.hsm import adaptive_entry, adaptive_peeling
from raygeo.ops.cleared_area import ClearedArea

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
step_over = 0.1

print("=== Running adaptive_entry... ===")
_, cp = adaptive_entry(
    pocket_boundary=boundary,
    islands=islands,
    tool_radius=tool_radius,
    step_over=step_over,
    safe_z=2.0,
    target_z=-5.0,
    plunge_pitch=1.0,
)

print("=== Running adaptive_peeling (profiling output below)... ===")
ops = adaptive_peeling(
    cleared=ClearedArea(initial=cp),
    pocket_boundary=boundary,
    islands=islands,
    tool_radius=tool_radius,
    step_over=step_over,
    cut_z=-5.0,
    safe_z=5.0,
)

print(f"Ops commands generated: {len(ops)}")
