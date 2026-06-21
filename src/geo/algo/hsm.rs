//! HSM cutting-arc extraction.
//!
//! Pure geometric helpers for adaptive clearing: cutting-arc extraction.
//! Fillet operations (create, append, trim-to-safe) live in
//! [`super::fillet`]; this module only contains the bite-to-arc logic.

use crate::geo::shape::polygon::{
    get_polygon_closest_point, trim_polyline_angular_ends,
};
use crate::types::{Point, Polygon};

/// Extract the cutting-arc (outer) vertices from a bite polygon.
///
/// The bite is a crescent between the cleared frontier and the expanded
/// boundary.  The cutting arc is the longest contiguous run of bite
/// vertices that lie *outside* all cleared fragments.
///
/// Returns `(arc_vertices, cut_start, cut_len)` where `arc_vertices` is
/// the contiguous slice of `bite` forming the outer arc, `cut_start`
/// is the index into `bite`, and `cut_len` is the number of vertices
/// in the arc.  Returns `None` when the bite is degenerate (no outer
/// arc found).
pub fn find_cutting_arc(
    bite: &Polygon,
    cleared_fragments: &[Polygon],
) -> Option<(Vec<Point>, usize, usize)> {
    let n = bite.len();
    if n < 3 {
        return None;
    }

    let is_outer: Vec<bool> = bite
        .iter()
        .map(|p| {
            !cleared_fragments.iter().any(|frag| {
                let d2 = get_polygon_closest_point(frag, p.x, p.y)
                    .map(|(_, _, d2)| d2)
                    .unwrap_or(f64::MAX);
                d2 < 1e-2
            })
        })
        .collect();

    let extended: Vec<bool> = is_outer
        .iter()
        .copied()
        .chain(is_outer.iter().copied())
        .collect();

    let mut cut_start = 0usize;
    let mut cut_len = 0usize;
    {
        let mut cs: Option<usize> = None;
        let mut cl = 0usize;
        for (i, &val) in extended.iter().enumerate() {
            if val {
                if cs.is_none() {
                    cs = Some(i);
                    cl = 1;
                } else {
                    cl += 1;
                }
                if cl > cut_len {
                    cut_start = cs.unwrap();
                    cut_len = cl;
                }
            } else {
                cs = None;
                cl = 0;
            }
        }
    }

    if cut_len < 3 {
        return None;
    }

    // Trim transition vertices at the tips where the outer arc meets
    // the inner arc by detecting sharp jumps in interior angle.
    const DERIV_THRESHOLD: f64 = 0.436_332_312_998_582_4; // 25° in radians
    (cut_start, cut_len) =
        trim_polyline_angular_ends(bite, cut_start, cut_len, DERIV_THRESHOLD);

    if cut_len < 3 {
        return None;
    }

    let vertices: Vec<Point> =
        (0..cut_len).map(|i| bite[(cut_start + i) % n]).collect();
    Some((vertices, cut_start, cut_len))
}
