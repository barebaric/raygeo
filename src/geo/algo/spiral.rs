use crate::geo::algo::helix::HelixDirection;
use crate::types::{Point, Point3D};

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
/// `revolutions <= 0`, or `angular_step <= 0`.
pub fn generate_spiral_3d(opts: &SpiralOptions) -> Vec<Point3D> {
    if (opts.start_radius - opts.end_radius).abs() < 1e-12
        || opts.revolutions <= 0.0
        || opts.angular_step <= 0.0
    {
        return vec![];
    }

    let total_angle = opts.revolutions * 2.0 * std::f64::consts::PI;
    let dir_sign = match opts.direction {
        HelixDirection::Cw => -1.0,
        HelixDirection::Ccw => 1.0,
    };
    let n_steps = (total_angle / opts.angular_step).ceil() as usize;
    let n_steps = n_steps.max(1);

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
