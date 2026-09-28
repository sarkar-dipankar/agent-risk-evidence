//! Industry Benchmarks for Agent Insure
//!
//! This crate provides privacy-preserving industry benchmarks:
//!
//! - K-anonymity aggregation (k >= 3)
//! - Industry-level statistics
//! - Percentile calculations
//!
//! # Privacy Tiers
//!
//! - **Tier 0**: Raw data (never stored centrally)
//! - **Tier 1**: Aggregated per-org data (org-visible only)
//! - **Tier 2**: K-anonymized benchmarks (public)
//!
//! # Example
//!
//! ```rust,ignore
//! use ai_benchmarks::{BenchmarkGenerator, BenchmarkConfig};
//!
//! let generator = BenchmarkGenerator::new(config);
//! let benchmarks = generator.generate_industry_benchmarks().await?;
//! ```

pub mod aggregator;
pub mod attacks;
pub mod dataset;
pub mod dp_noise;
pub mod generalize;
pub mod hierarchy;
pub mod kanonymity;
pub mod ldiversity;
pub mod statistics;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Benchmark errors
#[derive(Debug, Error)]
pub enum BenchmarkError {
    #[error("Insufficient data: need {required} samples, have {actual}")]
    InsufficientData { required: usize, actual: usize },

    #[error("K-anonymity violation: group size {size} < k={k}")]
    KAnonymityViolation { k: usize, size: usize },

    #[error("Storage error: {0}")]
    StorageError(#[from] ai_storage::StorageError),
}

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Minimum group size for k-anonymity
    pub k_threshold: usize,

    /// Minimum sample size for publishing
    pub min_sample_size: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            k_threshold: 3,
            min_sample_size: 5,
        }
    }
}

/// Percentile statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Percentiles {
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub mean: f64,
    pub std: f64,
}

/// Industry benchmark data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryBenchmark {
    /// Industry classification
    pub industry: String,

    /// Number of organizations in sample
    pub sample_size: usize,

    /// Audit log coverage percentiles
    pub audit_log_coverage: Option<Percentiles>,

    /// Red team coverage percentiles
    pub red_team_coverage: Option<Percentiles>,

    /// Incident frequency per 1000 hours
    pub incident_frequency_per_1000_hours: Option<Percentiles>,

    /// Average claim severity
    pub average_severity: Option<Percentiles>,

    /// HITL adoption rate (percentage)
    pub hitl_adoption_rate: Option<f64>,
}

/// Complete benchmark report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    /// Report generation timestamp
    pub generated_at: chrono::DateTime<chrono::Utc>,

    /// Schema version
    pub schema_version: String,

    /// K-anonymity threshold used
    pub k_threshold: usize,

    /// Benchmarks by industry
    pub industries: Vec<IndustryBenchmark>,

    /// Industries suppressed due to k-anonymity
    pub suppressed_industries: Vec<String>,
}
