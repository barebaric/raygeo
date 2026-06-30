#!/usr/bin/env python
"""Interactive adaptive clearing inspector.

Two subcommands:

    trace   — run adaptive clearing and write a trace file.
    inspect — open the interactive viewer for a trace file.

Usage::

    python tools/adaptive_inspector.py trace /tmp/trace.bin
    python tools/adaptive_inspector.py trace /tmp/tr.bin
    python tools/adaptive_inspector.py trace /tmp/tr.bin \\
        --scenario centre-island
    python tools/adaptive_inspector.py trace /tmp/tr.bin \\
        --svg logo.svg --tool-radius 2.5
    python tools/adaptive_inspector.py inspect /tmp/trace.bin
    python tools/adaptive_inspector.py inspect /tmp/trace.bin 500

Controls:
     TextBox + Go button  — jump to any step number
     ◀ / ▶ buttons        — previous / next step
     ◀◀ Seg / Seg ▶▶  — previous / next segment
     Left / Right arrows   — previous / next step
     Shift+Left / Right    — previous / next segment
     Home / End            — first / last step
"""

import argparse
import dataclasses
import math
import pathlib
import struct

import matplotlib.pyplot as plt

# ── Workaround for matplotlib 3.11 bug ────────────────────────────
# ResizeEvent lacks the 'inaxes' attribute, causing AttributeError
# in the widget event decorator (_call_with_reparented_event).
# Patch every decorated event handler to add 'inaxes' if missing.
import matplotlib.widgets as _mw
from matplotlib.collections import LineCollection
from matplotlib.patches import Circle
from matplotlib.widgets import Button, TextBox


def _patch_widget_events() -> None:
    for _cls in (_mw.Button, _mw.TextBox):
        for _name in (
            "_click",
            "_motion",
            "_release",
            "_keypress",
            "_resize",
        ):
            _meth = getattr(_cls, _name, None)
            if _meth is not None and hasattr(_meth, "__wrapped__"):
                _orig = _meth

                def _safe(self, event, _orig=_orig):
                    if not hasattr(event, "inaxes"):
                        event.inaxes = None
                    if not hasattr(event, "x"):
                        event.x = 0
                    if not hasattr(event, "y"):
                        event.y = 0
                    return _orig(self, event)

                setattr(_cls, _name, _safe)


_patch_widget_events()

from raygeo.geo.shape.polygon import (  # noqa: E402
    get_polygon_area,
    get_polygon_signed_area,
    is_point_inside_polygon,
)
from raygeo.ops.assembly.adaptive import adaptive_clearing  # noqa: E402
from raygeo.ops.assembly.entry import adaptive_entry  # noqa: E402
from raygeo.ops.cut.cleared_area import ClearedArea  # noqa: E402

# ── Trace file format ────────────────────────────────────────────

TRACE_HEADER_SIZE = 12  # magic(4) + version(4) + count(4)
TRACE_RECORD_SIZE = 128
TRACE_MAGIC = b"ADPT"

KIND_NAMES = {
    0: "init",
    1: "cut",
    2: "resume_stall",
    3: "resume_stuck",
    4: "exit",
}
STATUS_NAMES = {
    0: "Ok",
    1: "BoundaryHit",
    2: "LostEngagement",
    3: "NoConvergence",
}

RESUME_SOURCE_NAMES = {
    0: "none",
    1: "segment_resume",
    2: "mat_resume",
    3: "boundary_walk",
    4: "wall_hug",
}


class TraceRecord:
    """One per-step record from the trace file (128 bytes)."""

    __slots__ = (
        "kind",
        "status",
        "step_idx",
        "iters",
        "pos_x",
        "pos_y",
        "heading",
        "smoothed_heading",
        "predicted_angle",
        "iteration_angle",
        "eng_angle",
        "eng_area",
        "eng_chord",
        "cut_area",
        "total_area",
        "remaining_area",
        "prev_x",
        "prev_y",
        "ops_len",
        "resume_source",
    )

    def __init__(self, buf):
        self.kind = buf[0]
        self.status = buf[1]
        self.step_idx = struct.unpack_from("<I", buf, 2)[0]
        self.iters = struct.unpack_from("<I", buf, 6)[0]
        self.pos_x = struct.unpack_from("<d", buf, 10)[0]
        self.pos_y = struct.unpack_from("<d", buf, 18)[0]
        self.heading = struct.unpack_from("<d", buf, 26)[0]
        self.smoothed_heading = struct.unpack_from("<d", buf, 34)[0]
        self.predicted_angle = struct.unpack_from("<d", buf, 42)[0]
        self.iteration_angle = struct.unpack_from("<d", buf, 50)[0]
        self.eng_angle = struct.unpack_from("<d", buf, 58)[0]
        self.eng_area = struct.unpack_from("<d", buf, 66)[0]
        self.eng_chord = struct.unpack_from("<d", buf, 74)[0]
        self.cut_area = struct.unpack_from("<d", buf, 82)[0]
        self.total_area = struct.unpack_from("<d", buf, 90)[0]
        self.remaining_area = struct.unpack_from("<d", buf, 98)[0]
        self.prev_x = struct.unpack_from("<d", buf, 106)[0]
        self.prev_y = struct.unpack_from("<d", buf, 114)[0]
        self.ops_len = struct.unpack_from("<I", buf, 122)[0]
        self.resume_source = buf[126]


class TraceGeometry:
    """Pocket geometry embedded in a trace file."""

    __slots__ = ("tool_radius", "boundary", "islands", "seeds")

    def __init__(self, tool_radius, boundary, islands, seeds):
        self.tool_radius = tool_radius
        self.boundary = boundary
        self.islands = islands
        self.seeds = seeds


class TraceFile:
    """Binary trace reader with random access to records.

    Geometry, seeds, toolpath, and per-step records are all embedded in
    a single self-contained file.
    """

    def __init__(self, path):
        with open(path, "rb") as f:
            magic = f.read(4)
            if magic != TRACE_MAGIC:
                raise ValueError(f"bad magic: {magic}")
            self.version = struct.unpack("<I", f.read(4))[0]
            if self.version < 3:
                raise ValueError(
                    f"trace version {self.version} is no longer supported; "
                    f"re-generate with the current inspector"
                )
            self.count = struct.unpack("<I", f.read(4))[0]
            self.geometry = self._read_geometry(f)
            self.toolpath = self._read_toolpath(f)
            self.data = f.read()

    def _read_geometry(self, f):
        tool_radius = struct.unpack("<d", f.read(8))[0]
        boundary = self._read_polygon(f)
        n_islands = struct.unpack("<I", f.read(4))[0]
        islands = [self._read_polygon(f) for _ in range(n_islands)]
        n_seeds = struct.unpack("<I", f.read(4))[0]
        seeds = [self._read_polygon(f) for _ in range(n_seeds)]
        return TraceGeometry(tool_radius, boundary, islands, seeds)

    @staticmethod
    def _read_polygon(f):
        n = struct.unpack("<I", f.read(4))[0]
        pts = []
        for _ in range(n):
            x = struct.unpack("<d", f.read(8))[0]
            y = struct.unpack("<d", f.read(8))[0]
            pts.append((x, y))
        return pts

    @staticmethod
    def _read_toolpath(f):
        n = struct.unpack("<I", f.read(4))[0]
        pts = []
        for _ in range(n):
            x = struct.unpack("<d", f.read(8))[0]
            y = struct.unpack("<d", f.read(8))[0]
            is_travel = f.read(1) != b"\x00"
            pts.append((x, y, bool(is_travel)))
        return pts

    def __len__(self):
        return self.count

    def __getitem__(self, idx):
        if idx < 0 or idx >= self.count:
            raise IndexError(idx)
        offset = idx * TRACE_RECORD_SIZE
        return TraceRecord(self.data[offset : offset + TRACE_RECORD_SIZE])


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
    batch = 0
    cut_count = 0
    for i in range(len(tp)):
        x, y, is_travel = tp[i]
        if is_travel:
            prev = (x, y)
            continue
        if prev is not None and cut_count >= start_cut:
            if batch == 0:
                ca.begin_batch()
            ca.expand_batched(prev, (x, y), geometry.tool_radius)
            batch += 1
            if batch >= 20:
                ca.commit_batch_local()
                batch = 0
                ca.compact_if_needed(0.5)
        prev = (x, y)
        cut_count += 1
        if cut_count >= n_cuts:
            break
    if batch > 0:
        ca.commit_batch_local()
    return ca


# ── Scenario infrastructure ──────────────────────────────────────


@dataclasses.dataclass
class Scenario:
    """Pocket geometry, cutting parameters, and seed polygons."""

    name: str
    boundary: list
    islands: list
    tool_radius: float
    advance: float
    step_over: float
    cut_z: float
    safe_z: float
    area_tolerance: float


SCENARIOS: dict[str, Scenario] = {}


def register_scenario(scenario: Scenario) -> None:
    SCENARIOS[scenario.name] = scenario


# ── Helper geometry functions ────────────────────────────────────


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _circle_polygon(cx, cy, r, n=64):
    pts = []
    for i in range(n):
        a = 2.0 * math.pi * i / n
        pts.append((cx + r * math.cos(a), cy + r * math.sin(a)))
    return pts


# ── Built-in scenarios ───────────────────────────────────────────

register_scenario(
    Scenario(
        name="default",
        boundary=[(0, 0), (180, 0), (180, 120), (0, 120)],
        islands=[
            [(15, 15), (35, 15), (35, 35), (15, 35)],
            [(70, 40), (90, 40), (90, 60), (70, 60)],
            [(130, 80), (160, 80), (160, 105), (130, 105)],
        ],
        tool_radius=3.0,
        advance=2.0,
        step_over=2.0,
        cut_z=-5.0,
        safe_z=2.0,
        area_tolerance=1.0,
    )
)

register_scenario(
    Scenario(
        name="centre-island",
        boundary=_rect(0, 0, 60, 60),
        islands=[_rect(5, 0, 10, 10)],
        tool_radius=3.0,
        advance=1.5,
        step_over=2.0,
        cut_z=-5.0,
        safe_z=2.0,
        area_tolerance=1.0,
    )
)


# ── SVG scenario loader ──────────────────────────────────────────


def scenario_from_svg(
    svg_source,
    tool_radius=3.0,
    advance=1.5,
    step_over=2.0,
    cut_z=-5.0,
    safe_z=2.0,
    area_tolerance=1.0,
):
    """Build a Scenario from an SVG document.

    For SVG with multiple contours, the **first** outer contour is used
    as the pocket boundary; its holes become islands.
    """
    import raygeo.svg as _svg

    p = pathlib.Path(svg_source)
    if p.exists():
        svg_str = p.read_text()
    else:
        svg_str = svg_source

    geoms = _svg.svg_string_to_geometries(svg_str)

    all_polys = []
    for g in geoms:
        polys = g.to_polygons(tolerance=0.1)
        all_polys.extend(polys)

    if not all_polys:
        raise ValueError("No polygons found in SVG")

    # Flip Y: SVG Y-down * math Y-up
    all_y = [y for p in all_polys for _, y in p]
    y_min, y_max = min(all_y), max(all_y)
    flipped = []
    for p in all_polys:
        flipped.append([(x, y_min + y_max - y) for x, y in p])

    # Separate outer (CW after flip = negative signed area) and inner
    outer = [p for p in flipped if get_polygon_signed_area(p) < -0.01]
    inner = [p for p in flipped if get_polygon_signed_area(p) >= 0.01]

    if not outer:
        raise ValueError("No outer contour found in SVG")

    boundary = outer[0]

    # Find holes inside the first boundary
    islands = [h for h in inner if is_point_inside_polygon(h[0], boundary)]

    return Scenario(
        name="svg",
        boundary=boundary,
        islands=islands,
        tool_radius=tool_radius,
        advance=advance,
        step_over=step_over,
        cut_z=cut_z,
        safe_z=safe_z,
        area_tolerance=area_tolerance,
    )


# ── Seed / entry helpers ─────────────────────────────────────────


def run_entry(scenario):
    """Run adaptive_entry and return (entry_ops, seed_polys)."""
    entry_ops, cp = adaptive_entry(
        pocket_boundary=list(scenario.boundary),
        islands=[list(isl) for isl in scenario.islands],
        tool_radius=scenario.tool_radius,
        step_over=scenario.step_over,
        safe_z=scenario.safe_z,
        target_z=scenario.cut_z,
        plunge_pitch=1.0,
    )
    return entry_ops, cp


def build_scenario(args):
    """Build (scenario, seed_polys, entry_ops) from parsed CLI args."""
    if args.svg:
        scenario = scenario_from_svg(
            args.svg,
            tool_radius=args.tool_radius,
            advance=args.advance,
            step_over=args.step_over,
            cut_z=args.cut_z,
            safe_z=args.safe_z,
            area_tolerance=args.area_tolerance,
        )
        entry_ops, seed_polys = run_entry(scenario)
        print(
            f"  SVG scenario: boundary={len(scenario.boundary)} verts, "
            f"{len(scenario.islands)} islands"
        )

    elif args.scenario == "centre-island":
        scenario = SCENARIOS["centre-island"]
        # Apply any overrides
        if args.tool_radius is not None:
            scenario = dataclasses.replace(
                scenario, tool_radius=args.tool_radius
            )
        if args.advance is not None:
            scenario = dataclasses.replace(scenario, advance=args.advance)
        if args.step_over is not None:
            scenario = dataclasses.replace(scenario, step_over=args.step_over)
        if args.cut_z is not None:
            scenario = dataclasses.replace(scenario, cut_z=args.cut_z)
        if args.safe_z is not None:
            scenario = dataclasses.replace(scenario, safe_z=args.safe_z)
        if args.area_tolerance is not None:
            scenario = dataclasses.replace(
                scenario, area_tolerance=args.area_tolerance
            )

        seed_polys = [_circle_polygon(-13.7, 13.7, 12.2, 64)]
        entry_ops = None
        print(
            "  Centre-island scenario: circle seed "
            "centre=(-13.7,13.7) radius=12.2"
        )

    else:
        # Default scenario (or any other registered name)
        scenario = SCENARIOS.get(args.scenario)
        if scenario is None:
            raise ValueError(
                f"Unknown scenario: {args.scenario}. "
                f"Available: {', '.join(SCENARIOS)}"
            )
        # Apply overrides
        if args.tool_radius is not None:
            scenario = dataclasses.replace(
                scenario, tool_radius=args.tool_radius
            )
        if args.advance is not None:
            scenario = dataclasses.replace(scenario, advance=args.advance)
        if args.step_over is not None:
            scenario = dataclasses.replace(scenario, step_over=args.step_over)
        if args.cut_z is not None:
            scenario = dataclasses.replace(scenario, cut_z=args.cut_z)
        if args.safe_z is not None:
            scenario = dataclasses.replace(scenario, safe_z=args.safe_z)
        if args.area_tolerance is not None:
            scenario = dataclasses.replace(
                scenario, area_tolerance=args.area_tolerance
            )

        entry_ops, seed_polys = run_entry(scenario)

    return scenario, seed_polys, entry_ops


# ── Inspector ─────────────────────────────────────────────────────


class Inspector:
    def __init__(self, trace, tp, seed_polys, geometry):
        self.trace = trace
        self.tp = tp
        self.n_steps = len(trace)
        self.seed_polys = seed_polys
        self.geometry = geometry
        self.current = 0
        self._ca_cache = {}
        self._precompute_toolpath()
        self._build_segment_steps()

        self.fig, self.ax = plt.subplots(1, 1, figsize=(14, 9))
        self.fig.subplots_adjust(bottom=0.18, top=0.92)
        self.ax.set_aspect("equal")

        ax_text = self.fig.add_axes((0.12, 0.06, 0.18, 0.04))
        ax_btn = self.fig.add_axes((0.32, 0.06, 0.08, 0.04))
        ax_prev = self.fig.add_axes((0.42, 0.06, 0.05, 0.04))
        ax_next = self.fig.add_axes((0.48, 0.06, 0.05, 0.04))
        ax_prev_seg = self.fig.add_axes((0.55, 0.06, 0.07, 0.04))
        ax_next_seg = self.fig.add_axes((0.63, 0.06, 0.07, 0.04))

        self.textbox = TextBox(ax_text, "Step:", initial="0")
        self.textbox.on_submit(self._on_submit)
        self.btn_go = Button(ax_btn, "Go")
        self.btn_go.on_clicked(self._on_go)
        self.btn_prev = Button(ax_prev, "◀")
        self.btn_prev.on_clicked(lambda e: self._step(-1))
        self.btn_next = Button(ax_next, "▶")
        self.btn_next.on_clicked(lambda e: self._step(1))
        self.btn_prev_seg = Button(ax_prev_seg, "◀◀ Seg")
        self.btn_prev_seg.on_clicked(lambda e: self._step_segment(-1))
        self.btn_next_seg = Button(ax_next_seg, "Seg ▶▶")
        self.btn_next_seg.on_clicked(lambda e: self._step_segment(1))

        self.ax_info = self.fig.add_axes(
            (0.01, 0.11, 0.98, 0.05), frameon=False
        )
        self.ax_info.set_xlim(0, 1)
        self.ax_info.set_ylim(0, 1)
        self.ax_info.axis("off")
        self.info_text = self.ax_info.text(
            0.01, 0.5, "", fontsize=8, family="monospace", va="center"
        )

        self.fig.canvas.mpl_connect("key_press_event", self._on_key)

        self._draw(0)

    def _on_submit(self, text):
        try:
            self._draw(int(text.strip()))
        except ValueError:
            pass

    def _on_go(self, _event):
        try:
            self._draw(int(self.textbox.text.strip()))
        except ValueError:
            pass

    def _step_segment(self, delta):
        cur_seg = self._current_segment_idx()
        seg_idx = cur_seg + delta
        seg_idx = max(0, min(seg_idx, len(self._seg_start_steps) - 1))
        if seg_idx != cur_seg:
            self._draw(self._seg_start_steps[seg_idx])

    def _current_segment_idx(self):
        for i in range(len(self._seg_start_steps) - 1, -1, -1):
            if self._seg_start_steps[i] <= self.current:
                return i
        return 0

    def _build_segment_steps(self):
        n_seg = len(self._segment_starts)
        seg_steps = [0] * n_seg
        seg_steps[0] = 0
        si = 1
        for step_idx in range(1, self.n_steps):
            n_moves = min(self.trace[step_idx].ops_len, len(self.tp))
            while si < n_seg and n_moves > self._segment_starts[si][2]:
                seg_steps[si] = step_idx
                si += 1
            if si >= n_seg:
                break
        for j in range(si, n_seg):
            seg_steps[j] = self.n_steps - 1
        self._seg_start_steps = seg_steps

    def _step(self, delta):
        self._draw(self.current + delta)

    def _on_key(self, event):
        if event.key == "left":
            self._step(-1)
        elif event.key == "right":
            self._step(1)
        elif event.key == "shift+left":
            self._step_segment(-1)
        elif event.key == "shift+right":
            self._step_segment(1)
        elif event.key == "home":
            self._draw(0)
        elif event.key == "end":
            self._draw(self.n_steps - 1)

    def _precompute_toolpath(self):
        """Precompute cutting edges, travel edges, and cumulative distances
        for the full toolpath once, so _draw_toolpath is O(edges) per step
        and colors are stable."""
        n_total = len(self.tp)
        self._all_cut_segs = []  # [[(x1,y1),(x2,y2)], ...]
        self._all_cut_cum = []  # cumulative distance at each cut edge
        self._all_travel_segs = []  # [[(x1,y1),(x2,y2)], ...]
        self._segment_starts = []  # [(x, y, tp_idx), ...]
        self._move_to_edge_count = []
        self._move_to_travel_count = []

        cut_prev = None
        prev_any = None
        cum = 0.0
        edge_count = 0
        travel_count = 0
        prev_was_travel = None

        for i in range(n_total):
            x, y, is_travel = self.tp[i]

            if is_travel:
                if prev_any is not None:
                    self._all_travel_segs.append([prev_any, (x, y)])
                    travel_count += 1
                prev_any = (x, y)
                cut_prev = None
            else:
                if cut_prev is None and prev_any is not None:
                    # First cut after travel: draw edge from the
                    # travel destination to this first cut endpoint.
                    self._all_cut_segs.append([prev_any, (x, y)])
                    cum += math.hypot(x - prev_any[0], y - prev_any[1])
                    self._all_cut_cum.append(cum)
                    edge_count += 1
                elif cut_prev is not None:
                    self._all_cut_segs.append([cut_prev, (x, y)])
                    cum += math.hypot(x - cut_prev[0], y - cut_prev[1])
                    self._all_cut_cum.append(cum)
                    edge_count += 1
                cut_prev = (x, y)
                prev_any = (x, y)

            # Mark a segment start only at the travel destination
            # (cut→travel transition) and at the initial point.
            # A travel→cut transition would mark the first cut
            # endpoint, which is one step into the segment, not its
            # start.
            if prev_was_travel is None or (
                prev_was_travel is False and is_travel
            ):
                self._segment_starts.append((x, y, i))
            prev_was_travel = is_travel

            self._move_to_edge_count.append(edge_count)
            self._move_to_travel_count.append(travel_count)

        self._all_cut_total = cum if cum > 0 else 1.0

    def _get_cleared(self, n_cuts):
        """Rebuild ClearedArea up to n_cuts cutting moves.
        Extends from the nearest cached smaller cut count when possible."""
        if n_cuts in self._ca_cache:
            return self._ca_cache[n_cuts]

        smaller = max(
            (k for k in self._ca_cache if k <= n_cuts),
            default=None,
        )
        if smaller is not None:
            ca = rebuild_cleared(
                n_cuts,
                self.tp,
                self.seed_polys,
                self.geometry,
                existing_fragments=self._ca_cache[smaller].fragments(),
                start_cut=smaller,
            )
        else:
            ca = rebuild_cleared(
                n_cuts, self.tp, self.seed_polys, self.geometry
            )

        self._ca_cache[n_cuts] = ca
        if len(self._ca_cache) > 20:
            for k in sorted(self._ca_cache)[:-20]:
                del self._ca_cache[k]
        return ca

    def _draw(self, step_idx):
        step_idx = max(0, min(step_idx, self.n_steps - 1))
        self.current = step_idx

        self.ax.clear()
        self.ax.set_aspect("equal")

        rec = self.trace[step_idx]

        geo = self.geometry
        boundary = geo.boundary
        islands = geo.islands
        tool_radius = geo.tool_radius

        # ── Boundary ──
        bx = [p[0] for p in boundary] + [boundary[0][0]]
        by = [p[1] for p in boundary] + [boundary[0][1]]
        self.ax.plot(bx, by, "k-", linewidth=1.5)

        # ── Islands ──
        for isl in islands:
            ix = [p[0] for p in isl] + [isl[0][0]]
            iy = [p[1] for p in isl] + [isl[0][1]]
            self.ax.fill(ix, iy, color="gray", alpha=0.4)
            self.ax.plot(ix, iy, color="dimgray", linewidth=1.0)

        # ── Seed outline ──
        for poly in self.seed_polys:
            if len(poly) < 3:
                continue
            sx = [p[0] for p in poly] + [poly[0][0]]
            sy = [p[1] for p in poly] + [poly[0][1]]
            self.ax.plot(
                sx,
                sy,
                color="steelblue",
                linewidth=1.5,
                linestyle="--",
                alpha=0.6,
            )

        # ── Envelope (static) ──
        ca0 = self._get_cleared(0)
        envelope = ca0.envelope(tool_radius)
        for env in envelope:
            if len(env) < 3:
                continue
            ex = [p[0] for p in env] + [env[0][0]]
            ey = [p[1] for p in env] + [env[0][1]]
            self.ax.plot(ex, ey, "b--", linewidth=0.7, alpha=0.5)

        # ── Cleared area at this step ──
        # ops_len in the trace record is the moving-command count
        # (toolpath index), not the raw ops command count.
        n_tp_moves = min(rec.ops_len, len(self.tp))
        n_cuts = sum(1 for i in range(n_tp_moves) if not self.tp[i][2])
        ca = self._get_cleared(n_cuts)

        # Remaining (only exterior rings)
        remaining = ca.remaining()
        for poly in remaining:
            if len(poly) < 3:
                continue
            a = get_polygon_area(poly)
            if a <= 0 or a < 0.1:
                continue
            rx = [p[0] for p in poly] + [poly[0][0]]
            ry = [p[1] for p in poly] + [poly[0][1]]
            self.ax.fill(rx, ry, color="crimson", alpha=0.08)
            self.ax.plot(rx, ry, color="crimson", linewidth=0.3, alpha=0.3)

        # Cleared (white fill)
        for poly in ca.fragments():
            if len(poly) < 3:
                continue
            cx = [p[0] for p in poly] + [poly[0][0]]
            cy = [p[1] for p in poly] + [poly[0][1]]
            self.ax.fill(cx, cy, color="white")

        # ── Toolpath ──
        self._draw_toolpath(rec)

        # ── Tool position ──
        self._draw_tool(rec)

        # ── Title ──
        kind_name = KIND_NAMES.get(rec.kind, str(rec.kind))
        status_name = STATUS_NAMES.get(rec.status, str(rec.status))
        resume_src = ""
        if rec.kind in (2, 3) and rec.resume_source:
            rs = RESUME_SOURCE_NAMES.get(
                rec.resume_source, str(rec.resume_source)
            )
            resume_src = f"  via={rs}"
        self.ax.set_title(
            f"Step {step_idx}/{self.n_steps - 1}  "
            f"kind={kind_name}  status={status_name}{resume_src}  "
            f"cleared={rec.total_area:.0f}  "
            f"remaining={rec.remaining_area:.0f}",
            fontsize=10,
        )
        self.ax.set_xlabel("X")
        self.ax.set_ylabel("Y")
        # Auto-fit axis limits to the boundary with a small margin.
        bx_min, bx_max = min(bx), max(bx)
        by_min, by_max = min(by), max(by)
        mx = (bx_max - bx_min) * 0.05 + tool_radius
        my = (by_max - by_min) * 0.05 + tool_radius
        self.ax.set_xlim(bx_min - mx, bx_max + mx)
        self.ax.set_ylim(by_min - my, by_max + my)
        self.ax.grid(True, alpha=0.2)

        # ── Info panel ──
        info = self._format_info(rec, kind_name, status_name)
        self.info_text.set_text(info)

        self.fig.canvas.draw_idle()

    def _draw_toolpath(self, rec):
        """Draw toolpath up to the moving-command count in this record."""
        n_moves = min(rec.ops_len, len(self.tp))
        if n_moves == 0:
            return

        n_edges = self._move_to_edge_count[n_moves - 1]
        n_travel = self._move_to_travel_count[n_moves - 1]

        # Cutting segments (turbo LineCollection, stable coloring)
        if n_edges:
            self.ax.add_collection(
                LineCollection(
                    self._all_cut_segs[:n_edges],
                    colors=plt.cm.turbo(
                        [
                            d / self._all_cut_total
                            for d in self._all_cut_cum[:n_edges]
                        ]
                    ),
                    linewidth=0.7,
                    alpha=0.9,
                    zorder=2,
                )
            )

        # Travel segments (dashed gray, drawn above cutting lines)
        if n_travel:
            for seg in self._all_travel_segs[:n_travel]:
                self.ax.plot(
                    [seg[0][0], seg[1][0]],
                    [seg[0][1], seg[1][1]],
                    linestyle="--",
                    linewidth=0.8,
                    color="dimgray",
                    alpha=0.7,
                    zorder=3,
                )

        # Blue dot at the start of each new segment (up to current move)
        for sx, sy, idx in self._segment_starts:
            if idx < n_moves:
                self.ax.plot(
                    sx,
                    sy,
                    "o",
                    color="blue",
                    markersize=2.5,
                    zorder=4,
                    alpha=0.8,
                )

    def _draw_tool(self, rec):
        """Draw tool circle, position dot, and heading arrow."""
        x, y = rec.pos_x, rec.pos_y
        r = self.geometry.tool_radius

        circle = Circle(
            (x, y), r, fill=False, edgecolor="red", linewidth=1.5, alpha=0.8
        )
        self.ax.add_patch(circle)
        self.ax.plot(x, y, "ro", markersize=4)

        # Heading arrow (red)
        h = rec.heading
        dx = math.cos(h) * r * 2
        dy = math.sin(h) * r * 2
        self.ax.annotate(
            "",
            xy=(x + dx, y + dy),
            xytext=(x, y),
            arrowprops=dict(arrowstyle="->", color="red", lw=1.5),
        )

        # Smoothed heading (orange, shorter)
        sh = rec.smoothed_heading
        dx2 = math.cos(sh) * r * 1.5
        dy2 = math.sin(sh) * r * 1.5
        self.ax.annotate(
            "",
            xy=(x + dx2, y + dy2),
            xytext=(x, y),
            arrowprops=dict(
                arrowstyle="->", color="orange", lw=1.0, alpha=0.6
            ),
        )

    def _format_info(self, rec, kind_name, status_name):
        h_deg = math.degrees(rec.heading)
        sh_deg = math.degrees(rec.smoothed_heading)
        pa_deg = math.degrees(rec.predicted_angle)
        ia_deg = math.degrees(rec.iteration_angle)
        eng_deg = math.degrees(rec.eng_angle)
        step_dist = math.hypot(rec.pos_x - rec.prev_x, rec.pos_y - rec.prev_y)
        resume_src = ""
        if rec.kind in (2, 3) and rec.resume_source:
            rs = RESUME_SOURCE_NAMES.get(
                rec.resume_source, str(rec.resume_source)
            )
            resume_src = f"  resume_via={rs}"
        return (
            f"step={rec.step_idx}  kind={kind_name}  status={status_name}"
            f"{resume_src}  "
            f"pos=({rec.pos_x:.1f},{rec.pos_y:.1f})  "
            f"hdg={h_deg:.1f}°  smooth={sh_deg:.1f}°  "
            f"pred={pa_deg:.1f}°  iter={ia_deg:.1f}°  "
            f"iters={rec.iters}\n"
            f"step_dist={step_dist:.2f}  cut_area={rec.cut_area:.3f}  "
            f"eng: angle={eng_deg:.1f}°  area={rec.eng_area:.3f}  "
            f"chord={rec.eng_chord:.3f}  "
            f"cleared={rec.total_area:.1f}  "
            f"remaining={rec.remaining_area:.1f}"
        )


# ── Subcommands ──────────────────────────────────────────────────


def cmd_trace(args: argparse.Namespace) -> None:
    """Run adaptive entry + clearing with tracing, write trace file."""
    trace_path = args.tracefile

    scenario, seed_polys, entry_ops = build_scenario(args)

    print(f"Running {scenario.name} scenario with tracing...")
    print(
        f"  tool_radius={scenario.tool_radius}  advance={scenario.advance}  "
        f"step_over={scenario.step_over}"
    )
    print(
        f"  boundary: {len(scenario.boundary)} verts  "
        f"islands: {len(scenario.islands)}"
    )
    print(
        f"  seeds: {len(seed_polys)} polygons "
        f"({sum(len(p) for p in seed_polys)} verts)"
    )

    ca = ClearedArea(
        boundary=list(scenario.boundary),
        islands=[list(isl) for isl in scenario.islands],
        initial=seed_polys,
    )

    if entry_ops is not None:
        print(f"  Entry: {entry_ops.len()} ops")

    clear_ops = adaptive_clearing(
        cleared=ca,
        pocket_boundary=list(scenario.boundary),
        islands=[list(isl) for isl in scenario.islands],
        radius=scenario.tool_radius,
        advance=scenario.advance,
        cut_z=scenario.cut_z,
        safe_z=scenario.safe_z,
        area_tolerance=scenario.area_tolerance,
        trace_path=trace_path,
    )

    print(
        f"  Clearing: {clear_ops.len()} ops, "
        f"{ca.total_area():.1f} mm² cleared, "
        f"{ca.remaining_area():.1f} mm² remaining"
    )
    print(f"  Trace written: {trace_path}")


def cmd_inspect(args: argparse.Namespace) -> None:
    """Open the interactive viewer for a trace file."""
    trace_path = args.tracefile
    initial_step = args.step or 0

    print(f"Loading trace from {trace_path}")
    trace = TraceFile(trace_path)
    print(f"  version={trace.version}  {len(trace)} trace records")

    geo = trace.geometry
    tp = trace.toolpath
    print(
        f"  geometry: tool_radius={geo.tool_radius}  "
        f"boundary={len(geo.boundary)} verts  "
        f"islands={len(geo.islands)}  "
        f"seeds={len(geo.seeds)}"
    )
    print(f"  toolpath: {len(tp)} moves")

    # Seeds are embedded in the trace -- no re-derivation needed.
    seed_polys = geo.seeds

    inspector = Inspector(trace, tp, seed_polys, geo)
    if initial_step > 0:
        inspector._draw(initial_step)
    plt.show()


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Interactive adaptive clearing inspector."
    )
    sub = parser.add_subparsers(dest="command", required=True)

    p_trace = sub.add_parser(
        "trace", help="Run adaptive clearing and write a trace file."
    )
    p_trace.add_argument(
        "tracefile", help="Output path for the .bin trace file."
    )
    p_trace.add_argument(
        "--scenario",
        default="default",
        choices=list(SCENARIOS),
        help="Pocket scenario to use (default: default).",
    )
    p_trace.add_argument(
        "--svg",
        type=str,
        default=None,
        help="Path to SVG file. Overrides --scenario. "
        "First outer contour becomes pocket boundary; holes become islands.",
    )
    p_trace.add_argument("--tool-radius", type=float, default=None)
    p_trace.add_argument("--advance", type=float, default=None)
    p_trace.add_argument("--step-over", type=float, default=None)
    p_trace.add_argument("--cut-z", type=float, default=None)
    p_trace.add_argument("--safe-z", type=float, default=None)
    p_trace.add_argument("--area-tolerance", type=float, default=None)
    p_trace.set_defaults(func=cmd_trace)

    p_inspect = sub.add_parser(
        "inspect", help="Open the interactive viewer for a trace file."
    )
    p_inspect.add_argument(
        "tracefile", help="Input path for the .bin trace file."
    )
    p_inspect.add_argument(
        "step",
        nargs="?",
        type=int,
        default=0,
        help="Initial step to display (default: 0).",
    )
    p_inspect.set_defaults(func=cmd_inspect)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
