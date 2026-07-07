//! Narrow-passage detection via clustered edge-pair convex hulls.
//!
//! For each edge in the pocket boundary (outer + holes), the nearest
//! non-excluded edge within `max_width` is found.  Edge pairs are then
//! grouped into connected clusters: two pairs are connected when their
//! edges are adjacent (share a vertex) on the same polygon.  One convex
//! hull is computed per cluster from all collected endpoint vertices.
//! Each hull is clipped to the pocket (outer − holes) to produce the
//! final narrow-region polygon.

use rstar::{PointDistance, RTree, RTreeObject, AABB};

use crate::geo::shape::polygon::{
    get_polygon_convex_hull, get_polygons_group_difference,
    get_polygons_group_intersection, resample_polygon,
};
use crate::types::{Point, Polygon};

/// An edge in the pocket boundary, tagged for the R-tree.
#[derive(Clone, Debug)]
struct EdgeEntry {
    midpoint: Point,
    poly_id: usize,
    edge_idx: usize,
    a: Point,
    b: Point,
}

impl RTreeObject for EdgeEntry {
    type Envelope = AABB<[f64; 2]>;
    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(
            [self.midpoint.x, self.midpoint.y],
            [self.midpoint.x, self.midpoint.y],
        )
    }
}

impl PointDistance for EdgeEntry {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        let dx = self.midpoint.x - point[0];
        let dy = self.midpoint.y - point[1];
        dx * dx + dy * dy
    }
}

/// Union-Find for clustering edge pairs by adjacency.
struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }
    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }
    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            self.parent[ra] = rb;
        }
    }
}

/// Detect narrow passages in a polygon (with optional holes).
///
/// Returns the regions of the polygon that are narrower than `max_width`.
pub fn find_narrow_passages(
    polygon: &Polygon,
    holes: &[Polygon],
    max_width: f64,
) -> Result<Vec<Polygon>, String> {
    if polygon.len() < 3 {
        return Err("polygon must have at least 3 vertices".to_string());
    }
    if max_width <= 0.0 {
        return Err("max_width must be positive".to_string());
    }

    // Collect all boundaries: [outer] ++ holes
    let mut boundaries = vec![polygon.clone()];
    boundaries.extend_from_slice(holes);

    // Resample each boundary so that even narrow gaps on long edges
    // are detected.  Spacing = max_width/10, clamped to [0.1, 2.0].
    let spacing = (max_width / 10.0).clamp(0.1, 2.0);
    let resampled: Vec<Vec<Point>> = boundaries
        .iter()
        .map(|p| resample_polygon(p, spacing))
        .collect();

    // Edge count per polygon for adjacency checks
    let edge_counts: Vec<usize> = resampled.iter().map(|p| p.len()).collect();

    // Build R-tree of all edge midpoints
    let mut entries: Vec<EdgeEntry> = Vec::new();
    for (pid, poly) in resampled.iter().enumerate() {
        let n = poly.len();
        for i in 0..n {
            let j = (i + 1) % n;
            let a = poly[i];
            let b = poly[j];
            let mid = Point::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
            entries.push(EdgeEntry {
                midpoint: mid,
                poly_id: pid,
                edge_idx: i,
                a,
                b,
            });
        }
    }

    if entries.is_empty() {
        return Ok(vec![]);
    }

    let tree = RTree::bulk_load(entries.clone());
    let mw2 = max_width * max_width;

    // Check if an edge is excluded (same edge or adjacent on same polygon).
    let is_excluded =
        |pid: usize, eidx: usize, opid: usize, oeidx: usize| -> bool {
            if pid != opid {
                return false;
            }
            let n = edge_counts[pid];
            let prev = (eidx + n - 1) % n;
            let next = (eidx + 1) % n;
            oeidx == eidx || oeidx == prev || oeidx == next
        };

    // Minimum parallelism (dot product squared ratio) for two edges to be
    // considered "facing" each other rather than meeting at a corner.
    // cos²(80°) ≈ 0.03 — walls up to 80° from parallel are allowed,
    // e.g. diverging island flanks.  Only perpendicular corners are
    // rejected.
    const PARALLEL_THRESH: f64 = 0.03;
    // For same-polygon pairs, the gap direction must be mostly perpendicular
    // to the edge direction (across the passage, not along the wall).
    // cos²(60°) = 0.25 → |cos| ≤ 0.5 → angle ≥ 60° from parallel.
    const ACROSS_THRESH: f64 = 0.25;

    // Collect edge pairs: (edge_a_id, edge_b_id) where edge ids are
    // global indices into `entries`.
    // Each pair also stores its 4 endpoint vertices.
    struct Pair {
        // global edge index in `entries` for edge on poly_a
        ea: usize,
        eb: usize,
        // polygon ids
        pa: usize,
        pb: usize,
        // edge indices within their polygon
        ia: usize,
        ib: usize,
        // endpoint vertices
        pts: [Point; 4],
    }

    let mut pairs: Vec<Pair> = Vec::new();

    for (gi, entry) in entries.iter().enumerate() {
        let bbox = AABB::from_corners(
            [entry.midpoint.x - max_width, entry.midpoint.y - max_width],
            [entry.midpoint.x + max_width, entry.midpoint.y + max_width],
        );

        let mut best_dist2 = mw2;
        let mut best: Option<&EdgeEntry> = None;

        let edx = entry.b.x - entry.a.x;
        let edy = entry.b.y - entry.a.y;
        let elen2 = edx * edx + edy * edy;

        for candidate in tree.locate_in_envelope(&bbox) {
            if is_excluded(
                entry.poly_id,
                entry.edge_idx,
                candidate.poly_id,
                candidate.edge_idx,
            ) {
                continue;
            }
            // Skip pairs where both edges are on the same hole (island).
            if entry.poly_id > 0 && entry.poly_id == candidate.poly_id {
                continue;
            }
            // Skip pairs whose edge directions are nearly perpendicular
            // (corners), keeping only roughly-parallel facing walls.
            let cdx = candidate.b.x - candidate.a.x;
            let cdy = candidate.b.y - candidate.a.y;
            let clen2 = cdx * cdx + cdy * cdy;
            let dot_abs = (edx * cdx + edy * cdy).abs();
            if dot_abs * dot_abs < PARALLEL_THRESH * elen2 * clen2 {
                continue;
            }
            // For same-polygon pairs, also require that the gap direction is
            // roughly perpendicular to the edge (across the passage, not
            // along the same wall).
            if entry.poly_id == candidate.poly_id {
                let vx = candidate.midpoint.x - entry.midpoint.x;
                let vy = candidate.midpoint.y - entry.midpoint.y;
                let v_len2 = vx * vx + vy * vy;
                let v_dot = (vx * edx + vy * edy).abs();
                if v_dot * v_dot > ACROSS_THRESH * v_len2 * elen2 {
                    continue;
                }
            }
            let d2 = (entry.midpoint - candidate.midpoint).length_squared();
            if d2 < best_dist2 {
                best_dist2 = d2;
                best = Some(candidate);
            }
        }

        if let Some(nb) = best {
            // Find the global index of the candidate in `entries`.
            // We need to look it up.  Since `entries` were built in order,
            // compute the global index from poly_id and edge_idx.
            let mut g_nb = 0usize;
            for (k, e) in entries.iter().enumerate() {
                if e.poly_id == nb.poly_id && e.edge_idx == nb.edge_idx {
                    g_nb = k;
                    break;
                }
            }
            // Order the pair so (pa, ia) <= (pb, ib) lexicographically, to
            // make dedup easier later.
            let (pa, ia, ea, pb, ib, eb, pts) = if entry.poly_id < nb.poly_id
                || (entry.poly_id == nb.poly_id
                    && entry.edge_idx <= nb.edge_idx)
            {
                (
                    entry.poly_id,
                    entry.edge_idx,
                    gi,
                    nb.poly_id,
                    nb.edge_idx,
                    g_nb,
                    [entry.a, entry.b, nb.a, nb.b],
                )
            } else {
                (
                    nb.poly_id,
                    nb.edge_idx,
                    g_nb,
                    entry.poly_id,
                    entry.edge_idx,
                    gi,
                    [nb.a, nb.b, entry.a, entry.b],
                )
            };
            pairs.push(Pair {
                ea,
                eb,
                pa,
                pb,
                ia,
                ib,
                pts,
            });
        }
    }

    if pairs.is_empty() {
        return Ok(vec![]);
    }

    // Deduplicate identical pairs (same two global edges).
    pairs.sort_by(|a, b| a.ea.cmp(&b.ea).then(a.eb.cmp(&b.eb)));
    pairs.dedup_by(|a, b| a.ea == b.ea && a.eb == b.eb);

    let npairs = pairs.len();
    let mut uf = UnionFind::new(npairs);

    // Cluster pairs by adjacency on the same polygon.
    // For each polygon, sort pair-indices by edge index on that polygon,
    // then union consecutive pairs whose edge indices differ by ≤ 1
    // (adjacent edges share a vertex).
    for (pid, edge_count) in edge_counts.iter().enumerate() {
        // Collect (edge_idx_on_this_poly, pair_index)
        let mut hits: Vec<(usize, usize)> = Vec::new();
        for (pi, p) in pairs.iter().enumerate() {
            if p.pa == pid {
                hits.push((p.ia, pi));
            }
            if p.pb == pid {
                hits.push((p.ib, pi));
            }
        }
        hits.sort_by_key(|a| a.0);
        // Union consecutive hits that are adjacent (edge_idx diff ≤ 1).
        for w in 1..hits.len() {
            let (e0, p0) = hits[w - 1];
            let (e1, p1) = hits[w];
            let n = edge_count;
            let diff = if e1 >= e0 { e1 - e0 } else { n - e0 + e1 };
            if diff <= 1 {
                uf.union(p0, p1);
            }
        }
        // Also check wrap-around: the last hit and the first hit may be
        // adjacent across the polygon seam (indices n-1 and 0).
        if hits.len() >= 2 {
            let (e_first, p_first) = hits[0];
            let (e_last, p_last) = hits[hits.len() - 1];
            let wrap_diff = e_first + edge_count - e_last;
            if wrap_diff <= 1 {
                uf.union(p_first, p_last);
            }
        }
    }

    // Group pair indices by cluster root.
    let mut clusters: Vec<usize> = (0..npairs).collect();
    for c in clusters.iter_mut() {
        *c = uf.find(*c);
    }
    // Collect vertices per cluster.
    let mut cluster_vertices: std::collections::HashMap<usize, Vec<Point>> =
        std::collections::HashMap::new();
    for (i, p) in pairs.iter().enumerate() {
        let v = cluster_vertices.entry(clusters[i]).or_default();
        v.extend_from_slice(&p.pts);
    }

    // Compute one convex hull per cluster, then clip to pocket.
    let mut hulls: Vec<Polygon> = Vec::new();
    for (_, mut verts) in cluster_vertices {
        verts.sort_by(|a, b| {
            a.x.partial_cmp(&b.x)
                .unwrap()
                .then(a.y.partial_cmp(&b.y).unwrap())
        });
        verts.dedup_by(|a, b| {
            (a.x - b.x).abs() < 1e-10 && (a.y - b.y).abs() < 1e-10
        });
        let hull = get_polygon_convex_hull(&verts);
        if hull.len() >= 3 {
            hulls.push(hull);
        }
    }

    if hulls.is_empty() {
        return Ok(vec![]);
    }

    // Compute pocket = outer − holes (the actual cuttable area).
    let pocket = if holes.is_empty() {
        vec![polygon.clone()]
    } else {
        get_polygons_group_difference(std::slice::from_ref(polygon), holes)
    };

    // Clip each hull to the pocket individually (no union of hulls).
    let mut result: Vec<Polygon> = Vec::new();
    for hull in &hulls {
        let clipped = get_polygons_group_intersection(
            std::slice::from_ref(hull),
            &pocket,
        );
        for p in clipped {
            if p.len() >= 3 {
                result.push(p);
            }
        }
    }

    Ok(result)
}
