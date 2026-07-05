import math

from raygeo.trace import TraceFile, get_route_detail_name


def register(subparsers):
    p = subparsers.add_parser(
        "print", help="Dump all trace records as an event log."
    )
    p.add_argument("tracefile", help="Input path for the .bin trace file.")
    p.set_defaults(func=run)


def run(args):
    """Dump all trace records as a human-readable event log."""
    trace_path = args.tracefile
    print(f"Trace file: {trace_path}")

    trace = TraceFile(trace_path)
    n = len(trace)
    print(f"Records: {n}")
    print()

    geo = trace.geometry
    tp = trace.toolpath
    print("Geometry:")
    print(f"  tool_radius={geo['tool_radius']}")
    print(f"  boundary: {len(geo['boundary'])} verts")
    print(f"  islands: {len(geo['islands'])}")
    print(f"  seeds: {len(geo['seeds'])} polygon(s)")
    print(f"  toolpath: {len(tp)} moves")
    print()

    RESUME_KINDS = {"resume_stall", "resume_stuck", "exit"}

    for i in range(n):
        rec = trace[i]
        if rec.kind in ("geometry", "mat"):
            continue
        kind_name = rec.kind
        status_name = rec.status.name

        h_deg = math.degrees(rec.heading)
        sh_deg = math.degrees(rec.smoothed_heading)
        pa_deg = math.degrees(rec.predicted_angle)
        ia_deg = math.degrees(rec.iteration_angle)
        eng_deg = math.degrees(rec.eng_angle)
        step_dist = math.hypot(rec.pos_x - rec.prev_x, rec.pos_y - rec.prev_y)

        route_src = ""
        if rec.route_source.value:
            route_src = f" route={rec.route_source.name}"

        resume_src = ""
        if rec.kind in RESUME_KINDS and rec.resume_source.value:
            resume_src = f" resume_via={rec.resume_source.name}"

        print(
            f"{i}\t{kind_name}\t{status_name}{route_src}{resume_src}"
            f"\tpos=({rec.pos_x:.4f},{rec.pos_y:.4f})"
            f"\tprev=({rec.prev_x:.4f},{rec.prev_y:.4f})"
            f"\tdist={step_dist:.4f}"
            f"\thdg={h_deg:.4f}"
            f"\tsmooth={sh_deg:.4f}"
            f"\tpred={pa_deg:.4f}"
            f"\titer={ia_deg:.4f}"
            f"\teng_angle={eng_deg:.4f}"
            f"\teng_area={rec.eng_area:.4f}"
            f"\teng_chord={rec.eng_chord:.4f}"
            f"\tcut_area={rec.cut_area:.4f}"
            f"\ttotal_area={rec.total_area:.4f}"
            f"\trem_area={rec.remaining_area:.4f}"
            f"\titers={rec.iters}"
            f"\tops_len={rec.ops_len}"
            f"\tstrat="
            + "|".join(
                "WSMFEI"[i]
                + (":" + [".", "X", "B"][v] if v <= 2 else ":?")
                + (
                    f"[{rec.resume_strategy_details[i]}]"
                    if rec.resume_strategy_details[i]
                    else ""
                )
                for i, v in enumerate(rec.resume_strategy_reasons)
            )
            + "\trout="
                + "|".join(
                "DFMZ"[i]
                + ":"
                + get_route_detail_name(rec.route_strategy_details[i])
                for i in range(4)
            )
        )
