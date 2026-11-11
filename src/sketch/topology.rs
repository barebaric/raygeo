//! Sketch topology: chain assembly and closed-loop detection.
//!
//! These functions work on flat descriptors — entity ids, endpoint
//! point ids, coincident groups and caller-precomputed tangent
//! angles — so entity kinds and geometry emission stay with the
//! caller.  [`plan_chains`] mirrors the reference chain assembly of
//! `Sketch.to_geometry` (union-find over coincident points, then
//! growing continuous paths from each unvisited edge);
//! [`find_closed_loops`] mirrors the reference face traversal
//! (`_find_all_closed_loops`), including the circular-segment area
//! correction for arcs and the positive-area filter.

use std::collections::{BTreeMap, HashMap, HashSet};

/// An edge of the sketch graph: an entity with exactly two
/// endpoints.
#[derive(Clone, Debug)]
pub struct SketchEdge {
    /// Unique entity id.
    pub entity_id: u64,
    /// First endpoint point id.
    pub p1: u64,
    /// Second endpoint point id.
    pub p2: u64,
    /// atan2 of the entity tangent leaving from `p1`, precomputed by
    /// the caller (tangents are entity-kind-specific).
    pub angle_at_p1: f64,
    /// atan2 of the entity tangent leaving from `p2`.
    pub angle_at_p2: f64,
}

/// An arc edge, for the circular-segment correction of loop areas.
#[derive(Clone, Debug)]
pub struct SketchArc {
    /// Entity id matching a [`SketchEdge`] of the same sketch.
    pub entity_id: u64,
    /// Arc start point id.
    pub start: u64,
    /// Arc end point id.
    pub end: u64,
    /// Arc center point id.
    pub center: u64,
    /// Chirality of the arc.
    pub clockwise: bool,
}

/// A closed single-entity loop (circle, ellipse), reported as its
/// own face with the caller-computed enclosed signed area.
#[derive(Clone, Debug)]
pub struct ClosedEntity {
    /// Unique entity id.
    pub entity_id: u64,
    /// Enclosed signed area (positive for counter-clockwise).
    pub signed_area: f64,
}

/// Node-rooted adjacency: nodes in first-seen order, each holding
/// `(edge index, neighbor node)` entries in insertion order.
struct RootAdjacency {
    node_order: Vec<u64>,
    entries: HashMap<u64, Vec<(usize, u64)>>,
}

/// Union-find over point ids with the reference implementation's
/// attachment rule (the first root attaches to the second) and path
/// compression, so node identity matches the reference exactly.
struct UnionFind {
    parent: BTreeMap<u64, u64>,
}

impl UnionFind {
    fn new(points: &[u64]) -> Self {
        let parent = points
            .iter()
            .map(|&id| (id, id))
            .collect::<BTreeMap<u64, u64>>();
        UnionFind { parent }
    }

    fn find(&mut self, id: u64) -> u64 {
        let mut root = id;
        while let Some(&next) = self.parent.get(&root) {
            if next == root {
                break;
            }
            root = next;
        }
        let mut current = id;
        while let Some(&next) = self.parent.get(&current) {
            if next == current {
                break;
            }
            self.parent.insert(current, root);
            current = next;
        }
        root
    }

    fn union(&mut self, a: u64, b: u64) {
        let root_a = self.find(a);
        let root_b = self.find(b);
        if root_a != root_b {
            self.parent.insert(root_a, root_b);
        }
    }
}

/// Builds the node-rooted adjacency list.  Nodes are the union-find
/// roots of the endpoints; each undirected edge appears as two
/// directed entries appended in the same order as the reference.
fn build_root_adjacency(
    edges: &[SketchEdge],
    parent: &mut UnionFind,
) -> RootAdjacency {
    let mut result = RootAdjacency {
        node_order: Vec::new(),
        entries: HashMap::new(),
    };

    for (edge_index, edge) in edges.iter().enumerate() {
        let root_u = parent.find(edge.p1);
        let root_v = parent.find(edge.p2);
        for node in [root_u, root_v] {
            if !result.node_order.contains(&node) {
                result.node_order.push(node);
            }
        }
        result
            .entries
            .entry(root_u)
            .or_default()
            .push((edge_index, root_v));
        result
            .entries
            .entry(root_v)
            .or_default()
            .push((edge_index, root_u));
    }
    result
}

/// Partitions the edges into continuous chains of
/// `(entity_id, forward)` tuples, mirroring the reference chain
/// assembly: coincident points collapse to one node, and each
/// unvisited edge seeds a path grown in both directions, taking the
/// first unvisited continuation at every step.
pub fn plan_chains(
    edges: &[SketchEdge],
    coincident_pairs: &[(u64, u64)],
) -> Vec<Vec<(u64, bool)>> {
    let points: Vec<u64> =
        edges.iter().flat_map(|edge| [edge.p1, edge.p2]).collect();
    let mut parent = UnionFind::new(&points);
    for &(a, b) in coincident_pairs {
        if parent.parent.contains_key(&a) && parent.parent.contains_key(&b) {
            parent.union(a, b);
        }
    }

    let adjacency = build_root_adjacency(edges, &mut parent);

    let mut visited: HashSet<u64> = HashSet::new();
    let mut chains = Vec::new();

    for edge in edges {
        if visited.contains(&edge.entity_id) {
            continue;
        }
        visited.insert(edge.entity_id);

        let u = parent.find(edge.p1);
        let v = parent.find(edge.p2);

        // Grow right: from the seed's far endpoint, following flow
        // node -> node.
        let mut right: Vec<(usize, bool)> = Vec::new();
        let mut current = v;
        loop {
            let found = adjacency.entries.get(&current).and_then(|entries| {
                entries
                    .iter()
                    .find(|(edge_index, _)| {
                        !visited.contains(&edges[*edge_index].entity_id)
                    })
                    .copied()
            });
            let Some((edge_index, next_node)) = found else {
                break;
            };
            visited.insert(edges[edge_index].entity_id);
            let is_forward = parent.find(edges[edge_index].p1) == current;
            right.push((edge_index, is_forward));
            current = next_node;
        }

        // Grow left: walking backwards from the seed's near endpoint;
        // the continuation flows next_node -> current.
        let mut left: Vec<(usize, bool)> = Vec::new();
        let mut current = u;
        loop {
            let found = adjacency.entries.get(&current).and_then(|entries| {
                entries
                    .iter()
                    .find(|(edge_index, _)| {
                        !visited.contains(&edges[*edge_index].entity_id)
                    })
                    .copied()
            });
            let Some((edge_index, next_node)) = found else {
                break;
            };
            visited.insert(edges[edge_index].entity_id);
            let is_forward = parent.find(edges[edge_index].p1) == next_node;
            left.push((edge_index, is_forward));
            current = next_node;
        }

        let mut chain: Vec<(u64, bool)> =
            Vec::with_capacity(left.len() + right.len() + 1);
        for (edge_index, is_forward) in left.iter().rev() {
            chain.push((edges[*edge_index].entity_id, *is_forward));
        }
        chain.push((edge.entity_id, true));
        for (edge_index, is_forward) in &right {
            chain.push((edges[*edge_index].entity_id, *is_forward));
        }
        chains.push(chain);
    }

    chains
}

/// Maps each point id to its coincident group (points outside any
/// group are absent and fall back to a singleton).
fn group_lookup(coincident_groups: &[Vec<u64>]) -> HashMap<u64, Vec<u64>> {
    let mut lookup: HashMap<u64, Vec<u64>> = HashMap::new();
    for group in coincident_groups {
        for pid in group {
            lookup.entry(*pid).or_insert_with(|| group.clone());
        }
    }
    lookup
}

/// Signed area of a loop: the shoelace formula over the endpoint
/// walk, refined by the circular-segment correction for arcs.
fn loop_signed_area(
    loop_edges: &[(u64, bool)],
    point_coords: &HashMap<u64, (f64, f64)>,
    edge_by_id: &HashMap<u64, &SketchEdge>,
    arcs_by_id: &HashMap<u64, &SketchArc>,
) -> f64 {
    if loop_edges.is_empty() {
        return 0.0;
    }

    // Walk the endpoints: start at the loop's first edge's leaving
    // point, then hop to the far endpoint of each edge.
    let Some(&first_edge) = edge_by_id.get(&loop_edges[0].0) else {
        return 0.0;
    };
    let mut current = if loop_edges[0].1 {
        first_edge.p1
    } else {
        first_edge.p2
    };
    let mut points: Vec<(f64, f64)> = Vec::new();
    for (entity_id, _) in loop_edges {
        let Some(&coord) = point_coords.get(&current) else {
            return 0.0;
        };
        points.push(coord);
        let Some(edge) = edge_by_id.get(entity_id) else {
            return 0.0;
        };
        current = if current == edge.p1 { edge.p2 } else { edge.p1 };
    }

    let mut area = 0.0;
    for i in 0..points.len() {
        let p1 = points[i];
        let p2 = points[(i + 1) % points.len()];
        area += p1.0 * p2.1 - p2.0 * p1.1;
    }
    area *= 0.5;

    for (entity_id, fwd) in loop_edges {
        let Some(arc) = arcs_by_id.get(entity_id) else {
            continue;
        };
        let Some(&(sx, sy)) = point_coords.get(&arc.start) else {
            continue;
        };
        let Some(&(ex, ey)) = point_coords.get(&arc.end) else {
            continue;
        };
        let Some(&(cx, cy)) = point_coords.get(&arc.center) else {
            continue;
        };

        let radius_sq = (sx - cx) * (sx - cx) + (sy - cy) * (sy - cy);
        let ang_start = (sy - cy).atan2(sx - cx);
        let ang_end = (ey - cy).atan2(ex - cx);

        let mut diff = if arc.clockwise {
            ang_start - ang_end
        } else {
            ang_end - ang_start
        };
        while diff < 0.0 {
            diff += 2.0 * std::f64::consts::PI;
        }
        while diff >= 2.0 * std::f64::consts::PI {
            diff -= 2.0 * std::f64::consts::PI;
        }

        let segment_area = 0.5 * radius_sq * (diff - diff.sin());
        let is_ccw_arc = !arc.clockwise;
        if is_ccw_arc == *fwd {
            area += segment_area;
        } else {
            area -= segment_area;
        }
    }

    area
}

/// Finds all closed loops (faces) of the sketch graph, mirroring the
/// reference half-edge traversal: edges are grouped into nodes by
/// coincident group, outgoing edges are sorted by their leaving
/// tangent angle, faces are traced by always taking the previous
/// edge in CCW order, and only positively-oriented loops (area
/// greater than 1e-6) are kept, with closed single-entity loops
/// appended unconditionally at the end.
pub fn find_closed_loops(
    edges: &[SketchEdge],
    points: &[(u64, f64, f64)],
    coincident_groups: &[Vec<u64>],
    arcs: &[SketchArc],
    closed_entities: &[ClosedEntity],
) -> Vec<Vec<(u64, bool)>> {
    let point_coords: HashMap<u64, (f64, f64)> =
        points.iter().map(|(id, x, y)| (*id, (*x, *y))).collect();
    let edge_by_id: HashMap<u64, &SketchEdge> =
        edges.iter().map(|e| (e.entity_id, e)).collect();
    let arcs_by_id: HashMap<u64, &SketchArc> =
        arcs.iter().map(|a| (a.entity_id, a)).collect();
    let groups = group_lookup(coincident_groups);

    // Adjacency: node point id -> directed half-edges.  Edges are
    // expanded across all coincident group members, exactly like the
    // reference adjacency builder.
    let mut node_order: Vec<u64> = Vec::new();
    let mut adjacency: HashMap<u64, Vec<(u64, u64, bool, f64)>> =
        HashMap::new();

    for edge in edges {
        let singleton = vec![edge.p1];
        let group1 = groups.get(&edge.p1).unwrap_or(&singleton);
        let fallback;
        let group2 = match groups.get(&edge.p2) {
            Some(group) => group,
            None => {
                fallback = vec![edge.p2];
                &fallback
            }
        };
        for &src in group1 {
            for &dst in group2 {
                if src == dst {
                    continue;
                }
                let entry = adjacency.entry(src).or_default();
                if entry.is_empty() {
                    node_order.push(src);
                }
                entry.push((dst, edge.entity_id, true, edge.angle_at_p1));
            }
        }
        for &src in group2 {
            for &dst in group1 {
                if src == dst {
                    continue;
                }
                let entry = adjacency.entry(src).or_default();
                if entry.is_empty() {
                    node_order.push(src);
                }
                entry.push((dst, edge.entity_id, false, edge.angle_at_p2));
            }
        }
    }

    // Sort each node's outgoing half-edges by leaving angle (stable,
    // like the reference).
    for entries in adjacency.values_mut() {
        entries.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap());
    }

    let mut loops: Vec<Vec<(u64, bool)>> = Vec::new();
    let mut visited_half_edges: HashSet<(u64, u64, bool)> = HashSet::new();

    for &p_start in &node_order {
        let Some(edges_at_node) = adjacency.get(&p_start) else {
            continue;
        };
        let node_edges = edges_at_node.clone();
        for &(to, entity_id, fwd, _) in &node_edges {
            let half_edge_key = (p_start, entity_id, fwd);
            if visited_half_edges.contains(&half_edge_key) {
                continue;
            }

            let mut loop_edges: Vec<(u64, bool)> = Vec::new();
            let mut loop_half_edges: Vec<(u64, u64, bool)> = Vec::new();
            let mut curr_p = p_start;
            let mut curr_edge = (to, entity_id, fwd);
            let mut closed = false;

            for _ in 0..=edges.len() {
                let current_half_edge = (curr_p, curr_edge.1, curr_edge.2);
                if visited_half_edges.contains(&current_half_edge) {
                    loop_edges.clear();
                    break;
                }

                loop_edges.push((curr_edge.1, curr_edge.2));
                loop_half_edges.push(current_half_edge);

                let next_p = curr_edge.0;
                let outgoing = adjacency.get(&next_p);
                let Some(outgoing) = outgoing else {
                    loop_edges.clear();
                    break;
                };
                let rev_fwd = !curr_edge.2;
                let incoming_index =
                    outgoing.iter().position(|&(_, id, f, _)| {
                        id == curr_edge.1 && f == rev_fwd
                    });
                let next_edge = incoming_index.and_then(|index| {
                    outgoing
                        .get((index + outgoing.len() - 1) % outgoing.len())
                        .map(|&(to, id, f, _)| (to, id, f))
                });
                let Some(next_edge) = next_edge else {
                    loop_edges.clear();
                    break;
                };

                let next_key = (next_p, next_edge.1, next_edge.2);
                if next_key == half_edge_key {
                    closed = true;
                    break;
                }

                curr_p = next_p;
                curr_edge = next_edge;
            }

            if !closed {
                loop_edges.clear();
            }

            if !loop_edges.is_empty()
                && loop_signed_area(
                    &loop_edges,
                    &point_coords,
                    &edge_by_id,
                    &arcs_by_id,
                ) > 1e-6
            {
                loops.push(loop_edges.clone());
                visited_half_edges.extend(loop_half_edges);
            }
        }
    }

    for closed in closed_entities {
        loops.push(vec![(closed.entity_id, true)]);
    }

    loops
}
