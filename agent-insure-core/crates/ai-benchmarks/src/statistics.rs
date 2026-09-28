//! Statistical calculations for benchmarks

use crate::Percentiles;

/// Calculate percentiles from a list of values
pub fn calculate_percentiles(values: &[f64]) -> Option<Percentiles> {
    if values.is_empty() {
        return None;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = sorted.len();

    let p25 = percentile(&sorted, 25);
    let p50 = percentile(&sorted, 50);
    let p75 = percentile(&sorted, 75);

    let mean = sorted.iter().sum::<f64>() / n as f64;

    let variance = sorted.iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>() / n as f64;
    let std = variance.sqrt();

    Some(Percentiles {
        p25,
        p50,
        p75,
        mean,
        std,
    })
}

/// Calculate a specific percentile
fn percentile(sorted: &[f64], p: usize) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }

    let n = sorted.len();
    let rank = (p as f64 / 100.0) * (n - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    let frac = rank - lower as f64;

    if lower == upper || upper >= n {
        sorted[lower.min(n - 1)]
    } else {
        sorted[lower] * (1.0 - frac) + sorted[upper] * frac
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentiles() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let percentiles = calculate_percentiles(&values).unwrap();

        assert!((percentiles.p50 - 5.5).abs() < 0.1);
        assert!((percentiles.mean - 5.5).abs() < 0.01);
    }

    #[test]
    fn test_empty_values() {
        let values: Vec<f64> = vec![];
        assert!(calculate_percentiles(&values).is_none());
    }
}
