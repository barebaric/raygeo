from raygeo.ops.cut import StockRegion
from raygeo.ops.cut.cleared_area import ClearedArea

# ── ClearedArea rebuild ──────────────────────────────────────────


def rebuild_cleared(
    n_cuts,
    tp,
    seed_polys,
    geometry,
    existing_fragments=None,
    start_cut=0,
):
    """Build a ClearedArea expanded up to the Nth cutting move.

    *tp* is a list of ``(x, y, move_kind)`` where ``move_kind`` is a
    string (``"cut"`` for material-removing moves).  When
    *existing_fragments* is provided, begin with those as the initial
    cleared state (from a previously cached CA at *start_cut* cuts) and
    only expand cutting moves from *start_cut* onward.
    """
    boundary_pts = [tuple(p) for p in geometry["boundary"]]
    islands_pts = [[tuple(p) for p in isl] for isl in geometry["islands"]]
    region = StockRegion(boundary=boundary_pts, islands=islands_pts)
    seed_pts = [[tuple(p) for p in poly] for poly in seed_polys]
    if existing_fragments is None:
        ca = ClearedArea(initial=seed_pts)
    else:
        ca = ClearedArea(initial=existing_fragments)

    prev = None
    cut_count = 0
    for i in range(len(tp)):
        x, y, move_kind = tp[i]
        if move_kind != "cut":
            prev = (x, y)
            continue
        if prev is not None and cut_count >= start_cut:
            ca.expand_step(prev, (x, y), geometry["tool_radius"])
            ca.compact_if_needed(region, 0.1)
        prev = (x, y)
        cut_count += 1
        if cut_count >= n_cuts:
            break
    return ca
