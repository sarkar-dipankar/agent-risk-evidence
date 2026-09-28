//! Storage Layer for Agent Insure
//!
//! This crate provides storage abstractions for persisting Agent Insure data:
//!
//! - Time-series metrics (exposure periods)
//! - Event store (incidents, claims)
//! - Attestation manifests
//!
//! # Example
//!
//! ```rust,ignore
//! use ai_storage::{Storage, PostgresStorage};
//!
//! let storage = PostgresStorage::connect(&database_url).await?;
//! storage.store_exposure(&exposure).await?;
//! ```

pub mod error;
pub mod traits;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

pub use error::StorageError;
pub use traits::Storage;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteStorage;

#[cfg(feature = "postgres")]
pub use postgres::PostgresStorage;
