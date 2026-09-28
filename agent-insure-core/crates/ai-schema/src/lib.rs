//! Agent Insure Schema Types and Validation
//!
//! This crate provides the core data types for the Agent Insure standard:
//!
//! - [`Agent`] - AI agent profile and classification
//! - [`ExposurePeriod`] - Time-bounded activity metrics
//! - [`ControlSnapshot`] - Security posture at a point in time
//! - [`Incident`] - Loss events and near-misses
//! - [`ClaimDevelopment`] - Financial development of claims
//!
//! # Example
//!
//! ```rust
//! use ai_schema::{Agent, UseCaseClass, IndustryClass};
//!
//! let agent = Agent {
//!     agent_id: "agent_001".to_string(),
//!     use_case_class: UseCaseClass::CustomerSupport,
//!     industry_class: IndustryClass::FinancialServices,
//!     ..Default::default()
//! };
//! ```

pub mod entities;
pub mod enums;
pub mod error;
pub mod validation;

pub use entities::*;
pub use enums::*;
pub use error::SchemaError;
pub use validation::Validator;

/// Schema version constant
pub const SCHEMA_VERSION: &str = "1.0.0";
