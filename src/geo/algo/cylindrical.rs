use std::f64::consts::PI;

/// Maximum sagitta (mm) of a linearized chord relative to its home
/// circle.  Cylinder-mapped line segments are subdivided so no chord
/// deviates more than this from the circle, regardless of diameter.
///
/// The same value doubles as the outward radial lift applied to every
/// cylinder-mapped polyline (toolpath lines and overlay rings): with
/// the home circle at ``radius + MAX_SAGITTA``, each chord's deepest
/// point sits at ``radius + MAX_SAGITTA - sagitta >= radius``, i.e.
/// on or above the true cylinder surface.  The stock shell and the
/// engrave quads are inscribed in that true circle, so without the
/// lift the chord midpoints periodically sink below the surface
/// facets; orthographic views resolve the sub-millimetre gap and hide
/// those line stretches, while perspective views round it away.
pub const MAX_SAGITTA: f64 = 0.05;

fn mu_to_degrees(mu: f64, diameter: f64) -> f64 {
    if diameter <= 0.0 {
        return 0.0;
    }
    let circumference = diameter * PI;
    (mu / circumference) * 360.0
}

fn src_to_radians(src: f64, diameter: f64, degrees_input: bool) -> f64 {
    if degrees_input {
        src.to_radians()
    } else {
        mu_to_degrees(src, diameter).to_radians()
    }
}

struct DeinterleavedPairs {
    cyl1: Vec<f64>,
    src1: Vec<f64>,
    z1: Vec<f64>,
    cyl2: Vec<f64>,
    src2: Vec<f64>,
    z2: Vec<f64>,
}

fn deinterleave_pairs(verts: &[f32], num_pairs: usize) -> DeinterleavedPairs {
    let mut p = DeinterleavedPairs {
        cyl1: Vec::with_capacity(num_pairs),
        src1: Vec::with_capacity(num_pairs),
        z1: Vec::with_capacity(num_pairs),
        cyl2: Vec::with_capacity(num_pairs),
        src2: Vec::with_capacity(num_pairs),
        z2: Vec::with_capacity(num_pairs),
    };
    for i in 0..num_pairs {
        let idx0 = i * 6;
        let idx1 = i * 6 + 3;
        p.cyl1.push(verts[idx0] as f64);
        p.src1.push(verts[idx0 + 1] as f64);
        p.z1.push(verts[idx0 + 2] as f64);
        p.cyl2.push(verts[idx1] as f64);
        p.src2.push(verts[idx1 + 1] as f64);
        p.z2.push(verts[idx1 + 2] as f64);
    }
    p
}

fn compute_subdivisions(
    theta1: &[f64],
    theta2: &[f64],
    radius: f64,
) -> (Vec<i32>, i32) {
    let max_angle = if MAX_SAGITTA < radius {
        2.0 * (1.0 - MAX_SAGITTA / radius).acos()
    } else {
        PI / 12.0
    };

    let mut num_subs = Vec::with_capacity(theta1.len());
    let mut total_segments = 0i32;

    for i in 0..theta1.len() {
        let delta_theta = theta2[i] - theta1[i];

        let subs = (delta_theta.abs() / max_angle).ceil() as i32;
        let subs = subs.max(1);
        num_subs.push(subs);
        total_segments += subs;
    }

    (num_subs, total_segments)
}

#[allow(clippy::too_many_arguments)]
fn subdivide_and_map(
    pairs: &DeinterleavedPairs,
    num_subs: &[i32],
    total_segments: i32,
    radius: f64,
    diameter: f64,
    degrees_input: bool,
    colors: Option<&[f32]>,
    radial_offset: f64,
) -> (Vec<f32>, Option<Vec<f32>>) {
    let total = total_segments as usize;
    let mut result_verts: Vec<f32> = Vec::with_capacity(total * 6);
    let mut result_colors: Option<Vec<f32>> =
        colors.map(|_| Vec::with_capacity(total * 8));

    let full_src = if degrees_input { 360.0 } else { PI * diameter };

    for (pair, &subs_i32) in num_subs.iter().enumerate() {
        let subs = subs_i32 as usize;
        let d_cyl = pairs.cyl2[pair] - pairs.cyl1[pair];
        let d_z = pairs.z2[pair] - pairs.z1[pair];

        let src1 = pairs.src1[pair];
        let src2 = pairs.src2[pair];
        let theta1 = src_to_radians(src1, diameter, degrees_input);
        let theta2 = src_to_radians(src2, diameter, degrees_input);

        let mut d_src = src2 - src1;
        let d_theta = theta2 - theta1;

        if d_theta >= 2.0 * PI {
            d_src -= full_src;
        } else if d_theta <= -2.0 * PI {
            d_src += full_src;
        }

        for seg in 0..subs {
            let prev_t = seg as f64 / subs as f64;
            let curr_t = (seg + 1) as f64 / subs as f64;

            let prev_cyl = pairs.cyl1[pair] + prev_t * d_cyl;
            let prev_src = src1 + prev_t * d_src;
            let prev_z = pairs.z1[pair] + prev_t * d_z;
            let curr_cyl = pairs.cyl1[pair] + curr_t * d_cyl;
            let curr_src = src1 + curr_t * d_src;
            let curr_z = pairs.z1[pair] + curr_t * d_z;

            let prev_eff_r = radius + prev_z + radial_offset;
            let curr_eff_r = radius + curr_z + radial_offset;

            let theta_prev = src_to_radians(prev_src, diameter, degrees_input);
            let theta_curr = src_to_radians(curr_src, diameter, degrees_input);

            result_verts.push(prev_cyl as f32);
            result_verts.push((prev_eff_r * theta_prev.sin()) as f32);
            result_verts.push((prev_eff_r * theta_prev.cos()) as f32);
            result_verts.push(curr_cyl as f32);
            result_verts.push((curr_eff_r * theta_curr.sin()) as f32);
            result_verts.push((curr_eff_r * theta_curr.cos()) as f32);

            if let Some(ref mut cols) = result_colors {
                if let Some(c) = colors {
                    let ci0 = pair * 8;
                    for j in 0..4 {
                        cols.push(c[ci0 + j]);
                    }
                    for j in 0..4 {
                        cols.push(c[ci0 + 4 + j]);
                    }
                }
            }
        }
    }

    (result_verts, result_colors)
}

fn cumulative_offsets(num_subs: &[i32]) -> Vec<i32> {
    let mut cum = Vec::with_capacity(num_subs.len() + 1);
    cum.push(0);
    let mut running = 0i32;
    for &s in num_subs {
        running += s;
        cum.push(running);
    }
    cum
}

pub fn transform_to_cylinder(
    verts: &[f32],
    diameter: f64,
    colors: Option<&[f32]>,
    degrees_input: bool,
    radial_offset: f64,
) -> (Vec<f32>, Option<Vec<f32>>, Vec<i32>) {
    if verts.is_empty() || diameter <= 0.0 {
        return (verts.to_vec(), colors.map(|c| c.to_vec()), vec![0]);
    }

    let num_vertices = verts.len() / 3;
    let num_pairs = num_vertices / 2;
    if num_pairs == 0 {
        return (Vec::new(), colors.map(|_| Vec::new()), vec![0]);
    }

    let radius = diameter / 2.0;
    let pairs = deinterleave_pairs(verts, num_pairs);

    let theta1: Vec<f64> = pairs
        .src1
        .iter()
        .map(|&s| src_to_radians(s, diameter, degrees_input))
        .collect();
    let theta2: Vec<f64> = pairs
        .src2
        .iter()
        .map(|&s| src_to_radians(s, diameter, degrees_input))
        .collect();

    let (num_subs, total_segments) =
        compute_subdivisions(&theta1, &theta2, radius);

    let (result_verts, result_colors) = subdivide_and_map(
        &pairs,
        &num_subs,
        total_segments,
        radius,
        diameter,
        degrees_input,
        colors,
        radial_offset,
    );

    let cum_subs = cumulative_offsets(&num_subs);

    (result_verts, result_colors, cum_subs)
}
