"""`raygeo profile-cleared-area` — micro-benchmark for cleared-area
mutation paths and the `actionable_remaining` metric.

Runs a *deterministic*, seeded random walk of `expand_batched` +
`commit_batch_local` steps inside a configurable pocket.  At each
checkpoint, calls `actionable_remaining(tool_radius)` and reports
per-call timings.

Usage:
    raygeo profile-cleared-area
    raygeo profile-cleared-area --steps 1000 --seed 42
    raygeo profile-cleared-area --tool-radius 3.0 --pocket 100 80
    raygeo profile-cleared-area --islands 4
"""

import math
import random
import time

from raygeo.ops.cut import StockRegion
from raygeo.ops.cut.cleared_area import ClearedArea


def _add_pocket_args(p):
    p.add_argument(
        "--steps",
        type=int,
        default=500,
        help="Number of random walk steps (default: 500).",
    )
    p.add_argument(
        "--seed",
        type=int,
        default=42,
        help="Random seed for the walk (default: 42, deterministic).",
    )
    p.add_argument(
        "--tool-radius",
        type=float,
        default=3.0,
        help="Tool radius for the swept disc and envelope (default: 3.0).",
    )
    p.add_argument(
        "--pocket",
        type=float,
        nargs=2,
        default=(120.0, 120.0),
        metavar=("W", "H"),
        help="Pocket dimensions in mm (default: 120 120).",
    )
    p.add_argument(
        "--islands",
        type=int,
        default=3,
        choices=(0, 1, 2, 3, 4),
        help="Number of interior rectangular islands to add (default: 3).",
    )
    p.add_argument(
        "--checkpoint-every",
        type=int,
        default=50,
        help="Sample actionable_remaining every N steps (default: 50).",
    )


def register(subparsers):
    p = subparsers.add_parser(
        "profile-cleared-area",
        help=(
            "Micro-benchmark ClearedArea mutations and the "
            "actionable_remaining metric on a deterministic seeded walk."
        ),
    )
    _add_pocket_args(p)
    p.set_defaults(func=run)


def _build_islands(pocket_w, pocket_h, n, rng, island_size=20.0):
    """Deterministically (via rng) scatter n rectangular islands
    inside the pocket without overlapping walls or each other."""
    islands = []
    if n == 0:
        return islands
    margin = island_size  # avoid the wall band
    centers = []
    for _ in range(1000):
        if len(centers) == n:
            break
        cx = rng.uniform(margin, pocket_w - margin)
        cy = rng.uniform(margin, pocket_h - margin)
        # Keep islands disjoint
        ok = True
        for ix, iy in centers:
            if (
                abs(cx - ix) < 2 * island_size
                and abs(cy - iy) < 2 * island_size
            ):
                ok = False
                break
        if not ok:
            continue
        centers.append((cx, cy))
        half = island_size * 0.5
        islands.append(
            [
                (cx - half, cy - half),
                (cx + half, cy - half),
                (cx + half, cy + half),
                (cx - half, cy + half),
            ]
        )
    return islands


def run(args):
    pocket_w, pocket_h = args.pocket
    r = args.tool_radius
    seed = args.seed

    # Deterministic seed for island placement (separate from walk seed).
    island_rng = random.Random(seed + 1)
    islands = _build_islands(pocket_w, pocket_h, args.islands, island_rng)

    boundary = [
        (0.0, 0.0),
        (pocket_w, 0.0),
        (pocket_w, pocket_h),
        (0.0, pocket_h),
    ]

    region = StockRegion(boundary=boundary, islands=islands)
    ca = ClearedArea()

    # Deterministic walk seed.
    walk_rng = random.Random(seed)
    x = pocket_w * 0.5
    y = pocket_h * 0.5

    # Keep the tool centre inside the envelope inset (boundary − r).
    inset = r

    print(f"\n--- profile-cleared-area ({args.steps} steps) ---")
    print(
        f"Pocket: {pocket_w:g} × {pocket_h:g} mm, "
        f"{len(islands)} islands, tool_radius={r:g}, seed={seed}"
    )

    # Warm-up a single call to JIT the envelope cache.
    _ = ca.actionable_remaining(region, r)

    step_total = 0.0
    actionable_total = 0.0
    actionable_calls = 0
    actionable_max = 0.0
    samples = []

    for i in range(args.steps):
        # Generate next step (one mm of motion at a random angle).
        angle = walk_rng.uniform(0.0, 2.0 * math.pi)
        nx = x + math.cos(angle)
        ny = y + math.sin(angle)
        # Clip inside the envelope inset
        nx = max(inset, min(pocket_w - inset, nx))
        ny = max(inset, min(pocket_h - inset, ny))

        t = time.perf_counter()
        ca.begin_batch()
        ca.expand_batched((x, y), (nx, ny), r)
        ca.commit_batch_local()
        step_total += time.perf_counter() - t

        x, y = nx, ny

        if i % args.checkpoint_every == 0 or i == args.steps - 1:
            t = time.perf_counter()
            ar = ca.actionable_remaining(region, r)
            dt = time.perf_counter() - t
            actionable_total += dt
            actionable_calls += 1
            actionable_max = max(actionable_max, dt)
            samples.append((i, ar, dt))

    print(f"Wall clock (steps):            {step_total * 1000:.1f} ms")
    print(
        "  per-step:"
        f"                    {step_total / args.steps * 1000:.3f} ms"
    )
    print(f"actionable_remaining calls:    {actionable_calls}")
    print(f"  total:                       {actionable_total * 1000:.1f} ms")
    print(
        "  per-call:"
        f"                    {actionable_total / actionable_calls * 1e6:.1f}"
        " µs"
    )
    print(f"  max call:                    {actionable_max * 1e6:.1f} µs")

    # Print a few samples
    print("\nidx  actionable_remaining  call µs")
    print("----  -------------------  ------")
    for i, ar, dt in samples[:8]:
        print(f"{i:4d}  {ar:19.3f}  {dt * 1e6:7.1f}")
    if len(samples) > 8:
        print("  ...")
        for i, ar, dt in samples[-3:]:
            print(f"{i:4d}  {ar:19.3f}  {dt * 1e6:7.1f}")
    print()
