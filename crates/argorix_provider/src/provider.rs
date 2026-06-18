use crate::ProviderError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Simulated,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderCallStatus {
    Allowed,
    Denied,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelProviderRequest {
    pub call_id: String,
    pub agent: String,
    pub model: String,
    pub input_type: String,
    pub output_type: String,
    pub input_binding: String,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelProviderResponse {
    pub call_id: String,
    pub status: ProviderCallStatus,
    pub output_type: String,
    pub simulated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolProviderRequest {
    pub call_id: String,
    pub agent: String,
    pub tool: String,
    pub input_type: String,
    pub output_type: String,
    pub input_binding: String,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolProviderResponse {
    pub call_id: String,
    pub status: ProviderCallStatus,
    pub output_type: String,
    pub simulated: bool,
}

pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> ProviderKind;
    fn invoke_model(
        &self,
        request: ModelProviderRequest,
    ) -> Result<ModelProviderResponse, ProviderError>;
    fn invoke_tool(
        &self,
        request: ToolProviderRequest,
    ) -> Result<ToolProviderResponse, ProviderError>;
}
