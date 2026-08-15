//! PBR shading support: BRDF integration for image-based lighting.

use std::f64::consts::PI;

/// Van der Corput radical inverse (base 2) of a 32-bit integer.
fn radical_inverse_vdc(mut bits: u32) -> f64 {
    bits = bits.rotate_right(16);
    bits = ((bits & 0x55555555) << 1) | ((bits >> 1) & 0x55555555);
    bits = ((bits & 0x33333333) << 2) | ((bits >> 2) & 0x33333333);
    bits = ((bits & 0x0F0F0F0F) << 4) | ((bits >> 4) & 0x0F0F0F0F);
    bits = ((bits & 0x00FF00FF) << 8) | ((bits >> 8) & 0x00FF00FF);
    (bits as f64) / 4294967296.0
}

/// Schlick-style geometry term used with the IBL remapping of k.
fn schlick_ggx(nd: f64, k: f64) -> f64 {
    nd / (nd * (1.0 - k) + k)
}

/// Integrate the Cook-Torrance BRDF into a split-sum LUT.
///
/// For each `(NdotV, roughness)` texel the GGX distribution is
/// importance-sampled with a Hammersley sequence and the Smith
/// geometry term integrated, giving the Fresnel scale/bias pair such
/// that the specular IBL response is `F0 * scale + bias`.
///
/// Roughness samples are biased away from zero (`(j + 0.5) / size`)
/// to keep the peak of the GGX distribution numerically stable.
///
/// Returns a flat row-major array of `size * size * 2` float32
/// values, laid out as `lut[roughness][NdotV] = (scale, bias)`.
pub fn generate_brdf_lut(size: usize, sample_count: usize) -> Vec<f32> {
    // Precompute the Hammersley sequence once: it is independent of
    // the texel.
    let xi: Vec<(f64, f64)> = (0..sample_count)
        .map(|i| {
            (
                i as f64 / sample_count as f64,
                radical_inverse_vdc(i as u32),
            )
        })
        .collect();

    let mut lut = vec![0.0f32; size * size * 2];
    for j in 0..size {
        let roughness = (j as f64 + 0.5) / size as f64;
        let a = roughness * roughness;
        let a2 = a * a;
        let r = roughness + 1.0;
        let k = (r * r) / 8.0;

        for i in 0..size {
            let ndv = (i as f64 + 0.5) / size as f64;
            let vx = (1.0 - ndv * ndv).max(0.0).sqrt();
            let vz = ndv;

            let mut scale = 0.0f64;
            let mut bias = 0.0f64;
            for &(xi1, xi2) in &xi {
                let phi = 2.0 * PI * xi1;
                let cos_phi = phi.cos();

                let cos_theta = ((1.0 - xi2) / (1.0 + (a2 - 1.0) * xi2)).sqrt();
                let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
                // Half vector in tangent space with N == (0, 0, 1).
                let hx = sin_theta * cos_phi;
                let hz = cos_theta;

                let voh = (hx * vx + hz * vz).max(0.0);
                let noh = hz;
                // L = reflect(-V, H); L.z is NdotL.
                let lz = 2.0 * voh * hz - vz;

                if lz <= 0.0 || noh <= 0.0 {
                    continue;
                }
                let g = schlick_ggx(ndv, k) * schlick_ggx(lz, k);
                let g_vis = g * voh / (noh * ndv).max(1e-7);
                let fc = (1.0 - voh).powi(5);
                scale += (1.0 - fc) * g_vis;
                bias += fc * g_vis;
            }
            let norm = sample_count as f64;
            lut[(j * size + i) * 2] = (scale / norm) as f32;
            lut[(j * size + i) * 2 + 1] = (bias / norm) as f32;
        }
    }
    lut
}
