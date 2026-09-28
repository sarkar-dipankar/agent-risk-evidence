//! Core entity types for Agent Insure schema

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::*;
use crate::SCHEMA_VERSION;

/// AI Agent profile and classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Unique identifier for the agent
    pub agent_id: String,

    /// Use case classification
    pub use_case_class: UseCaseClass,

    /// Industry classification
    pub industry_class: IndustryClass,

    /// Organization identifier (pseudonymized for Tier 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,

    /// Geographic region of operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geographic_region: Option<GeographicRegion>,

    /// When this agent was first deployed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_date: Option<NaiveDate>,

    /// Schema metadata
    #[serde(default = "default_schema_name_agent")]
    pub schema_name: String,

    #[serde(default = "default_schema_version")]
    pub schema_version: String,

    pub generated_at: DateTime<Utc>,
}

/// Time-bounded exposure period with activity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposurePeriod {
    /// Unique identifier
    pub exposure_id: String,

    /// Reference to agent
    pub agent_id: String,

    /// Period start date
    pub period_start: NaiveDate,

    /// Period end date
    pub period_end: NaiveDate,

    /// Hours the agent was active
    pub agent_active_hours: Decimal,

    /// Total actions executed
    #[serde(default)]
    pub actions_executed_total: u64,

    /// High-risk actions executed
    #[serde(default)]
    pub actions_executed_high_risk: u64,

    /// Authority surface score (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority_surface_score: Option<u8>,

    /// Unique resources accessed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_resources_accessed: Option<u64>,

    /// External API calls made
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_api_calls: Option<u64>,

    /// Schema metadata
    #[serde(default = "default_schema_name_exposure")]
    pub schema_name: String,

    #[serde(default = "default_schema_version")]
    pub schema_version: String,

    pub generated_at: DateTime<Utc>,
}

/// Security posture snapshot at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlSnapshot {
    /// Unique identifier
    pub control_snapshot_id: String,

    /// Reference to agent
    pub agent_id: String,

    /// Effective start date
    pub effective_start: NaiveDate,

    /// Effective end date
    pub effective_end: NaiveDate,

    /// Autonomy level configuration
    pub autonomy_level: AutonomyLevel,

    /// Whether HITL is required for high-risk actions
    #[serde(default)]
    pub hitl_required_for_high_risk: bool,

    /// Whether least privilege is enforced
    #[serde(default)]
    pub least_privilege_enforced: bool,

    /// Whether sandboxing is enforced
    #[serde(default)]
    pub sandboxing_enforced: bool,

    /// Secret rotation period in days
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_rotation_days: Option<u32>,

    /// Audit log coverage percentage (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_log_coverage_pct: Option<u8>,

    /// Red team coverage percentage (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub red_team_coverage_pct: Option<u8>,

    /// Deployment events in last 30 days
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_events_30d: Option<u32>,

    /// Model version updates in last 30 days
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_version_updates_30d: Option<u32>,

    /// Schema metadata
    #[serde(default = "default_schema_name_control")]
    pub schema_name: String,

    #[serde(default = "default_schema_version")]
    pub schema_version: String,

    pub generated_at: DateTime<Utc>,
}

/// Loss event or near-miss incident
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    /// Unique identifier
    pub incident_id: String,

    /// Reference to agent
    pub agent_id: String,

    /// Incident classification
    pub incident_class: IncidentClass,

    /// Severity level
    pub severity_level: SeverityLevel,

    /// Line of business
    pub line_of_business: LineOfBusiness,

    /// When the incident occurred
    pub occurred_at: DateTime<Utc>,

    /// When the incident was detected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_at: Option<DateTime<Utc>>,

    /// When the incident was resolved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<DateTime<Utc>>,

    /// Root cause identifier (e.g., "prompt_injection", "model_hallucination")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_cause_id: Option<String>,

    /// Schema metadata
    #[serde(default = "default_schema_name_incident")]
    pub schema_name: String,

    #[serde(default = "default_schema_version")]
    pub schema_version: String,

    pub generated_at: DateTime<Utc>,
}

/// Financial development of a claim
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimDevelopment {
    /// Unique identifier
    pub claim_id: String,

    /// Reference to incident
    pub incident_id: String,

    /// Valuation date for this snapshot
    pub valuation_date: NaiveDate,

    /// Cumulative paid amount
    pub paid_amount_cumulative: Decimal,

    /// Current reserve amount
    pub reserve_amount: Decimal,

    /// Expected ultimate amount
    pub expected_ultimate_amount: Decimal,

    /// Claim status
    pub status: ClaimStatus,

    /// Currency code (ISO 4217)
    #[serde(default = "default_currency")]
    pub currency: String,

    /// Schema metadata
    #[serde(default = "default_schema_name_claim")]
    pub schema_name: String,

    #[serde(default = "default_schema_version")]
    pub schema_version: String,

    pub generated_at: DateTime<Utc>,
}

// Default functions for serde
fn default_schema_version() -> String {
    SCHEMA_VERSION.to_string()
}

fn default_schema_name_agent() -> String {
    "Agent".to_string()
}

fn default_schema_name_exposure() -> String {
    "ExposurePeriod".to_string()
}

fn default_schema_name_control() -> String {
    "ControlSnapshot".to_string()
}

fn default_schema_name_incident() -> String {
    "Incident".to_string()
}

fn default_schema_name_claim() -> String {
    "ClaimDevelopment".to_string()
}

fn default_currency() -> String {
    "USD".to_string()
}

impl Default for Agent {
    fn default() -> Self {
        Self {
            agent_id: Uuid::new_v4().to_string(),
            use_case_class: UseCaseClass::default(),
            industry_class: IndustryClass::default(),
            organization_id: None,
            geographic_region: None,
            deployment_date: None,
            schema_name: default_schema_name_agent(),
            schema_version: default_schema_version(),
            generated_at: Utc::now(),
        }
    }
}

impl Default for ExposurePeriod {
    fn default() -> Self {
        let today = Utc::now().date_naive();
        Self {
            exposure_id: Uuid::new_v4().to_string(),
            agent_id: String::new(),
            period_start: today,
            period_end: today,
            agent_active_hours: Decimal::ZERO,
            actions_executed_total: 0,
            actions_executed_high_risk: 0,
            authority_surface_score: None,
            unique_resources_accessed: None,
            external_api_calls: None,
            schema_name: default_schema_name_exposure(),
            schema_version: default_schema_version(),
            generated_at: Utc::now(),
        }
    }
}

impl Default for ControlSnapshot {
    fn default() -> Self {
        let today = Utc::now().date_naive();
        Self {
            control_snapshot_id: Uuid::new_v4().to_string(),
            agent_id: String::new(),
            effective_start: today,
            effective_end: today,
            autonomy_level: AutonomyLevel::default(),
            hitl_required_for_high_risk: true,
            least_privilege_enforced: true,
            sandboxing_enforced: true,
            secret_rotation_days: Some(30),
            audit_log_coverage_pct: Some(100),
            red_team_coverage_pct: None,
            deployment_events_30d: None,
            model_version_updates_30d: None,
            schema_name: default_schema_name_control(),
            schema_version: default_schema_version(),
            generated_at: Utc::now(),
        }
    }
}

impl Default for Incident {
    fn default() -> Self {
        Self {
            incident_id: Uuid::new_v4().to_string(),
            agent_id: String::new(),
            incident_class: IncidentClass::default(),
            severity_level: SeverityLevel::default(),
            line_of_business: LineOfBusiness::default(),
            occurred_at: Utc::now(),
            detected_at: None,
            resolved_at: None,
            root_cause_id: None,
            schema_name: default_schema_name_incident(),
            schema_version: default_schema_version(),
            generated_at: Utc::now(),
        }
    }
}

impl Default for ClaimDevelopment {
    fn default() -> Self {
        Self {
            claim_id: Uuid::new_v4().to_string(),
            incident_id: String::new(),
            valuation_date: Utc::now().date_naive(),
            paid_amount_cumulative: Decimal::ZERO,
            reserve_amount: Decimal::ZERO,
            expected_ultimate_amount: Decimal::ZERO,
            status: ClaimStatus::default(),
            currency: default_currency(),
            schema_name: default_schema_name_claim(),
            schema_version: default_schema_version(),
            generated_at: Utc::now(),
        }
    }
}
