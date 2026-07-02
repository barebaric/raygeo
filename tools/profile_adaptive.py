#!/usr/bin/env python3
"""Profile `adaptive_clearing` — measure per-function CPU time."""

import math
import sys
import time

sys.path.insert(0, "tools/examples")

from raygeo.ops.assembly.adaptive import adaptive_clearing
from raygeo.ops.cut.cleared_area import ClearedArea


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _seed_circle(cx, cy, r, n=64):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


_TOOL_RADIUS = 1.0
_ADVANCE = 0.1
_STEP_LENGTH = 0.1


def main():
    boundary = _rect(0, 0, 60, 60)
    islands = [_rect(5, 0, 10, 10)]
    seed = [_seed_circle(-13.7, 13.7, 12.2)]
    ca = ClearedArea(
        boundary=boundary,
        islands=islands,
        initial=seed,
    )

    t0 = time.perf_counter()
    ops = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        islands=islands,
        radius=_TOOL_RADIUS,
        advance=_ADVANCE,
        cut_z=-5.0,
        safe_z=2.0,
        step_length=_STEP_LENGTH,
        profile=True,
    )
    t1 = time.perf_counter()

    cut = sum(1 for i in range(ops.len()) if ops.is_cutting(i))
    travel = sum(1 for i in range(ops.len()) if ops.is_travel(i))

    print("\n--- adaptive_clearing profile (60x60 centre-island) ---")
    print(f"Wall clock:  {t1 - t0:.2f}s")
    print(f"Cut points:  {cut}")
    print(f"Travel ops:  {travel}")
    print()


if __name__ == "__main__":
    main()
