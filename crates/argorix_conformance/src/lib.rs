pub mod mutation;
pub mod runner;
pub mod types;
pub mod validation;
pub mod workspace;

pub use runner::run_suite;
pub use validation::{resolve_fixture_path, validate_suite, ConformanceValidationError};

#[derive(Debug, thiserror::Error)]
pub enum ConformanceError {
    #[error(transparent)]
    Validation(#[from] ConformanceValidationError),
    #[error("conformance workspace error: {0}")]
    Workspace(String),
    #[error("conformance JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
