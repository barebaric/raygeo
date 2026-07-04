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

    When *existing_fragments* is provided, begin with those as the
    initial cleared state (from a previously cached CA at *start_cut*
    cuts) and only expand cutting moves from *start_cut* onward.
    """
    if existing_fragments is None:
        ca = ClearedArea(
            boundary=list(geometry.boundary),
            islands=[list(isl) for isl in geometry.islands],
            initial=seed_polys,
        )
    else:
        ca = ClearedArea(
            boundary=list(geometry.boundary),
            islands=[list(isl) for isl in geometry.islands],
            initial=existing_fragments,
        )

    prev = None
    cut_count = 0
    for i in range(len(tp)):
        x, y, is_travel = tp[i]
        if is_travel:
            prev = (x, y)
            continue
        if prev is not None and cut_count >= start_cut:
            ca.expand_step(prev, (x, y), geometry.tool_radius)
            ca.compact_if_needed(0.1)
        prev = (x, y)
        cut_count += 1
        if cut_count >= n_cuts:
            break
    return ca
