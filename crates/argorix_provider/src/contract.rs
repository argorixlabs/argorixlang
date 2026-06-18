use crate::ProviderKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterContract {
    pub name: String,
    pub kind: ProviderKind,
    pub enabled: bool,
    pub dry_run_only: bool,
    pub requires_feature_flag: bool,
    pub requires_explicit_approval: bool,
    #[serde(default)]
    pub allowed_targets: Vec<String>,
    #[serde(default)]
    pub allowed_capabilities: Vec<String>,
}
