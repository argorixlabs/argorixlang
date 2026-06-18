use crate::{
    ModelProviderRequest, ModelProviderResponse, Provider, ProviderCallStatus, ProviderError,
    ProviderKind, ToolProviderRequest, ToolProviderResponse,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct SimulatedProvider;

impl Provider for SimulatedProvider {
    fn name(&self) -> &str {
        "simulated"
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Simulated
    }

    fn invoke_model(
        &self,
        request: ModelProviderRequest,
    ) -> Result<ModelProviderResponse, ProviderError> {
        if !request.dry_run {
            return Err(ProviderError::DryRunRequired);
        }
        Ok(ModelProviderResponse {
            call_id: request.call_id,
            status: ProviderCallStatus::Allowed,
            output_type: request.output_type,
            simulated: true,
        })
    }

    fn invoke_tool(
        &self,
        request: ToolProviderRequest,
    ) -> Result<ToolProviderResponse, ProviderError> {
        if !request.dry_run {
            return Err(ProviderError::DryRunRequired);
        }
        Ok(ToolProviderResponse {
            call_id: request.call_id,
            status: ProviderCallStatus::Allowed,
            output_type: request.output_type,
            simulated: true,
        })
    }
}
