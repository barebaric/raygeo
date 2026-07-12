import math
from typing import Any

import matplotlib.pyplot as plt
import matplotlib.widgets as _mw
from matplotlib.collections import LineCollection
from matplotlib.patches import Circle
from matplotlib.widgets import Button, TextBox

from raygeo.cli.cleared import rebuild_cleared
from raygeo.geo.shape.polygon import get_polygon_signed_area
from raygeo.ops.part import StockRegion
from raygeo.trace import TraceFile, get_route_detail_name


# ── Workaround for matplotlib 3.11 bug ────────────────────────────
# https://github.com/matplotlib/matplotlib/issues/22409
# ResizeEvent lacks the 'inaxes' attribute, causing AttributeError
# in the widget event decorator (_call_with_reparented_event).
# Patch every decorated event handler to add 'inaxes' if missing.
def patch_widget_events() -> None:
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


patch_widget_events()


# ── Event field accessors ─────────────────────────────────────────
# Thin read-helpers over the span/event reader API.  The trace stores
# per-step data in `event.tool` (position/heading), `event.progress`
# (step_idx/ops_len/status) and `event.meta` (a dict of assembler-
# specific scalars/lists).  These helpers read with sensible defaults.

_STATUS = {0: "Ok", 1: "BoundaryHit", 2: "LostEngagement", 3: "NoConvergence"}


def _meta(event, key, default: Any = 0.0) -> Any:
    m = event.meta
    return m[key] if m and key in m else default


def _has(event, key):
    m = event.meta
    return bool(m and key in m)


def _kind_name(event) -> str:
    if event.kind == "move":
        return event.move_kind or "move"
    return event.kind


def _status_name(event) -> str:
    if event.progress is not None:
        return _STATUS.get(event.progress.status, "?")
    return ""


# ── Inspector ─────────────────────────────────────────────────────


class Inspector:
    def __init__(self, trace: TraceFile):
        self.trace = trace
        # The assembler span (adaptive/profile) carries the geometry &
        # MAT setup in its attrs.  Fall back to the root span.
        self.assembler_span = next(
            (s for s in trace.spans if s.source in ("adaptive", "profile")),
            trace.root,
        )
        geo = self.assembler_span.attrs if self.assembler_span else {}
        self.geometry = geo
        self._mat = geo  # mat_nodes / mat_edges / ... live in attrs
        self.seed_polys = [
            [(p[0], p[1]) for p in poly] for poly in geo.get("seeds", [])
        ]
        self.n_steps = len(trace.events)
        self.current = 0
        self._ca_cache = {}
        self.show_mat = False
        # The toolpath has one entry per event (init/move/resume/exit) so
        # that step index maps 1:1 to toolpath index — matching the
        # original per-record toolpath semantics.
        self.tp = self._event_toolpath()
        self.tp_was_synthetic = True
        self._precompute_toolpath()
        self._precompute_bridge_segments(self.tp_was_synthetic)
        self._build_segment_steps()

        self.fig, (self.ax, self.ax_panel) = plt.subplots(
            1,
            2,
            figsize=(16, 9),
            gridspec_kw={"width_ratios": [3, 1]},
        )
        self.fig.subplots_adjust(
            bottom=0.18, top=0.95, left=0.05, right=0.95, wspace=0.05
        )
        self.ax.set_aspect("equal")
        self.ax_panel.axis("off")

        ax_text = self.fig.add_axes((0.06, 0.06, 0.16, 0.04))
        ax_btn = self.fig.add_axes((0.23, 0.06, 0.07, 0.04))
        ax_prev = self.fig.add_axes((0.31, 0.06, 0.04, 0.04))
        ax_next = self.fig.add_axes((0.36, 0.06, 0.04, 0.04))
        ax_prev_seg = self.fig.add_axes((0.41, 0.06, 0.07, 0.04))
        ax_next_seg = self.fig.add_axes((0.49, 0.06, 0.07, 0.04))
        ax_mat = self.fig.add_axes((0.58, 0.06, 0.07, 0.04))

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
        self.btn_mat = Button(ax_mat, "MAT: Off")
        self.btn_mat.on_clicked(self._toggle_mat)

        self.fig.canvas.mpl_connect("key_press_event", self._on_key)

        self._draw(0)

    def _rec(self, step_idx: int):
        """Return the event for a motion step index."""
        return self.trace.events[step_idx]

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
        if not self._seg_start_steps:
            return
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
        if self._segment_starts:
            n_seg = len(self._segment_starts)
            seg_steps = [0] * n_seg
            si = 1
            for step_idx in range(1, self.n_steps):
                n_moves = step_idx + 1
                while si < n_seg and n_moves > self._segment_starts[si][2]:
                    seg_steps[si] = step_idx
                    si += 1
                if si >= n_seg:
                    break
            for j in range(si, n_seg):
                seg_steps[j] = self.n_steps - 1
            self._seg_start_steps = seg_steps
            return

        # Fallback: roughly equal-sized chunks.
        n_seg = max(1, min(20, self.n_steps // 50))
        seg_size = max(1, self.n_steps // n_seg)
        seg_steps = list(range(0, self.n_steps, seg_size))
        if len(seg_steps) > 1 and seg_steps[-1] < self.n_steps - 1:
            seg_steps[-1] = self.n_steps - 1
        self._seg_start_steps = seg_steps

    def _step(self, delta):
        if delta < 0 and self.current == 0:
            self._draw(self.n_steps - 1)
        else:
            self._draw(self.current + delta)

    def _toggle_mat(self, _event=None):
        self.show_mat = not self.show_mat
        self.btn_mat.label.set_text("MAT: On" if self.show_mat else "MAT: Off")
        self._draw(self.current)

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
        elif event.key == "m":
            self._toggle_mat()

    def _event_toolpath(self):
        """One (x, y, move_kind) entry per event, in order.

        The inspector steps through events one at a time and needs each
        event's tool position classified as cutting or not for toolpath
        rendering, segment detection and cleared-area rebuild.  A move
        event keeps its own move_kind; init/resume/exit are positioning
        (non-cutting) points so the toolpath length equals the event
        count and step index maps 1:1 to a position.
        """
        tp = []
        for ev in self.trace.events:
            if ev.kind == "move":
                mk = ev.move_kind or "travel"
            else:
                mk = "travel"
            t = ev.tool
            tp.append((t.pos_x if t else 0.0, t.pos_y if t else 0.0, mk))
        return tp

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
            x, y, move_kind = self.tp[i]
            is_travel = move_kind != "cut"

            if is_travel:
                if prev_any is not None:
                    self._all_travel_segs.append([prev_any, (x, y)])
                    travel_count += 1
                prev_any = (x, y)
                # A Resume entry's position IS where the next cut resumes,
                # not a travel destination that needs a positioning move.
                # Set cut_prev so the following Cut creates a cut edge.
                cut_prev = (x, y)
            else:
                if cut_prev is None and prev_any is not None:
                    # First cut after travel: draw edge from the
                    # travel destination to this first cut endpoint
                    # as a travel segment (it's a positioning/plunge
                    # move, not an actual cutting edge).
                    self._all_travel_segs.append([prev_any, (x, y)])
                    travel_count += 1
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

    def _precompute_bridge_segments(self, tp_was_synthetic: bool = False):
        """Precompute the bridge segment for each event — the dashed line
        from the last toolpath point to the recorded tool centre position
        when they differ (e.g. after a resume travel that doesn't exactly
        reach the recorded pose).  Stored per event so they accumulate
        visually across steps.

        When *tp_was_synthetic* is true the real toolpath was empty
        (error/partial trace), so no bridges are computed.
        """
        self._all_bridge_segs: list[list[tuple[float, float]] | None] = [
            None
        ] * self.n_steps
        if tp_was_synthetic:
            return

        tp = self.tp
        for i in range(self.n_steps):
            ev = self._rec(i)
            if ev.tool is None:
                continue
            n_moves = i + 1
            if n_moves > len(tp):
                break
            last = tp[n_moves - 1]
            dx = ev.tool.pos_x - last[0]
            dy = ev.tool.pos_y - last[1]
            if math.hypot(dx, dy) > 0.01:
                self._all_bridge_segs[i] = [
                    (last[0], last[1]),
                    (ev.tool.pos_x, ev.tool.pos_y),
                ]

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
        self.textbox.set_val(str(step_idx))

        self.ax.clear()
        self.ax.set_aspect("equal")

        ev = self._rec(step_idx)

        geo = self.geometry
        boundary = geo.get("boundary", [])
        islands = geo.get("islands", [])
        tool_radius = geo.get("tool_radius", 0.0)

        # ── Boundary ──
        if boundary:
            bx = [p[0] for p in boundary] + [boundary[0][0]]
            by = [p[1] for p in boundary] + [boundary[0][1]]
            self.ax.plot(bx, by, "k-", linewidth=1.5)
        else:
            bx, by = [], []

        # ── Seed / initial outline ──
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

        # ── Cleared-area rendering (adaptive traces) ──
        has_seeds = bool(self.seed_polys) or "seeds" in geo
        n_tp_moves = step_idx + 1
        if has_seeds and tool_radius:
            region = StockRegion(
                boundary=[tuple(p) for p in boundary],
                islands=[[tuple(q) for q in isl] for isl in islands],
            )
            ca0 = self._get_cleared(0)
            envelope = ca0.envelope(region, tool_radius)
            for env in envelope:
                if len(env) < 3:
                    continue
                ex = [p[0] for p in env] + [env[0][0]]
                ey = [p[1] for p in env] + [env[0][1]]
                self.ax.plot(ex, ey, "b--", linewidth=0.7, alpha=0.5)

            n_cuts = sum(
                1 for i in range(n_tp_moves) if self.tp[i][2] == "cut"
            )
            ca = self._get_cleared(n_cuts)

            if boundary:
                bxf = [p[0] for p in boundary] + [boundary[0][0]]
                byf = [p[1] for p in boundary] + [boundary[0][1]]
                self.ax.fill(bxf, byf, color="white")

            _remaining = ca.remaining(region)
            _frontier = ca.frontier(region, 0.05)
            for poly in _remaining:
                if len(poly) < 3:
                    continue
                a = get_polygon_signed_area(poly)
                rx = [p[0] for p in poly] + [poly[0][0]]
                ry = [p[1] for p in poly] + [poly[0][1]]
                if a > 0:
                    self.ax.fill(rx, ry, color="#ffcccc")
                    self.ax.plot(rx, ry, color="#cc5555", linewidth=0.3)
                else:
                    self.ax.fill(rx, ry, color="white")

            for poly in _frontier:
                if len(poly) < 3:
                    continue
                fx = [p[0] for p in poly] + [poly[0][0]]
                fy = [p[1] for p in poly] + [poly[0][1]]
                self.ax.plot(fx, fy, color="#88dd88", linewidth=0.6, alpha=0.7)

        # ── Offset polygons (profile traces) ──
        offset_polys = geo.get("offset_polys", [])
        if offset_polys:
            palette = ["#3498db", "#e67e22", "#2ecc71", "#e74c3c", "#9b59b6"]
            for pi, op in enumerate(offset_polys):
                ox = [p[0] for p in op] + [op[0][0]]
                oy = [p[1] for p in op] + [op[0][1]]
                self.ax.plot(
                    ox,
                    oy,
                    linestyle="--",
                    linewidth=1.0,
                    color=palette[pi % len(palette)],
                    alpha=0.6,
                    label=f"offset poly {pi}" if pi == 0 else None,
                )

        # ── Islands ──
        for isl in islands:
            ix = [p[0] for p in isl] + [isl[0][0]]
            iy = [p[1] for p in isl] + [isl[0][1]]
            self.ax.fill(ix, iy, color="gray")
            self.ax.plot(ix, iy, color="dimgray", linewidth=1.0)

        # ── Toolpath ──
        self._draw_toolpath(n_tp_moves)

        # ── Tool position ──
        if ev.tool is not None:
            self._draw_tool(ev)

        # ── Wall hug points (if the event carries them) ──
        if _has(ev, "wall_hug_points"):
            self._draw_wall_hug(ev)

        # ── MAT overlay (if MAT data exists) ──
        if self._mat.get("mat_nodes"):
            self._draw_mat()

        # ── Polygon start marker (profile: first cut of a polygon) ──
        if _meta(ev, "polygon_start", False) and ev.tool is not None:
            self.ax.axvline(
                x=ev.tool.pos_x,
                linestyle=":",
                color="#555555",
                linewidth=0.8,
                alpha=0.7,
            )

        # ── Title (minimal — details in right panel) ──
        kind_name = _kind_name(ev)
        status_name = _status_name(ev)
        src = ev.source or ""
        self.ax.set_xlabel("X")
        self.ax.set_ylabel("Y")
        # Auto-fit axis limits to the boundary with a small margin.
        if bx:
            bx_min, bx_max = min(bx), max(bx)
            by_min, by_max = min(by), max(by)
            mx = (bx_max - bx_min) * 0.05 + tool_radius
            my = (by_max - by_min) * 0.05 + tool_radius
            self.ax.set_xlim(bx_min - mx, bx_max + mx)
            self.ax.set_ylim(by_min - my, by_max + my)
        self.ax.grid(True, alpha=0.2)

        # ── Right panel: parameter table ──
        self.ax_panel.clear()
        self.ax_panel.axis("off")
        cell_text, cell_colors, fmt = self._format_panel_data(
            ev, kind_name, status_name, src
        )
        tbl = self.ax_panel.table(
            cellText=cell_text,
            cellColours=cell_colors,
            cellLoc="left",
            loc="upper left",
            colWidths=[0.35, 0.65],
            edges="closed",
        )
        tbl.auto_set_font_size(False)
        tbl.set_fontsize(7)
        tbl.scale(1, 1.3)
        for r, c, kw in fmt:
            tbl[r, c].get_text().set(**kw)

        self.fig.canvas.draw_idle()

    def _draw_mat(self):
        nodes = self._mat.get("mat_nodes")
        edges = self._mat.get("mat_edges") or []
        clearances = self._mat.get("mat_clearances")
        root = self._mat.get("mat_root")
        if not nodes or not edges or not self.show_mat:
            return

        for i, j in edges:
            self.ax.plot(
                [nodes[i][0], nodes[j][0]],
                [nodes[i][1], nodes[j][1]],
                color="#4a90d9",
                linewidth=0.6,
                alpha=0.5,
                zorder=6,
            )
        xs = [n[0] for n in nodes]
        ys = [n[1] for n in nodes]
        self.ax.scatter(
            xs, ys, c=clearances, cmap="viridis", s=4, alpha=0.6, zorder=6
        )
        self.ax.plot(
            nodes[root][0],
            nodes[root][1],
            "r*",
            markersize=8,
            zorder=6,
        )

    def _draw_toolpath(self, n_moves):
        """Draw toolpath up to the moving-command count in this event."""
        if n_moves == 0 or n_moves > len(self._move_to_edge_count):
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

        # Bridge segments: draw all gaps between the toolpath endpoint
        # and the recorded tool position from all events up to the
        # current step, accumulating so previously-drawn bridges persist.
        for i in range(self.current + 1):
            seg = self._all_bridge_segs[i]
            if seg is not None:
                self.ax.plot(
                    [seg[0][0], seg[1][0]],
                    [seg[0][1], seg[1][1]],
                    linestyle="--",
                    linewidth=0.8,
                    color="dimgray",
                    alpha=0.7,
                    zorder=3,
                )

    def _draw_tool(self, ev):
        """Draw tool circle, position dot, and heading arrow."""
        t = ev.tool
        x, y = t.pos_x, t.pos_y
        r = self.geometry.get("tool_radius", 0.0)

        circle = Circle(
            (x, y), r, fill=False, edgecolor="red", linewidth=1.5, alpha=0.8
        )
        self.ax.add_patch(circle)
        self.ax.plot(x, y, "ro", markersize=4)

        # Heading arrow (red)
        h = t.heading
        dx = math.cos(h) * r * 2
        dy = math.sin(h) * r * 2
        self.ax.annotate(
            "",
            xy=(x + dx, y + dy),
            xytext=(x, y),
            arrowprops=dict(arrowstyle="->", color="red", lw=1.5),
        )

        # Smoothed heading (orange, shorter)
        sh = _meta(ev, "smoothed_heading", h)
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

    def _draw_wall_hug(self, ev):
        counts = _meta(ev, "wall_hug_segment_counts", [])
        pts = _meta(ev, "wall_hug_points", [])
        if not counts:
            counts = [len(pts)]
        idx = 0
        for seg_i, count in enumerate(counts):
            current = seg_i == 0
            alpha = 0.9 if current else 0.4
            ms = 3 if current else 2
            for _ in range(count):
                if idx >= len(pts):
                    break
                wx, wy = pts[idx][0], pts[idx][1]
                idx += 1
                if math.isnan(wx) or math.isnan(wy):
                    continue
                self.ax.plot(
                    wx,
                    wy,
                    "o",
                    color="gold",
                    markersize=ms,
                    zorder=7,
                    alpha=alpha,
                )

    def _format_panel_data(self, ev, kind_name, status_name, source):
        """Build a 2-column table for the right-hand info panel.

        Returns (cell_text, cell_colours, extra_styles):
          cell_text  — list of [label, value] row strings
          cell_colours — parallel list of [label_bg, value_bg] RGBA tuples
          extra_styles — list of (row, col, props) applied via .set(**props)
        """
        t = ev.tool
        pos_x = t.pos_x if t else 0.0
        pos_y = t.pos_y if t else 0.0
        pos_z = t.pos_z if t else 0.0
        prev_x = t.prev_x if t else 0.0
        prev_y = t.prev_y if t else 0.0
        prev_z = t.prev_z if t else 0.0
        heading = t.heading if t else 0.0

        h_deg = math.degrees(heading)
        sh_deg = math.degrees(_meta(ev, "smoothed_heading", heading))
        pa_deg = math.degrees(_meta(ev, "predicted_angle", 0.0))
        ia_deg = math.degrees(_meta(ev, "iteration_angle", 0.0))
        eng_deg = math.degrees(_meta(ev, "eng_angle", 0.0))
        step_dist = math.hypot(pos_x - prev_x, pos_y - prev_y)

        HDR = (0.15, 0.35, 0.55, 1.0)
        SEC = (0.92, 0.92, 0.92, 1.0)
        WHT = (1.0, 1.0, 1.0, 1.0)
        FAIL = (1.0, 0.85, 0.85, 1.0)
        WIN = (0.85, 1.0, 0.85, 1.0)

        cells = []
        colors = []
        styles = []

        def _cell(label, value, bg=WHT, **kw):
            cells.append([label, value])
            colors.append([bg, bg])
            if kw:
                r = len(cells) - 1
                styles.append((r, 0, kw))
                styles.append((r, 1, kw))

        step_idx = ev.progress.step_idx if ev.progress else 0

        # ── Header ──
        cells.append(
            [
                f"Step {step_idx}/{self.n_steps - 1}",
                f"{kind_name}  {status_name}".strip(),
            ]
        )
        colors.append([HDR, HDR])
        styles.append((0, 0, {"color": "white", "weight": "bold"}))
        styles.append((0, 1, {"color": "white", "weight": "bold"}))
        if source:
            _cell("source", source)

        # ── Position ──
        _cell("Position", "", bg=SEC, weight="bold")
        _cell("pos", f"({pos_x:.1f}, {pos_y:.1f}, {pos_z:.1f})")
        _cell("prev", f"({prev_x:.1f}, {prev_y:.1f}, {prev_z:.1f})")
        _cell("step_dist", f"{step_dist:.2f}")

        # ── Heading ──
        _cell("Heading", "", bg=SEC, weight="bold")
        _cell("raw", f"{h_deg:.1f}°")
        _cell("smoothed", f"{sh_deg:.1f}°")

        # ── Angles ──
        _cell("Angles", "", bg=SEC, weight="bold")
        _cell("predicted", f"{pa_deg:.1f}°")
        _cell("iteration", f"{ia_deg:.1f}°")
        _cell("eng_angle", f"{eng_deg:.1f}°")

        # ── Engagement ──
        _cell("Engagement", "", bg=SEC, weight="bold")
        _cell("eng_area", f"{_meta(ev, 'eng_area', 0.0):.3f}")
        _cell("eng_chord", f"{_meta(ev, 'eng_chord', 0.0):.3f}")

        # ── Area (mm²) ──
        _cell("Area (mm²)", "", bg=SEC, weight="bold")
        cut_area = _meta(ev, "cut_area", None)
        total_area = _meta(ev, "total_area", None)
        remaining = _meta(ev, "remaining_area", None)
        _cell("cut_area", f"{cut_area:.1f}" if cut_area is not None else "—")
        _cell(
            "cleared", f"{total_area:.1f}" if total_area is not None else "—"
        )
        _cell(
            "remaining",
            f"{remaining:.1f}" if remaining is not None else "—",
        )

        # ── Misc ──
        _cell("Misc", "", bg=SEC, weight="bold")
        _cell("iters", str(_meta(ev, "iters", 0)))
        _cell("ops_len", str(ev.progress.ops_len if ev.progress else 0))

        # ── Polygon tracking (profile events) ──
        if _has(ev, "target_polygon_idx"):
            _cell("Polygon", "", bg=SEC, weight="bold")
            cp = _meta(ev, "polygon_perimeter", 0.0)
            cum = _meta(ev, "cumulative_distance", 0.0)
            _cell("target_idx", str(int(_meta(ev, "target_polygon_idx", 0))))
            _cell("perimeter", f"{cp:.2f}")
            _cell("cum_dist", f"{cum:.2f}")
            progress = (cum / cp * 100) if cp and cp > 0 else 0.0
            _cell("progress", f"{progress:.1f}%")
            _cell("wall_dist", f"{_meta(ev, 'wall_distance', 0.0):.3f}")
            _cell("feed_rate", f"{_meta(ev, 'current_feed_rate', 0)} mm/min")
            _cell("step_len", f"{_meta(ev, 'step_length', 0.0):.3f}")
            _cell(
                "reductions", str(int(_meta(ev, "engagement_reductions", 0)))
            )

        # ── Strategy (adaptive resume events) ──
        rs = _meta(ev, "resume_source", 0)
        if rs:
            _cell("Resume Strategy", "", bg=SEC, weight="bold")

            # Priority order (index 0-5) maps to ResumeSource values as:
            #   0→1(WallHug), 1→2(Segment), 2→3(Mat),
            #   3→4(Frontier), 4→6(Envelope), 5→5(Island)
            src_to_idx = {1: 0, 2: 1, 3: 2, 4: 3, 6: 4, 5: 5}
            win_idx = src_to_idx.get(rs, -1)
            strat_names = [
                "wall_hug",
                "segment",
                "mat",
                "frontier",
                "envelope",
                "island",
            ]
            detail_labels = {
                0: "",
                1: "no_fragments",
                2: "no_growth",
                3: "outside_valid",
                4: "no_wall_hug_pt",
                5: "node_not_cleared",
                6: "no_crossing",
                7: "no_envelope",
                8: "no_frontier",
                9: "no_polygons",
                10: "no_holes",
                11: "no_engagement",
                12: "blacklisted",
                13: "no_wall_hit",
            }

            reasons = _meta(ev, "resume_reasons", [])
            details = _meta(ev, "resume_details", [])
            candidates = _meta(ev, "candidates", [])
            for i, name in enumerate(strat_names):
                val = reasons[i] if i < len(reasons) else 0
                det = details[i] if i < len(details) else 0
                if i < len(candidates):
                    c = candidates[i]
                    cpx, cpy, cpz = c[0], c[1], c[2]
                else:
                    cpx = cpy = cpz = float("nan")
                if math.isnan(cpx) or math.isnan(cpy) or math.isnan(cpz):
                    if i == win_idx:
                        _cell(name, "ok", bg=WIN, color="darkgreen")
                    elif val == 0:
                        _cell(name, "not_tried", color="gray")
                    else:
                        label = "no_candidate"
                        if det:
                            label += f" ({detail_labels.get(det, '?')})"
                        _cell(
                            name,
                            label,
                            bg=FAIL,
                            color="darkred",
                        )
                else:
                    _cell(
                        name,
                        f"({cpx:.3f}, {cpy:.3f}, {cpz:.3f})",
                        bg=WIN,
                        color="darkgreen",
                    )

            rsrc = _meta(ev, "route_source", 0)
            route_names = ["direct", "frontier", "mat", "zhop"]
            win_route = rsrc - 1 if rsrc > 0 else -1
            _cell("Routing", "", bg=SEC, weight="bold")
            route_details = _meta(ev, "route_details", [])
            for i, name in enumerate(route_names):
                det = route_details[i] if i < len(route_details) else 0
                if i == win_route:
                    _cell(name, "ok", bg=WIN, color="darkgreen")
                elif det == 0:
                    _cell(name, "not_tried", color="gray")
                else:
                    _cell(
                        name,
                        get_route_detail_name(det),
                        bg=FAIL,
                        color="darkred",
                    )

        return cells, colors, styles
