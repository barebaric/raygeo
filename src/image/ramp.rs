//! Edge ramps for relief engraving: how far each burned pixel lies from
//! the edge of the burned area, as a fraction of a ramp length.

/// Squared distances and nearest feature rows along every column.
struct ColumnPass {
    sq_dist: Vec<f64>,
    nearest_row: Vec<usize>,
}

/// For each padded pixel, the nearest unburned pixel in its column.
fn column_pass(
    padded: &[bool],
    width: usize,
    height: usize,
    sy: f64,
) -> ColumnPass {
    let mut sq_dist = vec![0.0; width * height];
    let mut nearest_row = vec![0usize; width * height];
    for x in 0..width {
        let mut last: Option<usize> = None;
        for y in 0..height {
            if !padded[y * width + x] {
                last = Some(y);
            }
            nearest_row[y * width + x] = last.unwrap_or(usize::MAX);
        }
        let mut next: Option<usize> = None;
        for y in (0..height).rev() {
            if !padded[y * width + x] {
                next = Some(y);
            }
            let idx = y * width + x;
            let above = nearest_row[idx];
            let pick = match (above, next) {
                (usize::MAX, Some(n)) => n,
                (a, Some(n)) if n - y < y - a => n,
                (a, _) => a,
            };
            nearest_row[idx] = pick;
            let dy = (y as f64 - pick as f64) * sy;
            sq_dist[idx] = dy * dy;
        }
    }
    ColumnPass {
        sq_dist,
        nearest_row,
    }
}

/// Felzenszwalb-Huttenlocher lower envelope: for every position, the
/// index whose parabola `f[i] + ((pos - i) * s)^2` is lowest.
fn nearest_along_row(f: &[f64], s: f64) -> Vec<usize> {
    let n = f.len();
    let scaled: Vec<f64> = f.iter().map(|v| v / (s * s)).collect();
    let mut v = vec![0usize; n];
    let mut z = vec![0.0f64; n + 1];
    let mut k = 0usize;
    z[0] = f64::NEG_INFINITY;
    z[1] = f64::INFINITY;
    let intersect = |q: usize, p: usize| -> f64 {
        let (qf, pf) = (q as f64, p as f64);
        ((scaled[q] + qf * qf) - (scaled[p] + pf * pf)) / (2.0 * (qf - pf))
    };
    for q in 1..n {
        let mut s_int = intersect(q, v[k]);
        while s_int <= z[k] {
            k -= 1;
            s_int = intersect(q, v[k]);
        }
        k += 1;
        v[k] = q;
        z[k] = s_int;
        z[k + 1] = f64::INFINITY;
    }
    let mut out = vec![0usize; n];
    k = 0;
    for (q, slot) in out.iter_mut().enumerate() {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        *slot = v[k];
    }
    out
}

/// Distance from a pixel centre to the edge of the unburned pixel whose
/// centre lies `(dx, dy)` mm away, measured along that direction.
fn distance_to_edge(dx: f64, dy: f64, sx: f64, sy: f64) -> f64 {
    let length = dx.hypot(dy);
    if length == 0.0 {
        return 0.0;
    }
    let reach = |d: f64, half: f64| {
        if d == 0.0 {
            f64::INFINITY
        } else {
            half / d.abs()
        }
    };
    let inside = reach(dx, sx / 2.0).min(reach(dy, sy / 2.0)).min(1.0);
    length * (1.0 - inside)
}

/// Writes, for every burned pixel (nonzero in `binary`), its distance to
/// the edge of the burned area divided by `ramp_length_mm` and clamped
/// to 1; unburned pixels get 0. Pixels outside the image count as
/// unburned. The distance runs from the pixel centre to the nearest
/// unburned pixel centre, less the part of that line inside the
/// unburned pixel; along the axes this is exact. A ramp length of 0
/// writes 1 for every burned pixel.
pub fn edge_ramp(
    binary: &[u8],
    width: usize,
    height: usize,
    ramp_length_mm: f64,
    pixels_per_mm_x: f64,
    pixels_per_mm_y: f64,
    output: &mut [f32],
) {
    if ramp_length_mm <= 0.0 {
        for (out, &b) in output.iter_mut().zip(binary) {
            *out = if b != 0 { 1.0 } else { 0.0 };
        }
        return;
    }
    let (sx, sy) = (1.0 / pixels_per_mm_x, 1.0 / pixels_per_mm_y);
    let (pw, ph) = (width + 2, height + 2);
    let mut padded = vec![false; pw * ph];
    for y in 0..height {
        for x in 0..width {
            padded[(y + 1) * pw + x + 1] = binary[y * width + x] != 0;
        }
    }
    let columns = column_pass(&padded, pw, ph, sy);
    for y in 0..height {
        let row = (y + 1) * pw;
        let nearest = nearest_along_row(&columns.sq_dist[row..row + pw], sx);
        for x in 0..width {
            let idx = y * width + x;
            if binary[idx] == 0 {
                output[idx] = 0.0;
                continue;
            }
            let fx = nearest[x + 1];
            let fy = columns.nearest_row[row + fx];
            let dx = (x as f64 + 1.0 - fx as f64) * sx;
            let dy = (y as f64 + 1.0 - fy as f64) * sy;
            let distance = distance_to_edge(dx, dy, sx, sy);
            output[idx] = (distance / ramp_length_mm).min(1.0) as f32;
        }
    }
}
