use crate::types::{Point, Point3D};

/// Style of ramp entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RampStyle {
    /// Single straight-line diagonal descent.
    Linear,
    /// Linear descent with lateral oscillation.
    ZigZag,
}

/// Options for generating a ramp entry path.
#[derive(Clone, Debug)]
pub struct RampOptions {
    pub start: Point,
    pub end: Point,
    pub z_start: f64,
    pub z_end: f64,
    pub max_ramp_angle_deg: f64,
    pub style: RampStyle,
    pub lateral_amplitude: f64,
}

const RAMP_POINT_SPACING: f64 = 0.1;

/// Generate a ramp polyline from `start` to `end` while descending from
/// `z_start` to `z_end`.
///
/// If the angle of the direct ramp exceeds `max_ramp_angle_deg`, the ramp is
/// extended in both directions (along the same line) so the descent satisfies
/// the angle constraint.
pub fn generate_ramp_3d(opts: &RampOptions) -> Vec<Point3D> {
    let xy_dist = opts.start.distance(opts.end);
    let z_drop = opts.z_start - opts.z_end;

    if xy_dist < 1e-12 || z_drop <= 0.0 {
        return vec![];
    }

    let max_angle_rad = opts.max_ramp_angle_deg.to_radians().max(1e-9);
    let actual_angle = (z_drop / xy_dist).atan();

    // If the ramp is too steep, extend it along the same direction.
    // Cap the extension to prevent runaway allocation.
    const MAX_EXTENSION_FACTOR: f64 = 100.0;
    let (ext_start, ext_end, ext_xy_dist) = if actual_angle > max_angle_rad {
        let needed_xy = z_drop / max_angle_rad.tan();
        let capped_xy = needed_xy.min(xy_dist * MAX_EXTENSION_FACTOR);
        let extra = capped_xy - xy_dist;
        let dir = (opts.end - opts.start).normalize();
        let es = opts.start - dir * (extra / 2.0);
        let ee = opts.end + dir * (extra / 2.0);
        (es, ee, capped_xy)
    } else {
        (opts.start, opts.end, xy_dist)
    };

    let n_points = (ext_xy_dist / RAMP_POINT_SPACING).ceil() as usize;
    let n_points = n_points.clamp(1, 1_000_000);

    match opts.style {
        RampStyle::Linear => {
            let mut points = Vec::with_capacity(n_points + 1);
            for i in 0..=n_points {
                let t = i as f64 / n_points as f64;
                let p = ext_start.lerp(ext_end, t);
                let x = p.x;
                let y = p.y;
                let z = opts.z_start - z_drop * t;
                points.push(Point3D::new(x, y, z));
            }
            points
        }
        RampStyle::ZigZag => {
            let dir = (ext_end - ext_start).normalize();
            let norm = Point::new(-dir.y, dir.x);
            let amp = opts.lateral_amplitude.max(0.0);

            let mut points = Vec::with_capacity(n_points + 1);
            for i in 0..=n_points {
                let t = i as f64 / n_points as f64;
                let s = ext_xy_dist * t; // distance along ramp
                                         // One full lateral oscillation per period = ext_xy_dist / 4
                let period = (ext_xy_dist / 4.0).max(1e-12);
                let lateral_offset =
                    amp * (2.0 * std::f64::consts::PI * s / period).sin();
                let base = ext_start.lerp(ext_end, t);
                let x = base.x + lateral_offset * norm.x;
                let y = base.y + lateral_offset * norm.y;
                let z = opts.z_start - z_drop * t;
                points.push(Point3D::new(x, y, z));
            }
            points
        }
    }
}
