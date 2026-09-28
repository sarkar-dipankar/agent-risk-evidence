//! SQLite storage implementation

use crate::{traits::Storage, StorageError};
use ai_schema::{
    Agent, ClaimDevelopment, ControlSnapshot, ExposurePeriod, Incident,
    AutonomyLevel, ClaimStatus, GeographicRegion, IncidentClass, IndustryClass,
    LineOfBusiness, SeverityLevel, UseCaseClass,
};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use std::str::FromStr;

/// SQLite storage backend
pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    /// Connect to SQLite database
    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    /// Create a new in-memory database (for testing)
    pub async fn in_memory() -> Result<Self, StorageError> {
        Self::connect("sqlite::memory:").await
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<(), StorageError> {
        let migration_sql = include_str!("../migrations/001_initial.sql");

        // Split by semicolon and execute each statement
        for statement in migration_sql.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() {
                sqlx::query(trimmed)
                    .execute(&self.pool)
                    .await?;
            }
        }

        Ok(())
    }
}

// Helper functions for enum serialization/deserialization
fn serialize_enum<T: std::fmt::Debug>(value: &T) -> String {
    format!("{:?}", value).to_lowercase()
}

fn parse_use_case_class(s: &str) -> UseCaseClass {
    match s.to_lowercase().as_str() {
        "customersupport" | "customer_support" => UseCaseClass::CustomerSupport,
        "codegeneration" | "code_generation" => UseCaseClass::CodeGeneration,
        "dataanalysis" | "data_analysis" => UseCaseClass::DataAnalysis,
        "financialoperations" | "financial_operations" => UseCaseClass::FinancialOperations,
        "contentcreation" | "content_creation" => UseCaseClass::ContentCreation,
        "researchassistant" | "research_assistant" => UseCaseClass::ResearchAssistant,
        "workflowautomation" | "workflow_automation" => UseCaseClass::WorkflowAutomation,
        _ => UseCaseClass::Other,
    }
}

fn parse_industry_class(s: &str) -> IndustryClass {
    match s.to_lowercase().as_str() {
        "financialservices" | "financial_services" => IndustryClass::FinancialServices,
        "healthcare" => IndustryClass::Healthcare,
        "technology" => IndustryClass::Technology,
        "retail" => IndustryClass::Retail,
        "manufacturing" => IndustryClass::Manufacturing,
        "professionalservices" | "professional_services" => IndustryClass::ProfessionalServices,
        "government" => IndustryClass::Government,
        "education" => IndustryClass::Education,
        _ => IndustryClass::Other,
    }
}

fn parse_geographic_region(s: &str) -> GeographicRegion {
    match s.to_lowercase().as_str() {
        "northamerica" | "north_america" => GeographicRegion::NorthAmerica,
        "europe" => GeographicRegion::Europe,
        "asiapacific" | "asia_pacific" => GeographicRegion::AsiaPacific,
        "latinamerica" | "latin_america" => GeographicRegion::LatinAmerica,
        "middleeast" | "middle_east" => GeographicRegion::MiddleEast,
        "africa" => GeographicRegion::Africa,
        _ => GeographicRegion::Global,
    }
}

fn parse_autonomy_level(s: &str) -> AutonomyLevel {
    match s.to_lowercase().as_str() {
        "assistive" => AutonomyLevel::Assistive,
        "semiautonomous" | "semi_autonomous" => AutonomyLevel::SemiAutonomous,
        "autonomous" => AutonomyLevel::Autonomous,
        _ => AutonomyLevel::SemiAutonomous,
    }
}

fn parse_incident_class(s: &str) -> IncidentClass {
    match s.to_lowercase().as_str() {
        "databreachorleakage" | "data_breach_or_leakage" => IncidentClass::DataBreachOrLeakage,
        "unauthorizedaction" | "unauthorized_action" => IncidentClass::UnauthorizedAction,
        "financialloss" | "financial_loss" => IncidentClass::FinancialLoss,
        "complianceviolation" | "compliance_violation" => IncidentClass::ComplianceViolation,
        "servicedisruption" | "service_disruption" => IncidentClass::ServiceDisruption,
        "reputationalharm" | "reputational_harm" => IncidentClass::ReputationalHarm,
        "modelfailure" | "model_failure" => IncidentClass::ModelFailure,
        _ => IncidentClass::Other,
    }
}

fn parse_severity_level(s: &str) -> SeverityLevel {
    match s.to_lowercase().as_str() {
        "low" => SeverityLevel::Low,
        "medium" => SeverityLevel::Medium,
        "high" => SeverityLevel::High,
        "critical" => SeverityLevel::Critical,
        _ => SeverityLevel::Medium,
    }
}

fn parse_line_of_business(s: &str) -> LineOfBusiness {
    match s.to_lowercase().as_str() {
        "cybersecurity" | "cyber_security" => LineOfBusiness::CyberSecurity,
        "financialcrime" | "financial_crime" => LineOfBusiness::FinancialCrime,
        "professionalliability" | "professional_liability" => LineOfBusiness::ProfessionalLiability,
        "operationalinterruption" | "operational_interruption" => LineOfBusiness::OperationalInterruption,
        "regulatoryfines" | "regulatory_fines" => LineOfBusiness::RegulatoryFines,
        _ => LineOfBusiness::Other,
    }
}

fn parse_claim_status(s: &str) -> ClaimStatus {
    match s.to_lowercase().as_str() {
        "open" => ClaimStatus::Open,
        "closed" => ClaimStatus::Closed,
        "reopened" => ClaimStatus::Reopened,
        _ => ClaimStatus::Open,
    }
}

#[async_trait::async_trait]
impl Storage for SqliteStorage {
    async fn store_agent(&self, agent: &Agent) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO agents (
                agent_id, organization_id, use_case_class, industry_class,
                geographic_region, deployment_date, schema_name, schema_version, generated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(agent_id) DO UPDATE SET
                organization_id = excluded.organization_id,
                use_case_class = excluded.use_case_class,
                industry_class = excluded.industry_class,
                geographic_region = excluded.geographic_region,
                deployment_date = excluded.deployment_date,
                schema_version = excluded.schema_version,
                generated_at = excluded.generated_at
            "#,
        )
        .bind(&agent.agent_id)
        .bind(&agent.organization_id)
        .bind(serialize_enum(&agent.use_case_class))
        .bind(serialize_enum(&agent.industry_class))
        .bind(agent.geographic_region.as_ref().map(|r| serialize_enum(r)))
        .bind(agent.deployment_date.map(|d| d.to_string()))
        .bind(&agent.schema_name)
        .bind(&agent.schema_version)
        .bind(agent.generated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_agent(&self, agent_id: &str) -> Result<Option<Agent>, StorageError> {
        let row = sqlx::query(
            "SELECT * FROM agents WHERE agent_id = ?1"
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let geographic_region: Option<String> = row.get("geographic_region");
                let deployment_date: Option<String> = row.get("deployment_date");
                let generated_at: String = row.get("generated_at");

                Ok(Some(Agent {
                    agent_id: row.get("agent_id"),
                    organization_id: row.get("organization_id"),
                    use_case_class: parse_use_case_class(row.get("use_case_class")),
                    industry_class: parse_industry_class(row.get("industry_class")),
                    geographic_region: geographic_region.map(|s| parse_geographic_region(&s)),
                    deployment_date: deployment_date.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                    schema_name: row.get("schema_name"),
                    schema_version: row.get("schema_version"),
                    generated_at: DateTime::parse_from_rfc3339(&generated_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                }))
            }
            None => Ok(None),
        }
    }

    async fn list_agents(
        &self,
        organization_id: Option<&str>,
    ) -> Result<Vec<Agent>, StorageError> {
        let rows = match organization_id {
            Some(org_id) => {
                sqlx::query("SELECT * FROM agents WHERE organization_id = ?1")
                    .bind(org_id)
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                sqlx::query("SELECT * FROM agents")
                    .fetch_all(&self.pool)
                    .await?
            }
        };

        let mut agents = Vec::new();
        for row in rows {
            let geographic_region: Option<String> = row.get("geographic_region");
            let deployment_date: Option<String> = row.get("deployment_date");
            let generated_at: String = row.get("generated_at");

            agents.push(Agent {
                agent_id: row.get("agent_id"),
                organization_id: row.get("organization_id"),
                use_case_class: parse_use_case_class(row.get("use_case_class")),
                industry_class: parse_industry_class(row.get("industry_class")),
                geographic_region: geographic_region.map(|s| parse_geographic_region(&s)),
                deployment_date: deployment_date.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                schema_name: row.get("schema_name"),
                schema_version: row.get("schema_version"),
                generated_at: DateTime::parse_from_rfc3339(&generated_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            });
        }

        Ok(agents)
    }

    async fn store_exposure(&self, exposure: &ExposurePeriod) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO exposure_periods (
                exposure_id, agent_id, period_start, period_end,
                agent_active_hours, actions_executed_total, actions_executed_high_risk,
                authority_surface_score, unique_resources_accessed, external_api_calls,
                schema_name, schema_version, generated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(exposure_id) DO UPDATE SET
                period_start = excluded.period_start,
                period_end = excluded.period_end,
                agent_active_hours = excluded.agent_active_hours,
                actions_executed_total = excluded.actions_executed_total,
                actions_executed_high_risk = excluded.actions_executed_high_risk,
                authority_surface_score = excluded.authority_surface_score,
                unique_resources_accessed = excluded.unique_resources_accessed,
                external_api_calls = excluded.external_api_calls,
                schema_version = excluded.schema_version,
                generated_at = excluded.generated_at
            "#,
        )
        .bind(&exposure.exposure_id)
        .bind(&exposure.agent_id)
        .bind(exposure.period_start.to_string())
        .bind(exposure.period_end.to_string())
        .bind(exposure.agent_active_hours.to_string().parse::<f64>().unwrap_or(0.0))
        .bind(exposure.actions_executed_total as i64)
        .bind(exposure.actions_executed_high_risk as i64)
        .bind(exposure.authority_surface_score.map(|v| v as i32))
        .bind(exposure.unique_resources_accessed.map(|v| v as i64))
        .bind(exposure.external_api_calls.map(|v| v as i64))
        .bind(&exposure.schema_name)
        .bind(&exposure.schema_version)
        .bind(exposure.generated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_exposures(
        &self,
        agent_id: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<ExposurePeriod>, StorageError> {
        let query = match (start_date, end_date) {
            (Some(start), Some(end)) => {
                sqlx::query(
                    "SELECT * FROM exposure_periods WHERE agent_id = ?1 AND period_start >= ?2 AND period_end <= ?3"
                )
                .bind(agent_id)
                .bind(start.to_string())
                .bind(end.to_string())
                .fetch_all(&self.pool)
                .await?
            }
            (Some(start), None) => {
                sqlx::query(
                    "SELECT * FROM exposure_periods WHERE agent_id = ?1 AND period_start >= ?2"
                )
                .bind(agent_id)
                .bind(start.to_string())
                .fetch_all(&self.pool)
                .await?
            }
            (None, Some(end)) => {
                sqlx::query(
                    "SELECT * FROM exposure_periods WHERE agent_id = ?1 AND period_end <= ?2"
                )
                .bind(agent_id)
                .bind(end.to_string())
                .fetch_all(&self.pool)
                .await?
            }
            (None, None) => {
                sqlx::query("SELECT * FROM exposure_periods WHERE agent_id = ?1")
                    .bind(agent_id)
                    .fetch_all(&self.pool)
                    .await?
            }
        };

        let mut exposures = Vec::new();
        for row in query {
            let period_start: String = row.get("period_start");
            let period_end: String = row.get("period_end");
            let generated_at: String = row.get("generated_at");
            let agent_active_hours: f64 = row.get("agent_active_hours");

            exposures.push(ExposurePeriod {
                exposure_id: row.get("exposure_id"),
                agent_id: row.get("agent_id"),
                period_start: NaiveDate::parse_from_str(&period_start, "%Y-%m-%d").unwrap_or_default(),
                period_end: NaiveDate::parse_from_str(&period_end, "%Y-%m-%d").unwrap_or_default(),
                agent_active_hours: Decimal::from_str(&agent_active_hours.to_string()).unwrap_or_default(),
                actions_executed_total: row.get::<i64, _>("actions_executed_total") as u64,
                actions_executed_high_risk: row.get::<i64, _>("actions_executed_high_risk") as u64,
                authority_surface_score: row.get::<Option<i32>, _>("authority_surface_score").map(|v| v as u8),
                unique_resources_accessed: row.get::<Option<i64>, _>("unique_resources_accessed").map(|v| v as u64),
                external_api_calls: row.get::<Option<i64>, _>("external_api_calls").map(|v| v as u64),
                schema_name: row.get("schema_name"),
                schema_version: row.get("schema_version"),
                generated_at: DateTime::parse_from_rfc3339(&generated_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            });
        }

        Ok(exposures)
    }

    async fn store_control(&self, control: &ControlSnapshot) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO control_snapshots (
                control_snapshot_id, agent_id, effective_start, effective_end,
                autonomy_level, hitl_required_for_high_risk, least_privilege_enforced,
                sandboxing_enforced, secret_rotation_days, audit_log_coverage_pct,
                red_team_coverage_pct, deployment_events_30d, model_version_updates_30d,
                schema_name, schema_version, generated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            ON CONFLICT(control_snapshot_id) DO UPDATE SET
                effective_start = excluded.effective_start,
                effective_end = excluded.effective_end,
                autonomy_level = excluded.autonomy_level,
                hitl_required_for_high_risk = excluded.hitl_required_for_high_risk,
                least_privilege_enforced = excluded.least_privilege_enforced,
                sandboxing_enforced = excluded.sandboxing_enforced,
                secret_rotation_days = excluded.secret_rotation_days,
                audit_log_coverage_pct = excluded.audit_log_coverage_pct,
                red_team_coverage_pct = excluded.red_team_coverage_pct,
                deployment_events_30d = excluded.deployment_events_30d,
                model_version_updates_30d = excluded.model_version_updates_30d,
                schema_version = excluded.schema_version,
                generated_at = excluded.generated_at
            "#,
        )
        .bind(&control.control_snapshot_id)
        .bind(&control.agent_id)
        .bind(control.effective_start.to_string())
        .bind(control.effective_end.to_string())
        .bind(serialize_enum(&control.autonomy_level))
        .bind(control.hitl_required_for_high_risk)
        .bind(control.least_privilege_enforced)
        .bind(control.sandboxing_enforced)
        .bind(control.secret_rotation_days.map(|v| v as i32))
        .bind(control.audit_log_coverage_pct.map(|v| v as i32))
        .bind(control.red_team_coverage_pct.map(|v| v as i32))
        .bind(control.deployment_events_30d.map(|v| v as i32))
        .bind(control.model_version_updates_30d.map(|v| v as i32))
        .bind(&control.schema_name)
        .bind(&control.schema_version)
        .bind(control.generated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_controls(
        &self,
        agent_id: &str,
        effective_date: Option<NaiveDate>,
    ) -> Result<Vec<ControlSnapshot>, StorageError> {
        let rows = match effective_date {
            Some(date) => {
                sqlx::query(
                    "SELECT * FROM control_snapshots WHERE agent_id = ?1 AND effective_start <= ?2 AND effective_end >= ?2"
                )
                .bind(agent_id)
                .bind(date.to_string())
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query("SELECT * FROM control_snapshots WHERE agent_id = ?1")
                    .bind(agent_id)
                    .fetch_all(&self.pool)
                    .await?
            }
        };

        let mut controls = Vec::new();
        for row in rows {
            let effective_start: String = row.get("effective_start");
            let effective_end: String = row.get("effective_end");
            let generated_at: String = row.get("generated_at");

            controls.push(ControlSnapshot {
                control_snapshot_id: row.get("control_snapshot_id"),
                agent_id: row.get("agent_id"),
                effective_start: NaiveDate::parse_from_str(&effective_start, "%Y-%m-%d").unwrap_or_default(),
                effective_end: NaiveDate::parse_from_str(&effective_end, "%Y-%m-%d").unwrap_or_default(),
                autonomy_level: parse_autonomy_level(row.get("autonomy_level")),
                hitl_required_for_high_risk: row.get("hitl_required_for_high_risk"),
                least_privilege_enforced: row.get("least_privilege_enforced"),
                sandboxing_enforced: row.get("sandboxing_enforced"),
                secret_rotation_days: row.get::<Option<i32>, _>("secret_rotation_days").map(|v| v as u32),
                audit_log_coverage_pct: row.get::<Option<i32>, _>("audit_log_coverage_pct").map(|v| v as u8),
                red_team_coverage_pct: row.get::<Option<i32>, _>("red_team_coverage_pct").map(|v| v as u8),
                deployment_events_30d: row.get::<Option<i32>, _>("deployment_events_30d").map(|v| v as u32),
                model_version_updates_30d: row.get::<Option<i32>, _>("model_version_updates_30d").map(|v| v as u32),
                schema_name: row.get("schema_name"),
                schema_version: row.get("schema_version"),
                generated_at: DateTime::parse_from_rfc3339(&generated_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            });
        }

        Ok(controls)
    }

    async fn store_incident(&self, incident: &Incident) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO incidents (
                incident_id, agent_id, incident_class, severity_level,
                line_of_business, occurred_at, detected_at, resolved_at,
                root_cause_id, schema_name, schema_version, generated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(incident_id) DO UPDATE SET
                incident_class = excluded.incident_class,
                severity_level = excluded.severity_level,
                line_of_business = excluded.line_of_business,
                occurred_at = excluded.occurred_at,
                detected_at = excluded.detected_at,
                resolved_at = excluded.resolved_at,
                root_cause_id = excluded.root_cause_id,
                schema_version = excluded.schema_version,
                generated_at = excluded.generated_at
            "#,
        )
        .bind(&incident.incident_id)
        .bind(&incident.agent_id)
        .bind(serialize_enum(&incident.incident_class))
        .bind(serialize_enum(&incident.severity_level))
        .bind(serialize_enum(&incident.line_of_business))
        .bind(incident.occurred_at.to_rfc3339())
        .bind(incident.detected_at.map(|dt| dt.to_rfc3339()))
        .bind(incident.resolved_at.map(|dt| dt.to_rfc3339()))
        .bind(&incident.root_cause_id)
        .bind(&incident.schema_name)
        .bind(&incident.schema_version)
        .bind(incident.generated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_incidents(
        &self,
        agent_id: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<Incident>, StorageError> {
        let rows = match (start_date, end_date) {
            (Some(start), Some(end)) => {
                sqlx::query(
                    "SELECT * FROM incidents WHERE agent_id = ?1 AND date(occurred_at) >= ?2 AND date(occurred_at) <= ?3"
                )
                .bind(agent_id)
                .bind(start.to_string())
                .bind(end.to_string())
                .fetch_all(&self.pool)
                .await?
            }
            (Some(start), None) => {
                sqlx::query(
                    "SELECT * FROM incidents WHERE agent_id = ?1 AND date(occurred_at) >= ?2"
                )
                .bind(agent_id)
                .bind(start.to_string())
                .fetch_all(&self.pool)
                .await?
            }
            (None, Some(end)) => {
                sqlx::query(
                    "SELECT * FROM incidents WHERE agent_id = ?1 AND date(occurred_at) <= ?2"
                )
                .bind(agent_id)
                .bind(end.to_string())
                .fetch_all(&self.pool)
                .await?
            }
            (None, None) => {
                sqlx::query("SELECT * FROM incidents WHERE agent_id = ?1")
                    .bind(agent_id)
                    .fetch_all(&self.pool)
                    .await?
            }
        };

        let mut incidents = Vec::new();
        for row in rows {
            let occurred_at: String = row.get("occurred_at");
            let detected_at: Option<String> = row.get("detected_at");
            let resolved_at: Option<String> = row.get("resolved_at");
            let generated_at: String = row.get("generated_at");

            incidents.push(Incident {
                incident_id: row.get("incident_id"),
                agent_id: row.get("agent_id"),
                incident_class: parse_incident_class(row.get("incident_class")),
                severity_level: parse_severity_level(row.get("severity_level")),
                line_of_business: parse_line_of_business(row.get("line_of_business")),
                occurred_at: DateTime::parse_from_rfc3339(&occurred_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                detected_at: detected_at.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
                resolved_at: resolved_at.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
                root_cause_id: row.get("root_cause_id"),
                schema_name: row.get("schema_name"),
                schema_version: row.get("schema_version"),
                generated_at: DateTime::parse_from_rfc3339(&generated_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            });
        }

        Ok(incidents)
    }

    async fn store_claim(&self, claim: &ClaimDevelopment) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO claim_developments (
                claim_id, incident_id, valuation_date,
                paid_amount_cumulative, reserve_amount, expected_ultimate_amount,
                status, currency, schema_name, schema_version, generated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(claim_id, valuation_date) DO UPDATE SET
                paid_amount_cumulative = excluded.paid_amount_cumulative,
                reserve_amount = excluded.reserve_amount,
                expected_ultimate_amount = excluded.expected_ultimate_amount,
                status = excluded.status,
                currency = excluded.currency,
                schema_version = excluded.schema_version,
                generated_at = excluded.generated_at
            "#,
        )
        .bind(&claim.claim_id)
        .bind(&claim.incident_id)
        .bind(claim.valuation_date.to_string())
        .bind(claim.paid_amount_cumulative.to_string().parse::<f64>().unwrap_or(0.0))
        .bind(claim.reserve_amount.to_string().parse::<f64>().unwrap_or(0.0))
        .bind(claim.expected_ultimate_amount.to_string().parse::<f64>().unwrap_or(0.0))
        .bind(serialize_enum(&claim.status))
        .bind(&claim.currency)
        .bind(&claim.schema_name)
        .bind(&claim.schema_version)
        .bind(claim.generated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_claims(
        &self,
        incident_id: &str,
        valuation_date: Option<NaiveDate>,
    ) -> Result<Vec<ClaimDevelopment>, StorageError> {
        let rows = match valuation_date {
            Some(date) => {
                sqlx::query(
                    "SELECT * FROM claim_developments WHERE incident_id = ?1 AND valuation_date = ?2"
                )
                .bind(incident_id)
                .bind(date.to_string())
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query("SELECT * FROM claim_developments WHERE incident_id = ?1")
                    .bind(incident_id)
                    .fetch_all(&self.pool)
                    .await?
            }
        };

        let mut claims = Vec::new();
        for row in rows {
            let valuation_date: String = row.get("valuation_date");
            let generated_at: String = row.get("generated_at");
            let paid_amount: f64 = row.get("paid_amount_cumulative");
            let reserve_amount: f64 = row.get("reserve_amount");
            let expected_ultimate: f64 = row.get("expected_ultimate_amount");

            claims.push(ClaimDevelopment {
                claim_id: row.get("claim_id"),
                incident_id: row.get("incident_id"),
                valuation_date: NaiveDate::parse_from_str(&valuation_date, "%Y-%m-%d").unwrap_or_default(),
                paid_amount_cumulative: Decimal::from_str(&paid_amount.to_string()).unwrap_or_default(),
                reserve_amount: Decimal::from_str(&reserve_amount.to_string()).unwrap_or_default(),
                expected_ultimate_amount: Decimal::from_str(&expected_ultimate.to_string()).unwrap_or_default(),
                status: parse_claim_status(row.get("status")),
                currency: row.get("currency"),
                schema_name: row.get("schema_name"),
                schema_version: row.get("schema_version"),
                generated_at: DateTime::parse_from_rfc3339(&generated_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            });
        }

        Ok(claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_roundtrip() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        storage.migrate().await.unwrap();

        let agent = Agent {
            agent_id: "test_agent_001".to_string(),
            use_case_class: UseCaseClass::CustomerSupport,
            industry_class: IndustryClass::FinancialServices,
            organization_id: Some("org_test".to_string()),
            ..Default::default()
        };

        storage.store_agent(&agent).await.unwrap();

        let retrieved = storage.get_agent("test_agent_001").await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.agent_id, agent.agent_id);
        assert_eq!(retrieved.use_case_class, agent.use_case_class);
        assert_eq!(retrieved.industry_class, agent.industry_class);
    }

    #[tokio::test]
    async fn test_list_agents_by_org() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        storage.migrate().await.unwrap();

        let agent1 = Agent {
            agent_id: "agent_1".to_string(),
            organization_id: Some("org_a".to_string()),
            ..Default::default()
        };
        let agent2 = Agent {
            agent_id: "agent_2".to_string(),
            organization_id: Some("org_a".to_string()),
            ..Default::default()
        };
        let agent3 = Agent {
            agent_id: "agent_3".to_string(),
            organization_id: Some("org_b".to_string()),
            ..Default::default()
        };

        storage.store_agent(&agent1).await.unwrap();
        storage.store_agent(&agent2).await.unwrap();
        storage.store_agent(&agent3).await.unwrap();

        let org_a_agents = storage.list_agents(Some("org_a")).await.unwrap();
        assert_eq!(org_a_agents.len(), 2);

        let all_agents = storage.list_agents(None).await.unwrap();
        assert_eq!(all_agents.len(), 3);
    }

    #[tokio::test]
    async fn test_exposure_roundtrip() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        storage.migrate().await.unwrap();

        // First create an agent
        let agent = Agent {
            agent_id: "test_agent".to_string(),
            ..Default::default()
        };
        storage.store_agent(&agent).await.unwrap();

        let exposure = ExposurePeriod {
            exposure_id: "exp_001".to_string(),
            agent_id: "test_agent".to_string(),
            period_start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            period_end: NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
            agent_active_hours: Decimal::from_str("160.5").unwrap(),
            actions_executed_total: 1000,
            actions_executed_high_risk: 50,
            ..Default::default()
        };

        storage.store_exposure(&exposure).await.unwrap();

        let retrieved = storage.get_exposures("test_agent", None, None).await.unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].exposure_id, exposure.exposure_id);
        assert_eq!(retrieved[0].actions_executed_total, 1000);
    }

    #[tokio::test]
    async fn test_incident_and_claim() {
        let storage = SqliteStorage::in_memory().await.unwrap();
        storage.migrate().await.unwrap();

        // Create agent
        let agent = Agent {
            agent_id: "test_agent".to_string(),
            ..Default::default()
        };
        storage.store_agent(&agent).await.unwrap();

        // Create incident
        let incident = Incident {
            incident_id: "inc_001".to_string(),
            agent_id: "test_agent".to_string(),
            incident_class: IncidentClass::DataBreachOrLeakage,
            severity_level: SeverityLevel::High,
            ..Default::default()
        };
        storage.store_incident(&incident).await.unwrap();

        // Create claim
        let claim = ClaimDevelopment {
            claim_id: "claim_001".to_string(),
            incident_id: "inc_001".to_string(),
            valuation_date: NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
            paid_amount_cumulative: Decimal::from_str("50000.00").unwrap(),
            reserve_amount: Decimal::from_str("100000.00").unwrap(),
            expected_ultimate_amount: Decimal::from_str("150000.00").unwrap(),
            status: ClaimStatus::Open,
            ..Default::default()
        };
        storage.store_claim(&claim).await.unwrap();

        let incidents = storage.get_incidents("test_agent", None, None).await.unwrap();
        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].incident_class, IncidentClass::DataBreachOrLeakage);

        let claims = storage.get_claims("inc_001", None).await.unwrap();
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].status, ClaimStatus::Open);
    }
}
