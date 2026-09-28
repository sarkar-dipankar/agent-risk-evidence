//! Enumeration types for Agent Insure schema

use serde::{Deserialize, Serialize};

/// Use case classification for AI agents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UseCaseClass {
    CustomerSupport,
    CodeGeneration,
    DataAnalysis,
    FinancialOperations,
    ContentCreation,
    ResearchAssistant,
    WorkflowAutomation,
    Other,
}

/// Industry classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndustryClass {
    FinancialServices,
    Healthcare,
    Technology,
    Retail,
    Manufacturing,
    ProfessionalServices,
    Government,
    Education,
    Other,
}

/// Autonomy level of AI agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyLevel {
    /// Human reviews every action
    Assistive,
    /// Human approves high-risk actions
    SemiAutonomous,
    /// Acts independently
    Autonomous,
}

/// Incident classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentClass {
    DataBreachOrLeakage,
    UnauthorizedAction,
    FinancialLoss,
    ComplianceViolation,
    ServiceDisruption,
    ReputationalHarm,
    ModelFailure,
    Other,
}

/// Severity level of incidents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeverityLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Line of business for insurance coverage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineOfBusiness {
    CyberSecurity,
    FinancialCrime,
    ProfessionalLiability,
    OperationalInterruption,
    RegulatoryFines,
    Other,
}

/// Claim status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Open,
    Closed,
    Reopened,
}

/// Geographic region
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeographicRegion {
    NorthAmerica,
    Europe,
    AsiaPacific,
    LatinAmerica,
    MiddleEast,
    Africa,
    Global,
}

impl Default for UseCaseClass {
    fn default() -> Self {
        Self::Other
    }
}

impl Default for IndustryClass {
    fn default() -> Self {
        Self::Other
    }
}

impl Default for AutonomyLevel {
    fn default() -> Self {
        Self::SemiAutonomous
    }
}

impl Default for IncidentClass {
    fn default() -> Self {
        Self::Other
    }
}

impl Default for SeverityLevel {
    fn default() -> Self {
        Self::Medium
    }
}

impl Default for LineOfBusiness {
    fn default() -> Self {
        Self::Other
    }
}

impl Default for ClaimStatus {
    fn default() -> Self {
        Self::Open
    }
}

impl Default for GeographicRegion {
    fn default() -> Self {
        Self::Global
    }
}
