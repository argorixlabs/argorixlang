use crate::{AdapterContract, Provider, ProviderError, ProviderKind, SimulatedProvider};
use std::collections::HashMap;

pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn Provider>>,
    contracts: HashMap<String, AdapterContract>,
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        let mut registry = Self {
            providers: HashMap::new(),
            contracts: HashMap::new(),
        };
        registry
            .register(SimulatedProvider)
            .expect("default simulated provider must register");
        registry
    }
}

impl ProviderRegistry {
    pub fn empty() -> Self {
        Self {
            providers: HashMap::new(),
            contracts: HashMap::new(),
        }
    }

    pub fn register<P>(&mut self, provider: P) -> Result<(), ProviderError>
    where
        P: Provider + 'static,
    {
        let name = provider.name().to_owned();
        if name != "simulated" || provider.kind() != ProviderKind::Simulated {
            return Err(ProviderError::ExecutableProviderForbidden(name));
        }
        if self.providers.contains_key(&name) {
            return Err(ProviderError::DuplicateProvider(name));
        }
        self.providers.insert(name, Box::new(provider));
        Ok(())
    }

    pub fn get(&self, name: &str) -> Result<&dyn Provider, ProviderError> {
        self.providers
            .get(name)
            .map(Box::as_ref)
            .ok_or_else(|| ProviderError::UnknownProvider(name.to_owned()))
    }

    pub fn register_contract(&mut self, contract: AdapterContract) -> Result<(), ProviderError> {
        if self.providers.contains_key(&contract.name) {
            return Err(ProviderError::ExecutableProviderName(contract.name));
        }
        if self.contracts.contains_key(&contract.name) {
            return Err(ProviderError::DuplicateContract(contract.name));
        }
        self.contracts.insert(contract.name.clone(), contract);
        Ok(())
    }

    pub fn get_contract(&self, name: &str) -> Result<&AdapterContract, ProviderError> {
        self.contracts
            .get(name)
            .ok_or_else(|| ProviderError::UnknownContract(name.to_owned()))
    }

    pub fn validate_contract(&self, name: &str) -> Result<&AdapterContract, ProviderError> {
        let contract = self.get_contract(name)?;
        let reason = if contract.name.trim().is_empty() {
            Some("provider contract name must not be empty")
        } else {
            match contract.kind {
                ProviderKind::Simulated => Some("only external contracts are declarative in v0.12"),
                ProviderKind::External if contract.enabled => {
                    Some("external provider contracts must be disabled")
                }
                ProviderKind::External if !contract.dry_run_only => {
                    Some("external provider contracts must be dry-run-only")
                }
                ProviderKind::External if !contract.requires_feature_flag => {
                    Some("external provider contracts require a feature flag")
                }
                ProviderKind::External if !contract.requires_explicit_approval => {
                    Some("external provider contracts require explicit approval")
                }
                ProviderKind::External => None,
            }
        };
        if let Some(reason) = reason {
            return Err(ProviderError::InvalidContract {
                name: contract.name.clone(),
                reason: reason.into(),
            });
        }
        Ok(contract)
    }

    pub fn is_enabled(&self, name: &str) -> Result<bool, ProviderError> {
        if self.providers.contains_key(name) {
            return Ok(true);
        }
        Ok(self.get_contract(name)?.enabled)
    }

    pub fn contract_entries(&self) -> Vec<&AdapterContract> {
        let mut entries = self.contracts.values().collect::<Vec<_>>();
        entries.sort_unstable_by_key(|contract| contract.name.as_str());
        entries
    }

    pub fn contains_contract(&self, name: &str) -> bool {
        self.contracts.contains_key(name)
    }

    pub fn execution_registry(&self) -> Self {
        let mut registry = Self::empty();
        if self.contains("simulated") {
            registry
                .register(SimulatedProvider)
                .expect("simulated provider must register in execution registry");
        }
        registry
    }

    pub fn contains(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }

    pub fn names(&self) -> Vec<&str> {
        let mut names = self
            .providers
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        names.sort_unstable();
        names
    }

    pub fn entries(&self) -> Vec<(&str, ProviderKind)> {
        let mut entries = self
            .providers
            .values()
            .map(|provider| (provider.name(), provider.kind()))
            .collect::<Vec<_>>();
        entries.sort_unstable_by_key(|(name, _)| *name);
        entries
    }
}
