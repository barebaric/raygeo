use crate::geo::algo::helix::HelixDirection;
use crate::geo::types::{Point, Point3D};

/// Options controlling flat (constant-Z) spiral generation.
#[derive(Clone, Debug)]
pub struct SpiralOptions {
    pub center: Point,
    pub z: f64,
    pub start_radius: f64,
    pub end_radius: f64,
    /// Total turns; may be fractional (e.g. 0.5 for a half-turn).
    pub revolutions: f64,
    pub direction: HelixDirection,
    /// Angular step in radians per vertex.
    pub angular_step: f64,
    /// Starting angle in radians (0 = +X axis).
    pub start_angle: f64,
}

/// Emit a (poly)line approximating a flat Archimedean spiral at constant
/// Z, sweeping linearly from `start_radius` to `end_radius`.
///
/// Returns an empty vec when `start_radius == end_radius`,
/// `revolutions <= 0`, or `angular_step <= 0`. Hostile input degrades to
/// no spiral or a capped resolution, never to overflow or non-finite
/// points.
pub fn generate_spiral_3d(opts: &SpiralOptions) -> Vec<Point3D> {
    // The finite checks make NaN fail the guards too, which a plain
    // `<= 0.0` comparison would let through.
    if !opts.revolutions.is_finite()
        || opts.revolutions <= 0.0
        || !opts.angular_step.is_finite()
        || opts.angular_step <= 0.0
        || !opts.start_radius.is_finite()
        || !opts.end_radius.is_finite()
        || !opts.center.x.is_finite()
        || !opts.center.y.is_finite()
        || (opts.start_radius - opts.end_radius).abs() < 1e-12
    {
        return vec![];
    }

    let total_angle = opts.revolutions * 2.0 * std::f64::consts::PI;
    let dir_sign = match opts.direction {
        HelixDirection::Cw => -1.0,
        HelixDirection::Ccw => 1.0,
    };
    // An absurdly fine angular step is capped at a fixed resolution
    // instead of asking for an absurd number of points.
    const MAX_STEPS: f64 = 100_000.0;
    let n_steps = (total_angle / opts.angular_step).ceil();
    let n_steps = n_steps.clamp(1.0, MAX_STEPS) as usize;

    let mut points = Vec::with_capacity(n_steps + 1);
    for i in 0..=n_steps {
        let t = i as f64 / n_steps as f64;
        let angle = opts.start_angle + total_angle * t * dir_sign;
        let radius =
            opts.start_radius + (opts.end_radius - opts.start_radius) * t;
        let x = opts.center.x + radius * angle.cos();
        let y = opts.center.y + radius * angle.sin();
        points.push(Point3D::new(x, y, opts.z));
    }
    points
}
