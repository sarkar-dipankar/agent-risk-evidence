//! Benchmark aggregation

use crate::{
    kanonymity::KAnonymity,
    statistics::calculate_percentiles,
    BenchmarkConfig, BenchmarkError, BenchmarkReport, IndustryBenchmark,
};
use ai_schema::ControlSnapshot;
use chrono::Utc;
use std::collections::HashMap;

/// Generates industry benchmarks from organization data
pub struct BenchmarkGenerator {
    config: BenchmarkConfig,
    kanonymity: KAnonymity,
}

impl BenchmarkGenerator {
    pub fn new(config: BenchmarkConfig) -> Self {
        let kanonymity = KAnonymity::new(config.k_threshold);
        Self { config, kanonymity }
    }

    /// Generate benchmarks from control snapshot data
    pub fn generate_from_controls(
        &self,
        controls: &[(String, ControlSnapshot)], // (industry, control)
    ) -> Result<BenchmarkReport, BenchmarkError> {
        // Group by industry
        let mut industry_groups: HashMap<String, Vec<&ControlSnapshot>> = HashMap::new();

        for (industry, control) in controls {
            industry_groups
                .entry(industry.clone())
                .or_default()
                .push(control);
        }

        // Apply k-anonymity
        let (passing, suppressed) = self.kanonymity.filter_groups(industry_groups);

        // Generate benchmarks for passing industries
        let mut industries = Vec::new();

        for (industry, controls) in passing {
            if controls.len() < self.config.min_sample_size {
                continue;
            }

            let benchmark = self.calculate_industry_benchmark(&industry, &controls);
            industries.push(benchmark);
        }

        Ok(BenchmarkReport {
            generated_at: Utc::now(),
            schema_version: ai_schema::SCHEMA_VERSION.to_string(),
            k_threshold: self.config.k_threshold,
            industries,
            suppressed_industries: suppressed,
        })
    }

    fn calculate_industry_benchmark(
        &self,
        industry: &str,
        controls: &[&ControlSnapshot],
    ) -> IndustryBenchmark {
        // Extract metrics
        let audit_scores: Vec<f64> = controls
            .iter()
            .filter_map(|c| c.audit_log_coverage_pct.map(|v| v as f64))
            .collect();

        let redteam_scores: Vec<f64> = controls
            .iter()
            .filter_map(|c| c.red_team_coverage_pct.map(|v| v as f64))
            .collect();

        let hitl_count = controls
            .iter()
            .filter(|c| c.hitl_required_for_high_risk)
            .count();

        let hitl_rate = if controls.is_empty() {
            None
        } else {
            Some((hitl_count as f64 / controls.len() as f64) * 100.0)
        };

        IndustryBenchmark {
            industry: industry.to_string(),
            sample_size: controls.len(),
            audit_log_coverage: calculate_percentiles(&audit_scores),
            red_team_coverage: calculate_percentiles(&redteam_scores),
            incident_frequency_per_1000_hours: None, // Would need incident data
            average_severity: None, // Would need claims data
            hitl_adoption_rate: hitl_rate,
        }
    }
}

impl Default for BenchmarkGenerator {
    fn default() -> Self {
        Self::new(BenchmarkConfig::default())
    }
}
