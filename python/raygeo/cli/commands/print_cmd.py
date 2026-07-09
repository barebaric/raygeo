from raygeo.trace import TraceFile


def register(subparsers):
    p = subparsers.add_parser(
        "print", help="Dump all trace records as an event log."
    )
    p.add_argument("tracefile", help="Input path for the .bin trace file.")
    p.set_defaults(func=run)


def _fmt_attrs(attrs):
    if not attrs:
        return ""
    parts = []
    for k, v in attrs.items():
        if isinstance(v, (list, tuple)):
            if len(v) > 3:
                parts.append(f"{k}=[{len(v)} items]")
            else:
                parts.append(f"{k}={v!r}")
        elif isinstance(v, dict):
            if len(v) > 3:
                parts.append(f"{k}={{{len(v)} keys}}")
            else:
                parts.append(f"{k}={v!r}")
        else:
            parts.append(f"{k}={v!r}")
    return "; ".join(parts)


def _fmt_meta(meta):
    if not meta:
        return ""
    parts = []
    for k, v in meta.items():
        if isinstance(v, (list, tuple)):
            parts.append(f"{k}=[{len(v)} items]")
        elif isinstance(v, dict):
            parts.append(f"{k}={{{len(v)} keys}}")
        else:
            parts.append(f"{k}={v!r}")
    return "; ".join(parts)


def _print_span(span, depth=0):
    indent = "  " * depth
    attrs_str = _fmt_attrs(span.attrs)
    attrs_part = f"  attrs: {attrs_str}" if attrs_str else ""
    print(
        f'{indent}[#{span.id}] {span.source} "{span.label}"'
        f"{attrs_part}  ({len(span.events)} events)"
    )
    for child in span.children:
        _print_span(child, depth + 1)


def run(args):
    trace_path = args.tracefile
    trace = TraceFile(trace_path)

    sources = sorted(trace.sources)
    print(
        f"trace: {trace_path}  "
        f"ver={trace.ver}  "
        f"spans={len(trace.spans)}  "
        f"events={len(trace.events)}  "
        f"sources={','.join(sources)}"
    )

    print("SPAN TREE:")
    if trace.root is not None:
        _print_span(trace.root)
    else:
        print("  (no root span)")

    print()
    print("EVENTS:")
    for ev in trace.events:
        seq = f"seq={ev.seq:>5}"
        sp = f"span={ev.span}"
        kind = f"{ev.kind.upper():>6}"
        src = f"source={ev.source}"
        mk = ev.move_kind or "-"
        move = f"move={mk}"
        if ev.tool is not None:
            pos = (
                f"pos=({ev.tool.pos_x:.2f},{ev.tool.pos_y:.2f},"
                f"{ev.tool.pos_z:.2f})"
            )
        else:
            pos = "pos=-"
        if ev.progress is not None:
            step = f"step={ev.progress.step_idx}"
        else:
            step = "step=-"
        meta_str = _fmt_meta(ev.meta)
        meta_part = f"  meta: {meta_str}" if meta_str else ""
        print(
            f"  {seq}  {sp}  {kind}  {src}  {move}  {pos}  {step}{meta_part}"
        )
