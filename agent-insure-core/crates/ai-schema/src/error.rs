//! Error types for schema validation

use thiserror::Error;

/// Schema-related errors
#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Unknown entity type: {0}")]
    UnknownEntityType(String),

    #[error("Schema version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },

    #[error("Required field missing: {0}")]
    MissingField(String),

    #[error("Invalid field value for {field}: {message}")]
    InvalidFieldValue { field: String, message: String },

    #[error("Privacy tier violation: {0}")]
    PrivacyViolation(String),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}
