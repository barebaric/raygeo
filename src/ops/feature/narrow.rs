use std::collections::HashSet;

use crate::geo::algo::narrow::find_narrow_passages;
use crate::geo::shape::line::get_segment_segment_distance;
use crate::geo::shape::polygon::{find_entry_edges, offset_polygon, JoinStyle};
use crate::geo::types::Polygon;

/// Classification of a narrow passage for machining strategy selection.
///
/// Width hierarchy from widest to narrowest:
///   `Adaptive` (≥ 1.5×D) → **Narrow / toroidal** (D+tol < w < 1.5×D) →
///   **Slot** (D < w ≤ D+tol) → **Unreachable** (w ≤ D)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PassageClass {
    /// Wide enough for toroidal/trochoidal clearing
    /// (`D + tol < width < 1.5×D`).
    Narrow,
    /// Tight enough for slotting (`D < width ≤ D + tol`).
    Slot,
    /// Too small for any tool to pass (`width ≤ D`).
    Unreachable,
}

/// A classified narrow region with entry-edge information.
#[derive(Clone, Debug)]
pub struct NarrowRegion {
    /// The exact polygon of the narrow passage.
    pub polygon: Polygon,
    /// Machining classification.
    pub class: PassageClass,
    /// Minimum width of the passage in mm.
    pub min_width: f64,
    /// Indices into `polygon` of entry-side edges (edges not collinear
    /// with any pocket / island boundary).
    pub entry_edge_indices: Vec<usize>,
}

/// Options for narrow-passage machining analysis.
#[derive(Clone, Debug)]
pub struct NarrowAnalysisOptions {
    /// Tool radius in mm.
    pub tool_radius: f64,
    /// Additional clearance tolerance in mm.
    pub tolerance: f64,
    /// Minimum passage width for slotting in mm.
    /// When set to `0.0` (default) it defaults to `tool_diameter`.
    pub min_slot_width: f64,
}

impl Default for NarrowAnalysisOptions {
    fn default() -> Self {
        Self {
            tool_radius: 3.0,
            tolerance: 0.5,
            min_slot_width: 0.0,
        }
    }
}

/// Group wall edges by contiguity (separated by entry edges) and compute
/// the minimum distance between edges from different wall groups.
fn compute_min_width(polygon: &Polygon, entry_indices: &[usize]) -> f64 {
    let n = polygon.len();
    if n < 3 || entry_indices.is_empty() {
        return 0.0;
    }

    let entry_set: HashSet<usize> = entry_indices.iter().copied().collect();

    // Start from the first entry edge to avoid wrap-around issues
    let start = *entry_indices.iter().min().unwrap();

    let mut wall_groups: Vec<Vec<(usize, usize)>> = Vec::new();
    let mut current_group: Vec<(usize, usize)> = Vec::new();

    for offset in 0..n {
        let i = (start + offset) % n;
        if entry_set.contains(&i) {
            if !current_group.is_empty() {
                wall_groups.push(std::mem::take(&mut current_group));
            }
        } else {
            current_group.push((i, (i + 1) % n));
        }
    }
    if !current_group.is_empty() {
        wall_groups.push(current_group);
    }

    if wall_groups.len() < 2 {
        return 0.0;
    }

    let mut min_d = f64::MAX;
    for ga in 0..wall_groups.len() {
        for gb in (ga + 1)..wall_groups.len() {
            for &(i, j) in &wall_groups[ga] {
                for &(k, l) in &wall_groups[gb] {
                    let d = get_segment_segment_distance(
                        polygon[i], polygon[j], polygon[k], polygon[l],
                    );
                    if d < min_d {
                        min_d = d;
                    }
                }
            }
        }
    }

    if min_d == f64::MAX {
        0.0
    } else {
        min_d
    }
}

/// Analyze a pocket and return classified narrow regions.
///
/// Calls `find_narrow_passages` with `max_width = 3.0 * tool_radius`
/// (1.5 × tool diameter), then classifies each result based on its
/// minimum passage width.
///
/// Classification uses Clipper2 offset checks: a passage is inset by
/// `tool_radius / 2` and `(tool_diameter + tolerance) / 2`.  Results
/// are sorted widest-first.
pub fn analyze_pocket(
    polygon: &Polygon,
    holes: &[Polygon],
    options: &NarrowAnalysisOptions,
) -> Result<Vec<NarrowRegion>, String> {
    let tool_diameter = 2.0 * options.tool_radius;
    let max_width = 1.5 * tool_diameter;
    let min_slot = if options.min_slot_width > 0.0 {
        options.min_slot_width
    } else {
        tool_diameter
    };
    let narrow_threshold = tool_diameter + options.tolerance;

    let passages = find_narrow_passages(polygon, holes, max_width)?;

    let mut boundaries = Vec::with_capacity(holes.len() + 1);
    boundaries.push(polygon.clone());
    boundaries.extend_from_slice(holes);

    let mut regions = Vec::new();
    for passage in passages {
        if passage.len() < 3 {
            continue;
        }

        // Ridge edges are formed by the convex-hull / clipping step and
        // are NOT on the pocket boundary, while wall edges are exact
        // copies.  A tiny tolerance (0.1 mm) is sufficient to distinguish.
        let entry_edges = find_entry_edges(&passage, &boundaries, 0.1);

        // Classify by checking if the passage survives insetting.
        // Slot is the widest band (D+tol < width < 1.5D).
        // Narrow is the tighter band (min_slot < width ≤ D+tol).
        let survives_wide = !offset_polygon(
            &passage,
            -narrow_threshold / 2.0,
            JoinStyle::Miter,
        )
        .is_empty();
        let survives_tight =
            !offset_polygon(&passage, -min_slot / 2.0, JoinStyle::Miter)
                .is_empty();

        let class = if survives_wide {
            PassageClass::Narrow
        } else if survives_tight {
            PassageClass::Slot
        } else {
            PassageClass::Unreachable
        };

        let min_width = if entry_edges.is_empty() {
            if survives_wide {
                narrow_threshold + 0.01
            } else if survives_tight {
                tool_diameter + 0.01
            } else {
                tool_diameter * 0.5
            }
        } else {
            compute_min_width(&passage, &entry_edges)
        };

        regions.push(NarrowRegion {
            polygon: passage,
            class,
            min_width,
            entry_edge_indices: entry_edges,
        });
    }

    // Sort widest first
    regions.sort_by(|a, b| {
        b.min_width
            .partial_cmp(&a.min_width)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(regions)
}
