mod contract;
mod errors;
mod provider;
mod registry;
mod simulated;

pub use contract::AdapterContract;
pub use errors::ProviderError;
pub use provider::{
    ModelProviderRequest, ModelProviderResponse, Provider, ProviderCallStatus, ProviderKind,
    ToolProviderRequest, ToolProviderResponse,
};
pub use registry::ProviderRegistry;
pub use simulated::SimulatedProvider;

#[cfg(test)]
mod tests {
    use crate::{
        AdapterContract, ModelProviderRequest, Provider, ProviderCallStatus, ProviderError,
        ProviderKind, ProviderRegistry, SimulatedProvider, ToolProviderRequest,
    };

    fn external_contract(name: &str) -> AdapterContract {
        AdapterContract {
            name: name.into(),
            kind: ProviderKind::External,
            enabled: false,
            dry_run_only: true,
            requires_feature_flag: true,
            requires_explicit_approval: true,
            allowed_targets: vec![],
            allowed_capabilities: vec![],
        }
    }

    #[test]
    fn registry_contains_simulated_by_default() {
        let registry = ProviderRegistry::default();
        assert!(registry.contains("simulated"));
        assert_eq!(registry.names(), vec!["simulated"]);
    }

    #[test]
    fn empty_registry_has_no_providers() {
        let registry = ProviderRegistry::empty();
        assert!(!registry.contains("simulated"));
        assert!(registry.names().is_empty());
    }

    #[test]
    fn registry_stores_and_validates_external_contract_separately() {
        let mut registry = ProviderRegistry::default();
        registry
            .register_contract(external_contract("OpenAI"))
            .unwrap();

        assert_eq!(
            registry.get_contract("OpenAI").unwrap().kind,
            ProviderKind::External
        );
        assert_eq!(registry.validate_contract("OpenAI").unwrap().name, "OpenAI");
        assert!(!registry.is_enabled("OpenAI").unwrap());
        assert!(matches!(
            registry.get("OpenAI"),
            Err(ProviderError::UnknownProvider(name)) if name == "OpenAI"
        ));
    }

    #[test]
    fn registry_rejects_duplicate_and_executable_contract_names() {
        let mut registry = ProviderRegistry::default();
        registry
            .register_contract(external_contract("OpenAI"))
            .unwrap();
        assert_eq!(
            registry
                .register_contract(external_contract("OpenAI"))
                .unwrap_err(),
            ProviderError::DuplicateContract("OpenAI".into())
        );
        assert_eq!(
            registry
                .register_contract(external_contract("simulated"))
                .unwrap_err(),
            ProviderError::ExecutableProviderName("simulated".into())
        );
    }

    struct ExternalExecutable;

    impl Provider for ExternalExecutable {
        fn name(&self) -> &str {
            "external-executable"
        }

        fn kind(&self) -> ProviderKind {
            ProviderKind::External
        }

        fn invoke_model(
            &self,
            _request: ModelProviderRequest,
        ) -> Result<crate::ModelProviderResponse, ProviderError> {
            unreachable!("external executable must never be invoked")
        }

        fn invoke_tool(
            &self,
            _request: ToolProviderRequest,
        ) -> Result<crate::ToolProviderResponse, ProviderError> {
            unreachable!("external executable must never be invoked")
        }
    }

    #[test]
    fn registry_rejects_external_executable_provider_implementations() {
        let mut registry = ProviderRegistry::empty();
        assert_eq!(
            registry.register(ExternalExecutable).unwrap_err(),
            ProviderError::ExecutableProviderForbidden("external-executable".into())
        );
        assert!(registry.names().is_empty());
    }

    #[test]
    fn external_contract_validation_enforces_v012_invariants() {
        let cases = [
            (
                AdapterContract {
                    name: String::new(),
                    ..external_contract("ignored")
                },
                "name",
            ),
            (
                AdapterContract {
                    enabled: true,
                    ..external_contract("Enabled")
                },
                "must be disabled",
            ),
            (
                AdapterContract {
                    dry_run_only: false,
                    ..external_contract("Live")
                },
                "dry-run-only",
            ),
            (
                AdapterContract {
                    requires_feature_flag: false,
                    ..external_contract("NoFlag")
                },
                "feature flag",
            ),
            (
                AdapterContract {
                    requires_explicit_approval: false,
                    ..external_contract("NoApproval")
                },
                "explicit approval",
            ),
        ];

        for (contract, expected) in cases {
            let mut registry = ProviderRegistry::empty();
            let name = contract.name.clone();
            registry.register_contract(contract).unwrap();
            let error = registry.validate_contract(&name).unwrap_err();
            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[test]
    fn registry_accepts_populated_allowlists_without_interpreting_them() {
        let mut registry = ProviderRegistry::default();
        let mut contract = external_contract("OpenAI");
        contract.allowed_targets = vec!["GuardModel".into()];
        contract.allowed_capabilities = vec!["model.invoke".into()];
        registry.register_contract(contract).unwrap();
        let validated = registry.validate_contract("OpenAI").unwrap();
        assert_eq!(validated.allowed_targets, vec!["GuardModel"]);
        assert_eq!(validated.allowed_capabilities, vec!["model.invoke"]);
    }
    #[test]
    fn simulated_provider_invokes_tool_in_dry_run() {
        let response = SimulatedProvider
            .invoke_tool(ToolProviderRequest {
                call_id: "tool_001".into(),
                agent: "ResearchAgent".into(),
                tool: "WebSearch".into(),
                input_type: "UserPrompt".into(),
                output_type: "ToolResult".into(),
                input_binding: "prompt".into(),
                dry_run: true,
            })
            .unwrap();
        assert_eq!(response.call_id, "tool_001");
        assert_eq!(response.status, ProviderCallStatus::Allowed);
        assert_eq!(response.output_type, "ToolResult");
        assert!(response.simulated);
    }

    #[test]
    fn simulated_provider_invokes_model_in_dry_run() {
        let response = SimulatedProvider
            .invoke_model(ModelProviderRequest {
                call_id: "model_001".into(),
                agent: "PolicyJudge".into(),
                model: "GuardModel".into(),
                input_type: "ToolResult".into(),
                output_type: "Decision".into(),
                input_binding: "result".into(),
                dry_run: true,
            })
            .unwrap();
        assert_eq!(response.call_id, "model_001");
        assert_eq!(response.status, ProviderCallStatus::Allowed);
        assert_eq!(response.output_type, "Decision");
        assert!(response.simulated);
    }

    #[test]
    fn simulated_provider_rejects_non_dry_run() {
        let error = SimulatedProvider
            .invoke_tool(ToolProviderRequest {
                call_id: "tool_001".into(),
                agent: "ResearchAgent".into(),
                tool: "WebSearch".into(),
                input_type: "UserPrompt".into(),
                output_type: "ToolResult".into(),
                input_binding: "prompt".into(),
                dry_run: false,
            })
            .unwrap_err();
        assert_eq!(error, ProviderError::DryRunRequired);
    }
}
