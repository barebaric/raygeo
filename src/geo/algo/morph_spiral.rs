use crate::geo::algo::medial_axis::{compute_medial_axis, MedialAxis};
use crate::geo::shape::polygon::{offset_polygon_with_style, JoinStyle};
use crate::types::{Point, Point3D};

/// Options for the full morph-spiral pipeline.
#[derive(Clone, Debug)]
pub struct MorphSpiralOptions<'a> {
    pub pocket_boundary: &'a [Point],
    pub islands: &'a [Vec<Point>],
    pub tool_radius: f64,
    pub step_over: f64,
    pub z: f64,
    /// Sampling spacing for the MAT (defaults to `step_over × 0.5`).
    pub sampling_spacing: Option<f64>,
}

/// Result of the full morph-spiral pipeline.
#[derive(Clone, Debug)]
pub struct MorphSpiralResult {
    pub toolpath: Vec<Point3D>,
    pub branches: Vec<Vec<Point3D>>,
    pub medial_axis: MedialAxis,
}

/// Compute a single continuous morphing spiral for the entire pocket.
///
/// This is the high-level entry point: offsets the boundaries by tool_radius,
/// computes the MAT, generates per-branch boustrophedon spirals, and links
/// them into one toolpath.
pub fn morph_spiral(
    opts: &MorphSpiralOptions,
) -> Result<MorphSpiralResult, String> {
    let sampling_spacing =
        opts.sampling_spacing.unwrap_or(opts.step_over * 0.5);

    let boundary_vec = opts.pocket_boundary.to_vec();
    let valid_outer = offset_polygon_with_style(
        &boundary_vec,
        -opts.tool_radius,
        JoinStyle::Miter,
    );
    if valid_outer.is_empty() {
        return Err("pocket is too narrow for tool radius".into());
    }
    let outer = valid_outer.into_iter().next().unwrap();

    let island_bufs: Vec<Vec<Point>> = opts
        .islands
        .iter()
        .flat_map(|isl| {
            offset_polygon_with_style(isl, opts.tool_radius, JoinStyle::Miter)
        })
        .collect();

    let ma = compute_medial_axis(
        &outer,
        &island_bufs,
        opts.tool_radius * 0.5,
        sampling_spacing,
    )?;

    let mut all_path: Vec<Point3D> = Vec::new();
    let mut branch_paths: Vec<Vec<Point3D>> = Vec::new();

    for branch in &ma.branches {
        let bp = morph_spiral_from_branch(
            &branch.points,
            &branch.clearances,
            opts.step_over,
            opts.z,
        );
        if bp.is_empty() {
            continue;
        }
        if !all_path.is_empty() {
            let last = *all_path.last().unwrap();
            let first = bp[0];
            let dx = first.x - last.x;
            let dy = first.y - last.y;
            if dx * dx + dy * dy > 1e-8 {
                let n_steps = ((dx * dx + dy * dy).sqrt() / opts.step_over
                    * 4.0)
                    .ceil() as usize;
                for i in 1..=n_steps {
                    let t = i as f64 / n_steps as f64;
                    all_path.push(Point3D::new(
                        last.x + t * dx,
                        last.y + t * dy,
                        opts.z,
                    ));
                }
            }
        }
        all_path.extend(bp.iter());
        branch_paths.push(bp);
    }

    Ok(MorphSpiralResult {
        toolpath: all_path,
        branches: branch_paths,
        medial_axis: ma,
    })
}

/// Generate a boustrophedon spiral for a single MAT branch (variable-width
/// channel).
///
/// `points` is the centerline polyline from root (high clearance) to leaf
/// (low clearance).  `clearances[i]` is the half-width of the channel at
/// `points[i]`.
pub fn morph_spiral_from_branch(
    points: &[Point],
    clearances: &[f64],
    step_over: f64,
    z: f64,
) -> Vec<Point3D> {
    if points.len() < 2 || step_over <= 0.0 {
        return vec![];
    }

    let n = points.len();
    let normals = compute_normals(points);
    let mut max_clearance = 0.0f64;
    for &c in clearances {
        if c > max_clearance {
            max_clearance = c;
        }
    }
    if max_clearance < 1e-12 {
        return vec![];
    }

    let max_level = (max_clearance / step_over).ceil() as usize;
    let mut path: Vec<Point3D> = Vec::new();
    let mut prev_end: Option<Point3D> = None;

    for level in 0..=max_level {
        let offset = level as f64 * step_over;
        if offset > max_clearance + 1e-9 {
            break;
        }

        let side_sign: f64 = if level == 0 {
            0.0
        } else if level % 2 == 1 {
            1.0
        } else {
            -1.0
        };

        let k = level.div_ceil(2);
        let actual = k as f64 * step_over * side_sign;

        let forward = level % 2 == 0;

        let valid_indices: Vec<usize> = if level == 0 {
            (0..n).collect()
        } else {
            (0..n)
                .filter(|&i| clearances[i] >= actual.abs() - 1e-9)
                .collect()
        };

        if valid_indices.is_empty() {
            continue;
        }

        let pass_indices: Vec<usize> = if forward {
            valid_indices
        } else {
            valid_indices.into_iter().rev().collect()
        };

        let mut pass_pts: Vec<Point3D> = pass_indices
            .iter()
            .map(|&i| {
                Point3D::new(
                    points[i].x + actual * normals[i].x,
                    points[i].y + actual * normals[i].y,
                    z,
                )
            })
            .collect();

        if pass_pts.len() < 2 {
            continue;
        }

        deduplicate_polyline(&mut pass_pts);

        if let Some(pe) = prev_end {
            let d2 =
                (pe.x - pass_pts[0].x).powi(2) + (pe.y - pass_pts[0].y).powi(2);
            if d2 > 1e-8 {
                let segs = ((d2.sqrt() / step_over) * 4.0).ceil() as usize;
                for s in 1..=segs {
                    let t = s as f64 / segs as f64;
                    path.push(Point3D::new(
                        pe.x + t * (pass_pts[0].x - pe.x),
                        pe.y + t * (pass_pts[0].y - pe.y),
                        z,
                    ));
                }
            }
        }

        path.extend(pass_pts);
        prev_end = Some(*path.last().unwrap());
    }

    path
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn compute_normals(points: &[Point]) -> Vec<Point> {
    let n = points.len();
    let mut normals = Vec::with_capacity(n);
    for i in 0..n {
        let tangent = if i == 0 {
            Point::new(points[1].x - points[0].x, points[1].y - points[0].y)
        } else if i == n - 1 {
            Point::new(
                points[n - 1].x - points[n - 2].x,
                points[n - 1].y - points[n - 2].y,
            )
        } else {
            Point::new(
                points[i + 1].x - points[i - 1].x,
                points[i + 1].y - points[i - 1].y,
            )
        };
        let len = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
        if len < 1e-12 {
            normals.push(Point::new(0.0, 1.0));
        } else {
            normals.push(Point::new(-tangent.y / len, tangent.x / len));
        }
    }
    normals
}

fn deduplicate_polyline(pts: &mut Vec<Point3D>) {
    if pts.is_empty() {
        return;
    }
    let mut j = 1;
    for i in 1..pts.len() {
        let dx = pts[i].x - pts[j - 1].x;
        let dy = pts[i].y - pts[j - 1].y;
        if dx * dx + dy * dy > 1e-12 {
            pts[j] = pts[i];
            j += 1;
        }
    }
    pts.truncate(j);
}
