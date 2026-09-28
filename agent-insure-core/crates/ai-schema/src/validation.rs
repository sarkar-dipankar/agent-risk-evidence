//! Schema validation utilities

use crate::error::SchemaError;
use crate::SCHEMA_VERSION;
use serde_json::Value;

/// Validation result containing errors and warnings
#[derive(Debug, Default)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
        self.valid = false;
    }

    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }
}

/// Schema validator for Agent Insure records
pub struct Validator {
    schema_version: String,
    enforce_privacy_tiers: bool,
    strict_mode: bool,
}

impl Validator {
    /// Create a new validator with default settings
    pub fn new() -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            enforce_privacy_tiers: true,
            strict_mode: false,
        }
    }

    /// Set the expected schema version
    pub fn with_schema_version(mut self, version: impl Into<String>) -> Self {
        self.schema_version = version.into();
        self
    }

    /// Enable or disable privacy tier enforcement
    pub fn with_privacy_enforcement(mut self, enforce: bool) -> Self {
        self.enforce_privacy_tiers = enforce;
        self
    }

    /// Enable or disable strict mode
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Validate a single record
    pub fn validate_record(
        &self,
        entity_type: &str,
        data: &Value,
    ) -> Result<ValidationResult, SchemaError> {
        let mut result = ValidationResult::new();

        // Check schema_name matches
        if let Some(schema_name) = data.get("schema_name").and_then(|v| v.as_str()) {
            if schema_name != entity_type {
                let msg = format!(
                    "schema_name mismatch: expected '{}', got '{}'",
                    entity_type, schema_name
                );
                if self.strict_mode {
                    result.add_error(&msg);
                } else {
                    result.add_warning(&msg);
                }
            }
        }

        // Check schema_version
        if let Some(version) = data.get("schema_version").and_then(|v| v.as_str()) {
            if version != self.schema_version {
                let msg = format!(
                    "schema_version mismatch: expected '{}', got '{}'",
                    self.schema_version, version
                );
                if self.strict_mode {
                    result.add_error(&msg);
                } else {
                    result.add_warning(&msg);
                }
            }
        }

        // Check for Tier 0 fields (should never be present)
        if self.enforce_privacy_tiers {
            self.check_privacy_tiers(data, &mut result);
        }

        // Entity-specific validation
        match entity_type {
            "Agent" => self.validate_agent(data, &mut result),
            "ExposurePeriod" => self.validate_exposure_period(data, &mut result),
            "ControlSnapshot" => self.validate_control_snapshot(data, &mut result),
            "Incident" => self.validate_incident(data, &mut result),
            "ClaimDevelopment" => self.validate_claim_development(data, &mut result),
            _ => {
                return Err(SchemaError::UnknownEntityType(entity_type.to_string()));
            }
        }

        Ok(result)
    }

    /// Check for Tier 0 fields that should not be present
    fn check_privacy_tiers(&self, data: &Value, result: &mut ValidationResult) {
        let tier_0_fields = [
            "raw_prompt",
            "raw_response",
            "customer_pii",
            "credentials",
            "internal_trace_id",
            "session_transcript",
        ];

        if let Some(obj) = data.as_object() {
            for field in tier_0_fields {
                if obj.contains_key(field) {
                    result.add_error(format!(
                        "Tier 0 field '{}' not allowed in submissions",
                        field
                    ));
                }
            }
        }
    }

    fn validate_agent(&self, data: &Value, result: &mut ValidationResult) {
        // Required fields
        if data.get("agent_id").is_none() {
            result.add_error("agent_id: Field required");
        }
        if data.get("use_case_class").is_none() {
            result.add_error("use_case_class: Field required");
        }
        if data.get("industry_class").is_none() {
            result.add_error("industry_class: Field required");
        }
    }

    fn validate_exposure_period(&self, data: &Value, result: &mut ValidationResult) {
        // Required fields
        if data.get("exposure_id").is_none() {
            result.add_error("exposure_id: Field required");
        }
        if data.get("agent_id").is_none() {
            result.add_error("agent_id: Field required");
        }
        if data.get("period_start").is_none() {
            result.add_error("period_start: Field required");
        }
        if data.get("period_end").is_none() {
            result.add_error("period_end: Field required");
        }

        // Business rules
        if let (Some(start), Some(end)) = (
            data.get("period_start").and_then(|v| v.as_str()),
            data.get("period_end").and_then(|v| v.as_str()),
        ) {
            if start > end {
                result.add_error("period_end must be >= period_start");
            }
        }

        // Range checks
        if let Some(score) = data.get("authority_surface_score").and_then(|v| v.as_u64()) {
            if score > 100 {
                result.add_error("authority_surface_score must be 0-100");
            }
        }
    }

    fn validate_control_snapshot(&self, data: &Value, result: &mut ValidationResult) {
        // Required fields
        if data.get("control_snapshot_id").is_none() {
            result.add_error("control_snapshot_id: Field required");
        }
        if data.get("agent_id").is_none() {
            result.add_error("agent_id: Field required");
        }
        if data.get("autonomy_level").is_none() {
            result.add_error("autonomy_level: Field required");
        }

        // Range checks
        if let Some(pct) = data.get("audit_log_coverage_pct").and_then(|v| v.as_u64()) {
            if pct > 100 {
                result.add_error("audit_log_coverage_pct must be 0-100");
            }
        }
        if let Some(pct) = data.get("red_team_coverage_pct").and_then(|v| v.as_u64()) {
            if pct > 100 {
                result.add_error("red_team_coverage_pct must be 0-100");
            }
        }
    }

    fn validate_incident(&self, data: &Value, result: &mut ValidationResult) {
        // Required fields
        if data.get("incident_id").is_none() {
            result.add_error("incident_id: Field required");
        }
        if data.get("agent_id").is_none() {
            result.add_error("agent_id: Field required");
        }
        if data.get("incident_class").is_none() {
            result.add_error("incident_class: Field required");
        }
        if data.get("occurred_at").is_none() {
            result.add_error("occurred_at: Field required");
        }
    }

    fn validate_claim_development(&self, data: &Value, result: &mut ValidationResult) {
        // Required fields
        if data.get("claim_id").is_none() {
            result.add_error("claim_id: Field required");
        }
        if data.get("incident_id").is_none() {
            result.add_error("incident_id: Field required");
        }
        if data.get("valuation_date").is_none() {
            result.add_error("valuation_date: Field required");
        }

        // Non-negative checks
        if let Some(amount) = data
            .get("paid_amount_cumulative")
            .and_then(|v| v.as_f64())
        {
            if amount < 0.0 {
                result.add_error("paid_amount_cumulative must be non-negative");
            }
        }
        if let Some(amount) = data.get("reserve_amount").and_then(|v| v.as_f64()) {
            if amount < 0.0 {
                result.add_error("reserve_amount must be non-negative");
            }
        }
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_valid_agent() {
        let validator = Validator::new();
        let data = json!({
            "schema_name": "Agent",
            "schema_version": "1.0.0",
            "agent_id": "agent_001",
            "use_case_class": "customer_support",
            "industry_class": "financial_services"
        });

        let result = validator.validate_record("Agent", &data).unwrap();
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_missing_required_field() {
        let validator = Validator::new();
        let data = json!({
            "schema_name": "Agent",
            "use_case_class": "customer_support"
            // Missing agent_id and industry_class
        });

        let result = validator.validate_record("Agent", &data).unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("agent_id")));
    }

    #[test]
    fn test_tier_0_field_rejected() {
        let validator = Validator::new();
        let data = json!({
            "schema_name": "Agent",
            "agent_id": "agent_001",
            "use_case_class": "customer_support",
            "industry_class": "technology",
            "raw_prompt": "This is sensitive data"
        });

        let result = validator.validate_record("Agent", &data).unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("Tier 0")));
    }
}
