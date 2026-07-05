import sys

from raygeo.trace import TraceFile


def register(subparsers):
    p = subparsers.add_parser(
        "inspect", help="Open the interactive viewer for a trace file."
    )
    p.add_argument("tracefile", help="Input path for the .bin trace file.")
    p.add_argument(
        "step",
        nargs="?",
        type=int,
        default=0,
        help="Initial step to display (default: 0).",
    )
    p.set_defaults(func=run)


def run(args):
    """Open the interactive viewer for a trace file."""
    try:
        import matplotlib.pyplot as plt
    except ImportError:
        print(
            "The 'inspect' command requires matplotlib.\n"
            "Install it with: pip install raygeo[cli]",
            file=sys.stderr,
            flush=True,
        )
        return

    from raygeo.cli.inspector import Inspector

    trace_path = args.tracefile
    initial_step = args.step or 0

    print(f"Loading trace from {trace_path}")
    trace = TraceFile(trace_path)
    print(f"  {len(trace)} trace records")

    geo = trace.geometry
    tp = trace.toolpath
    print(
        f"  geometry: tool_radius={geo['tool_radius']}  "
        f"boundary={len(geo['boundary'])} verts  "
        f"islands={len(geo['islands'])}  "
        f"seeds={len(geo['seeds'])}"
    )
    print(f"  toolpath: {len(tp)} moves")

    seed_polys = [[tuple(p) for p in poly] for poly in geo["seeds"]]

    inspector = Inspector(trace, tp, seed_polys, geo)
    if initial_step > 0:
        inspector._draw(initial_step)
    plt.show()
