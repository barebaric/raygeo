use std::f64::consts::PI;

const MAX_ANGLE_PER_SEGMENT: f64 = PI / 12.0; // 15 degrees

fn mu_to_degrees(mu: f64, diameter: f64) -> f64 {
    if diameter <= 0.0 {
        return 0.0;
    }
    let circumference = diameter * PI;
    (mu / circumference) * 360.0
}

pub fn transform_to_cylinder(
    verts: &[f32],
    diameter: f64,
    colors: Option<&[f32]>,
    degrees_input: bool,
) -> (Vec<f32>, Option<Vec<f32>>, Vec<i32>) {
    if verts.is_empty() || diameter <= 0.0 {
        return (verts.to_vec(), colors.map(|c| c.to_vec()), vec![0]);
    }

    let num_vertices = verts.len() / 3;
    let num_pairs = num_vertices / 2;
    if num_pairs == 0 {
        let empty_verts: Vec<f32> = Vec::new();
        let empty_cols = colors.map(|_| Vec::new());
        return (empty_verts, empty_cols, vec![0]);
    }

    let radius = diameter / 2.0;

    let mut cyl1: Vec<f64> = Vec::with_capacity(num_pairs);
    let mut src1: Vec<f64> = Vec::with_capacity(num_pairs);
    let mut z1: Vec<f64> = Vec::with_capacity(num_pairs);
    let mut cyl2: Vec<f64> = Vec::with_capacity(num_pairs);
    let mut src2: Vec<f64> = Vec::with_capacity(num_pairs);
    let mut z2: Vec<f64> = Vec::with_capacity(num_pairs);

    for i in 0..num_pairs {
        let idx0 = i * 6;
        let idx1 = i * 6 + 3;
        cyl1.push(verts[idx0] as f64);
        src1.push(verts[idx0 + 1] as f64);
        z1.push(verts[idx0 + 2] as f64);
        cyl2.push(verts[idx1] as f64);
        src2.push(verts[idx1 + 1] as f64);
        z2.push(verts[idx1 + 2] as f64);
    }

    let theta1: Vec<f64> = if degrees_input {
        src1.iter().map(|s| s.to_radians()).collect()
    } else {
        src1.iter()
            .map(|&s| mu_to_degrees(s, diameter).to_radians())
            .collect()
    };

    let theta2: Vec<f64> = if degrees_input {
        src2.iter().map(|s| s.to_radians()).collect()
    } else {
        src2.iter()
            .map(|&s| mu_to_degrees(s, diameter).to_radians())
            .collect()
    };

    let mut num_subs: Vec<i32> = Vec::with_capacity(num_pairs);
    let mut total_segments: i32 = 0;

    for i in 0..num_pairs {
        let mut delta_theta = theta2[i] - theta1[i];
        delta_theta = (delta_theta + PI) % (2.0 * PI);
        if delta_theta < 0.0 {
            delta_theta += 2.0 * PI;
        }
        delta_theta -= PI;

        let abs_delta = delta_theta.abs();
        let subs = (abs_delta / MAX_ANGLE_PER_SEGMENT).ceil() as i32;
        let subs = subs.max(1);
        num_subs.push(subs);
        total_segments += subs;
    }

    let total = total_segments as usize;
    let mut result_verts: Vec<f32> = Vec::with_capacity(total * 6);
    let mut result_colors: Option<Vec<f32>> =
        colors.map(|_| Vec::with_capacity(total * 8));

    for pair in 0..num_pairs {
        let subs = num_subs[pair] as usize;
        let c1 = cyl1[pair];
        let s1 = src1[pair];
        let z1v = z1[pair];
        let c2 = cyl2[pair];
        let s2 = src2[pair];
        let z2v = z2[pair];

        let d_cyl = c2 - c1;
        let d_src = s2 - s1;
        let d_z = z2v - z1v;

        for seg in 0..subs {
            let prev_t = seg as f64 / subs as f64;
            let curr_t = (seg + 1) as f64 / subs as f64;

            let prev_cyl = c1 + prev_t * d_cyl;
            let prev_src = s1 + prev_t * d_src;
            let prev_z = z1v + prev_t * d_z;
            let curr_cyl = c1 + curr_t * d_cyl;
            let curr_src = s1 + curr_t * d_src;
            let curr_z = z1v + curr_t * d_z;

            let prev_eff_r = radius + prev_z;
            let curr_eff_r = radius + curr_z;

            let theta_prev = if degrees_input {
                prev_src.to_radians()
            } else {
                mu_to_degrees(prev_src, diameter).to_radians()
            };
            let theta_curr = if degrees_input {
                curr_src.to_radians()
            } else {
                mu_to_degrees(curr_src, diameter).to_radians()
            };

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

    let mut cum_subs: Vec<i32> = Vec::with_capacity(num_pairs + 1);
    cum_subs.push(0);
    let mut running = 0i32;
    for &s in &num_subs {
        running += s;
        cum_subs.push(running);
    }

    (result_verts, result_colors, cum_subs)
}
