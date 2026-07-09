import matplotlib.pyplot as plt

from raygeo.cli.inspector import Inspector
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
    trace_path = args.tracefile
    initial_step = args.step or 0

    print(f"Loading trace from {trace_path}")
    trace = TraceFile(trace_path)
    print(
        f"  {len(trace.events)} events, {len(trace.spans)} spans, "
        f"sources={sorted(trace.sources)}"
    )

    inspector = Inspector(trace)
    if initial_step > 0:
        inspector._draw(initial_step)
    plt.show()
