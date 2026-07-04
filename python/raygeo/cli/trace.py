import struct

import msgpack

# ── Trace file format ────────────────────────────────────────────

TRACE_HEADER_SIZE = 12  # magic(4) + version(4) + count(4)
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
    1: "wall_hug",
    2: "segment",
    3: "mat",
    4: "frontier",
    5: "island",
    6: "envelope",
}

ROUTE_SOURCE_NAMES = {
    0: "none",
    1: "direct",
    2: "frontier",
    3: "mat",
    4: "astar",
}

ROUTE_DETAIL_LABELS = {
    0: ".",
    1: "sweep_collide",
    2: "no_obstacles",
    3: "no_frontier",
    4: "offset_empty",
    5: "diff_polygons",
    6: "too_few_verts",
    7: "same_vertex",
    8: "sweep_collide",
    9: "no_axis",
    10: "no_cleared",
    11: "no_path",
    12: "sweep_collide",
    13: "no_obstacles",
    14: "no_free_space",
    15: "astar_failed",
    16: "too_few_waypoints",
}


class TraceRecord:
    """One per-step record from the trace file (msgpack dict wrapper)."""

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
        "route_source",
        "wall_hug_points",
        "wall_hug_segment_counts",
        "resume_strategy_reasons",
        "resume_strategy_details",
        "route_strategy_details",
        "resume_point_x",
        "resume_point_y",
        "resume_candidate_points",
    )

    def __init__(self, d):
        for k, v in d.items():
            if k in self.__slots__:
                setattr(self, k, v)
        if not hasattr(self, "resume_strategy_details"):
            self.resume_strategy_details = [0] * 6
        if not hasattr(self, "route_strategy_details"):
            self.route_strategy_details = [0] * 4
        if not hasattr(self, "resume_point_x"):
            self.resume_point_x = 0.0
            self.resume_point_y = 0.0

    @property
    def wall_hug_count(self):
        return len(self.wall_hug_points)


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
    a single self-contained file.  Records are length-prefixed MessagePack
    blobs.
    """

    def __init__(self, path):
        with open(path, "rb") as f:
            magic = f.read(4)
            if magic != TRACE_MAGIC:
                raise ValueError(f"bad magic: {magic}")
            f.read(4)  # reserved
            self.count = struct.unpack("<I", f.read(4))[0]
            self.geometry = self._read_geometry(f)
            self._read_mat(f)
            self.toolpath = self._read_toolpath(f)
            self._records = []
            for _ in range(self.count):
                rec_len = struct.unpack("<I", f.read(4))[0]
                rec_bytes = f.read(rec_len)
                rec_dict = msgpack.unpackb(rec_bytes)
                self._records.append(TraceRecord(rec_dict))

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

    def _read_mat(self, f):
        present = struct.unpack("<B", f.read(1))[0]
        if present:
            n_nodes = struct.unpack("<I", f.read(4))[0]
            self.mat_nodes = []
            self.mat_clearances = []
            for _ in range(n_nodes):
                x = struct.unpack("<d", f.read(8))[0]
                y = struct.unpack("<d", f.read(8))[0]
                c = struct.unpack("<d", f.read(8))[0]
                self.mat_nodes.append((x, y))
                self.mat_clearances.append(c)
            n_edges = struct.unpack("<I", f.read(4))[0]
            self.mat_edges = []
            for _ in range(n_edges):
                i = struct.unpack("<I", f.read(4))[0]
                j = struct.unpack("<I", f.read(4))[0]
                self.mat_edges.append((i, j))
            self.mat_root = struct.unpack("<I", f.read(4))[0]
        else:
            self.mat_nodes = []
            self.mat_clearances = []
            self.mat_edges = []
            self.mat_root = 0

    def __len__(self):
        return len(self._records)

    def __getitem__(self, idx):
        if idx < 0 or idx >= len(self._records):
            raise IndexError(idx)
        return self._records[idx]
