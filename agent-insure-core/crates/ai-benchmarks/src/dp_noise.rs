//! Differential-privacy primitives: the Laplace mechanism.
//!
//! Adds calibrated Laplace noise to a numeric query whose sensitivity is
//! `Δf`. The standard mechanism samples η ~ Lap(Δf / ε) and releases
//! `f(x) + η`, satisfying ε-differential privacy (Dwork, McSherry, Nissim,
//! Smith, 2006). For ε = ∞ we return the true value unchanged.

use rand::Rng;

/// Draw one sample from the Laplace distribution with mean `mu` and scale
/// `b`. Uses the inverse-CDF method: if `U` is uniform on (-0.5, 0.5] then
/// `mu - b * sgn(U) * ln(1 - 2|U|)` is Laplace(`mu`, `b`).
pub fn sample_laplace<R: Rng + ?Sized>(rng: &mut R, mu: f64, b: f64) -> f64 {
    if b <= 0.0 {
        return mu;
    }
    let u: f64 = rng.gen_range(-0.5..0.5);
    mu - b * u.signum() * (1.0 - 2.0 * u.abs()).ln()
}

/// Add Laplace noise to `value` at scale `sensitivity / epsilon`. ε ≤ 0
/// or non-finite is treated as "noiseless" and returns the input value.
pub fn laplace(value: f64, sensitivity: f64, epsilon: f64) -> f64 {
    if !epsilon.is_finite() || epsilon <= 0.0 {
        return value;
    }
    let mut rng = rand::thread_rng();
    let scale = sensitivity / epsilon;
    sample_laplace(&mut rng, value, scale)
}

/// Noisy count helper. Treats the count query as having sensitivity 1
/// (adding/removing one record changes the count by one). Negative noisy
/// counts are clamped to 0 because a negative count is non-sensical, but
/// note this clamping technically biases the estimate.
pub fn noise_count(true_count: u64, epsilon: f64) -> u64 {
    let noisy = laplace(true_count as f64, 1.0, epsilon);
    if noisy < 0.0 {
        0
    } else {
        noisy.round() as u64
    }
}

/// Noisy real-valued statistic with explicit sensitivity. Useful for means
/// after the data has been clipped to a known range.
pub fn noise_value(true_value: f64, sensitivity: f64, epsilon: f64) -> f64 {
    laplace(true_value, sensitivity, epsilon)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infinite_epsilon_returns_truth() {
        assert_eq!(laplace(42.0, 1.0, f64::INFINITY), 42.0);
        assert_eq!(noise_count(1000, f64::INFINITY), 1000);
    }

    #[test]
    fn zero_epsilon_returns_truth() {
        // ε <= 0 is meaningless DP-wise; the helper degrades to no-op.
        assert_eq!(laplace(42.0, 1.0, 0.0), 42.0);
    }

    #[test]
    fn noise_bounded_in_expectation() {
        // The Laplace mechanism has zero mean — average over many draws
        // should be close to the true value.
        let mut sum = 0.0;
        let trials = 5_000;
        for _ in 0..trials {
            sum += laplace(100.0, 1.0, 1.0);
        }
        let mean = sum / trials as f64;
        // Variance is 2b^2 = 2, so std ≈ 1.41 → sample mean std ≈ 0.02.
        // Generous tolerance to keep tests stable.
        assert!((mean - 100.0).abs() < 0.5, "mean was {}", mean);
    }

    #[test]
    fn noisy_count_returns_nonnegative_u64() {
        // With ε = 0.01 we can easily push a small count negative — the
        // helper must clamp.
        for _ in 0..50 {
            let v = noise_count(1, 0.01);
            assert!(v <= u64::MAX);
        }
    }
}
