//! Storage trait definitions

use crate::StorageError;
use ai_schema::{Agent, ClaimDevelopment, ControlSnapshot, ExposurePeriod, Incident};
use chrono::NaiveDate;

/// Main storage trait for Agent Insure data
#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    // Agent operations
    async fn store_agent(&self, agent: &Agent) -> Result<(), StorageError>;
    async fn get_agent(&self, agent_id: &str) -> Result<Option<Agent>, StorageError>;
    async fn list_agents(&self, organization_id: Option<&str>)
        -> Result<Vec<Agent>, StorageError>;

    // Exposure operations
    async fn store_exposure(&self, exposure: &ExposurePeriod) -> Result<(), StorageError>;
    async fn get_exposures(
        &self,
        agent_id: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<ExposurePeriod>, StorageError>;

    // Control operations
    async fn store_control(&self, control: &ControlSnapshot) -> Result<(), StorageError>;
    async fn get_controls(
        &self,
        agent_id: &str,
        effective_date: Option<NaiveDate>,
    ) -> Result<Vec<ControlSnapshot>, StorageError>;

    // Incident operations
    async fn store_incident(&self, incident: &Incident) -> Result<(), StorageError>;
    async fn get_incidents(
        &self,
        agent_id: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<Incident>, StorageError>;

    // Claim operations
    async fn store_claim(&self, claim: &ClaimDevelopment) -> Result<(), StorageError>;
    async fn get_claims(
        &self,
        incident_id: &str,
        valuation_date: Option<NaiveDate>,
    ) -> Result<Vec<ClaimDevelopment>, StorageError>;
}
