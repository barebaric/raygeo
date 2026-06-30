use std::collections::HashMap;

use prof_macros::prof;

const NUM_GRAY_LEVELS: usize = 256;

fn otsu_threshold(hist: &[u32; NUM_GRAY_LEVELS], total: usize) -> u8 {
    if total == 0 {
        return 128;
    }

    let sum_total: u64 = hist
        .iter()
        .enumerate()
        .map(|(i, &c)| (i as u64) * c as u64)
        .sum();

    let mut sum_b: u64 = 0;
    let mut w_b: u64 = 0;
    let mut max_variance = 0.0f64;
    let mut threshold = 0u8;

    for (t, &count) in hist.iter().enumerate() {
        if count == 0 {
            continue;
        }
        w_b += count as u64;
        if w_b == 0 || w_b == total as u64 {
            continue;
        }
        let w_f = total as u64 - w_b;

        sum_b += (t as u64) * count as u64;
        let mean_b = sum_b as f64 / w_b as f64;
        let mean_f = (sum_total - sum_b) as f64 / w_f as f64;

        let variance = (w_b as f64) * (w_f as f64) * (mean_b - mean_f).powi(2);
        if variance > max_variance {
            max_variance = variance;
            threshold = t as u8;
        }
    }

    threshold
}

pub fn grayscale_to_binary(
    gray: &[u8],
    width: usize,
    height: usize,
    threshold: u8,
    invert: bool,
    auto_threshold: bool,
) -> Vec<u8> {
    let size = width * height;
    assert_eq!(gray.len(), size, "gray buffer size mismatch");

    let thr = if auto_threshold {
        let mut hist = [0u32; NUM_GRAY_LEVELS];
        for &p in gray {
            hist[p as usize] += 1;
        }
        otsu_threshold(&hist, size)
    } else {
        threshold
    };

    let mut output = vec![0u8; size];
    if invert {
        // THRESH_BINARY: src > thr -> foreground
        for (src, dst) in gray.iter().zip(output.iter_mut()) {
            *dst = if *src > thr { 1 } else { 0 };
        }
    } else {
        // THRESH_BINARY_INV: src <= thr -> foreground
        for (src, dst) in gray.iter().zip(output.iter_mut()) {
            *dst = if *src <= thr { 1 } else { 0 };
        }
    }
    output
}

struct DisjointSet {
    parent: Vec<usize>,
}

impl DisjointSet {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn union(&mut self, x: usize, y: usize) {
        let px = self.find(x);
        let py = self.find(y);
        if px != py {
            self.parent[py] = px;
        }
    }
}

fn connected_components_8(
    binary: &[u8],
    width: usize,
    height: usize,
) -> (Vec<u32>, HashMap<u32, u32>) {
    let size = width * height;
    let mut labels = vec![0u32; size];
    let mut ds = DisjointSet::new(size);
    let mut next_label: u32 = 1;

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if binary[idx] == 0 {
                continue;
            }

            let mut neighbor_labels: Vec<u32> = Vec::new();

            if x > 0 && binary[idx - 1] != 0 {
                neighbor_labels.push(labels[idx - 1]);
            }
            if y > 0 && binary[idx - width] != 0 {
                neighbor_labels.push(labels[idx - width]);
            }
            if x > 0 && y > 0 && binary[idx - width - 1] != 0 {
                neighbor_labels.push(labels[idx - width - 1]);
            }
            if x + 1 < width && y > 0 && binary[idx - width + 1] != 0 {
                neighbor_labels.push(labels[idx - width + 1]);
            }

            if neighbor_labels.is_empty() {
                labels[idx] = next_label;
                next_label += 1;
            } else {
                let min_label = *neighbor_labels.iter().min().unwrap();
                labels[idx] = min_label;
                for &nl in &neighbor_labels {
                    if nl != min_label {
                        ds.union(min_label as usize, nl as usize);
                    }
                }
            }
        }
    }

    // Map labels through union-find and count areas
    let mut area_map: HashMap<u32, u32> = HashMap::new();
    let mut label_map: HashMap<u32, u32> = HashMap::new();
    let mut next_compact = 1u32;

    for label in labels.iter_mut() {
        if *label == 0 {
            continue;
        }
        let root = ds.find(*label as usize) as u32;
        let compact = *label_map.entry(root).or_insert_with(|| {
            let c = next_compact;
            next_compact += 1;
            c
        });
        *label = compact;
        *area_map.entry(compact).or_insert(0) += 1;
    }

    (labels, area_map)
}

pub fn get_component_areas(
    binary: &[u8],
    width: usize,
    height: usize,
) -> Vec<u32> {
    let size = width * height;
    assert_eq!(binary.len(), size, "binary buffer size mismatch");

    if binary.iter().all(|&p| p == 0) {
        return Vec::new();
    }

    let (_labels, area_map) = connected_components_8(binary, width, height);

    let mut areas: Vec<u32> = area_map.into_values().collect();
    areas.sort_unstable();
    areas
}

#[prof]
pub fn compute_adaptive_threshold(areas: &[u32]) -> usize {
    if areas.is_empty() {
        return 0;
    }

    // Heuristic: if all unique areas are > 10 and each appears < 10 times,
    // the image is likely already clean — return minimum threshold.
    let max_a = *areas.iter().max().unwrap_or(&0) as usize;
    let mut bin_counts = vec![0u32; max_a + 1];
    for &a in areas {
        bin_counts[a as usize] += 1;
    }

    let mut is_clean = true;
    for (area, &count) in bin_counts.iter().enumerate() {
        if count > 0 && (area <= 10 || count >= 10) {
            is_clean = false;
            break;
        }
    }
    if is_clean {
        return 2;
    }

    // Find all area sizes that are actually present in the image
    let present_areas: Vec<usize> = bin_counts
        .iter()
        .enumerate()
        .filter(|(_, &c)| c > 0)
        .map(|(i, _)| i)
        .collect();

    if present_areas.len() <= 1 {
        return 2;
    }

    // Find the largest gap between consecutive size values
    let mut max_gap = 0usize;
    let mut gap_idx = 0;
    for i in 1..present_areas.len() {
        let gap = present_areas[i] - present_areas[i - 1];
        if gap > max_gap {
            max_gap = gap;
            gap_idx = i - 1;
        }
    }

    let last_noisy = present_areas[gap_idx];
    let threshold = last_noisy + 1;
    let capped = threshold.min(100);
    capped.max(2)
}

pub fn denoise_binary(binary: &[u8], width: usize, height: usize) -> Vec<u8> {
    let size = width * height;
    assert_eq!(binary.len(), size, "binary buffer size mismatch");

    if binary.iter().all(|&p| p == 0) {
        return binary.to_vec();
    }

    let (labels, area_map) = connected_components_8(binary, width, height);

    if area_map.len() <= 1 {
        return binary.to_vec();
    }

    let mut areas: Vec<u32> = area_map.values().copied().collect();
    areas.sort_unstable();

    let min_area = compute_adaptive_threshold(&areas);

    if min_area <= 1 {
        return binary.to_vec();
    }

    let mut output = vec![0u8; size];
    for (i, &label) in labels.iter().enumerate() {
        if label > 0 {
            if let Some(&area) = area_map.get(&label) {
                if (area as usize) >= min_area {
                    output[i] = 1;
                }
            }
        }
    }
    output
}

pub fn filter_components(
    binary: &[u8],
    width: usize,
    height: usize,
    min_area: usize,
) -> Vec<u8> {
    let size = width * height;
    assert_eq!(binary.len(), size, "binary buffer size mismatch");

    if min_area <= 1 || binary.iter().all(|&p| p == 0) {
        return binary.to_vec();
    }

    let (labels, area_map) = connected_components_8(binary, width, height);

    let mut output = vec![0u8; size];
    for (i, &label) in labels.iter().enumerate() {
        if label > 0 {
            if let Some(&area) = area_map.get(&label) {
                if (area as usize) >= min_area {
                    output[i] = 1;
                }
            }
        }
    }
    output
}
