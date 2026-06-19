use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProviderError {
    #[error("provider calls require dry_run=true in Argorix v0.15")]
    DryRunRequired,
    #[error("provider `{0}` is already registered")]
    DuplicateProvider(String),
    #[error("provider `{0}` is not registered")]
    UnknownProvider(String),
    #[error("provider `{0}` cannot be registered as executable; only `simulated` is allowed")]
    ExecutableProviderForbidden(String),
    #[error("provider contract `{0}` is already registered")]
    DuplicateContract(String),
    #[error("provider contract `{0}` collides with an executable provider")]
    ExecutableProviderName(String),
    #[error("provider contract `{0}` is not registered")]
    UnknownContract(String),
    #[error("provider contract `{name}` is invalid: {reason}")]
    InvalidContract { name: String, reason: String },
}
