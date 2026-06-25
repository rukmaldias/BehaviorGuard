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

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn mean_std_known_values() {
        let (m, s) = mean_std(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
        assert_abs_diff_eq!(m, 5.0, epsilon = 1e-4);
        assert_abs_diff_eq!(s, 2.0, epsilon = 1e-4);
    }

    #[test]
    fn mean_std_single_element() {
        let (m, s) = mean_std(&[42.0]);
        assert_abs_diff_eq!(m, 42.0, epsilon = 1e-6);
        assert_abs_diff_eq!(s, 0.0, epsilon = 1e-6);
    }

    #[test]
    fn mean_std_empty() {
        assert_eq!(mean_std(&[]), (0.0, 0.0));
    }

    #[test]
    fn mean_std_identical_values() {
        let (m, s) = mean_std(&[3.0, 3.0, 3.0, 3.0]);
        assert_abs_diff_eq!(m, 3.0, epsilon = 1e-6);
        assert_abs_diff_eq!(s, 0.0, epsilon = 1e-6);
    }

    #[test]
    fn percentile_median_of_odd_list() {
        let sorted = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_abs_diff_eq!(percentile(&sorted, 50.0), 3.0, epsilon = 1e-6);
    }

    #[test]
    fn percentile_min_max() {
        let sorted = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_abs_diff_eq!(percentile(&sorted, 0.0), 1.0, epsilon = 1e-6);
        assert_abs_diff_eq!(percentile(&sorted, 100.0), 5.0, epsilon = 1e-6);
    }

    #[test]
    fn percentile_interpolates() {
        let sorted = [0.0, 10.0];
        // p=25 → idx=0.25 → 0*0.75 + 10*0.25 = 2.5
        assert_abs_diff_eq!(percentile(&sorted, 25.0), 2.5, epsilon = 1e-4);
    }

    #[test]
    fn percentile_single_element() {
        assert_abs_diff_eq!(percentile(&[7.0], 50.0), 7.0, epsilon = 1e-6);
    }

    #[test]
    fn percentile_empty() {
        assert_abs_diff_eq!(percentile(&[], 50.0), 0.0, epsilon = 1e-6);
    }

    #[test]
    fn median_unsorted_input() {
        let mut v = vec![5.0, 3.0, 1.0, 4.0, 2.0];
        assert_abs_diff_eq!(median(&mut v), 3.0, epsilon = 1e-6);
    }

    #[test]
    fn median_even_count_interpolates() {
        let mut v = vec![1.0, 2.0, 3.0, 4.0];
        // sorted [1,2,3,4], p=50 → idx=1.5 → 2*0.5 + 3*0.5 = 2.5
        assert_abs_diff_eq!(median(&mut v), 2.5, epsilon = 1e-4);
    }

    #[test]
    fn median_empty() {
        assert_abs_diff_eq!(median(&mut vec![]), 0.0, epsilon = 1e-6);
    }
}
