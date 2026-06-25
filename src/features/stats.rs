/// Statistical helper functions for feature extraction.

/// Returns (mean, std_dev) of a slice. Returns (0, 0) on empty input.
pub fn mean_std(values: &[f32]) -> (f32, f32) {
    if values.is_empty() {
        return (0.0, 0.0);
    }
    let mean = values.iter().sum::<f32>() / values.len() as f32;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / values.len() as f32;
    (mean, variance.sqrt())
}

/// Returns the value at the given percentile (0–100) using linear interpolation.
pub fn percentile(sorted: &[f32], p: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let idx = (p / 100.0) * (sorted.len() - 1) as f32;
    let lo = idx.floor() as usize;
    let hi = (lo + 1).min(sorted.len() - 1);
    let frac = idx - lo as f32;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}

/// Sorts a slice in place and returns the median.
pub fn median(values: &mut Vec<f32>) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    percentile(values, 50.0)
}
