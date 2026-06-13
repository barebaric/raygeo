pub fn compute_auto_levels(gray_image: &[u8], clip_percent: f32) -> (u8, u8) {
    if gray_image.is_empty() {
        return (0, 255);
    }

    let mut histogram = [0usize; 256];
    for &v in gray_image {
        histogram[v as usize] += 1;
    }

    let total = gray_image.len() as f32;
    let lower_count = (total * clip_percent / 100.0).ceil() as usize;
    let upper_count = (total * (100.0 - clip_percent) / 100.0).floor() as usize;

    let mut black_point: u8 = 0;
    let mut cumulative = 0usize;
    for (i, &count) in histogram.iter().enumerate() {
        cumulative += count;
        if cumulative >= lower_count {
            black_point = i as u8;
            break;
        }
    }

    let mut white_point: u8 = 255;
    cumulative = 0;
    for (i, &count) in histogram.iter().enumerate().rev() {
        cumulative += count;
        if cumulative >= (total as usize - upper_count) {
            white_point = i as u8;
            break;
        }
    }

    let black_point = black_point.min(253);
    let white_point = white_point.max(black_point + 2);

    (black_point, white_point)
}

pub fn normalize_grayscale(
    gray_image: &[u8],
    black_point: u8,
    white_point: u8,
    output: &mut [u8],
) {
    assert!(
        black_point < white_point,
        "black_point must be less than white_point"
    );
    let len = gray_image.len().min(output.len());
    let bp = black_point as f32;
    let wp = white_point as f32;
    let range = wp - bp;
    for i in 0..len {
        let v = gray_image[i] as f32;
        let clamped = v.clamp(bp, wp);
        let normalized = (clamped - bp) / range * 255.0;
        output[i] = normalized.round() as u8;
    }
}
