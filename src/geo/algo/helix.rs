use crate::types::{Point, Point3D};

/// Direction of revolution of a generated helix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelixDirection {
    Cw,
    Ccw,
}

/// Options controlling helical (or conical helical) generation.
#[derive(Clone, Debug)]
pub struct HelixOptions {
    pub center: Point,
    pub start_radius: f64,
    pub end_radius: f64,
    pub z_start: f64,
    pub z_end: f64,
    pub pitch: f64,
    pub direction: HelixDirection,
    pub angular_step: f64,
    pub min_revolutions: Option<u32>,
}

/// Emit a (poly)line approximating the requested helix as 3D points.
///
/// Radius is interpolated linearly between start and end (conical helix when
/// they differ). Returns an empty vec when `z_start <= z_end` (no descent).
pub fn generate_helix_3d(opts: &HelixOptions) -> Vec<Point3D> {
    let z_descent = opts.z_start - opts.z_end;
    if z_descent <= 0.0 || opts.pitch <= 0.0 || opts.angular_step <= 0.0 {
        return vec![];
    }

    let mut revolutions = z_descent / opts.pitch;
    if let Some(min) = opts.min_revolutions {
        if revolutions < min as f64 {
            revolutions = min as f64;
        }
    }

    let total_angle = revolutions * 2.0 * std::f64::consts::PI;
    let dir_sign = match opts.direction {
        HelixDirection::Cw => -1.0,
        HelixDirection::Ccw => 1.0,
    };

    let n_steps = (total_angle / opts.angular_step).ceil() as usize;
    let n_steps = n_steps.max(1);

    let mut points = Vec::with_capacity(n_steps + 1);
    for i in 0..=n_steps {
        let t = i as f64 / n_steps as f64;
        let angle = total_angle * t * dir_sign;
        let radius =
            opts.start_radius + (opts.end_radius - opts.start_radius) * t;
        let z = opts.z_start - z_descent * t;
        let x = opts.center.x + radius * angle.cos();
        let y = opts.center.y + radius * angle.sin();
        points.push(Point3D::new(x, y, z));
    }
    points
}
