# Argorix Lang v0.11 External Adapter Contracts Design

## Objective

Argorix Lang v0.11 introduces formal external adapter contracts without
permitting external execution:

```text
Provider Boundary -> Adapter Contract -> Conformance Check -> Fail-Closed Runtime
```

The release performs no network access, reads no secrets or environment
variables, and integrates no real model or tool provider. `simulated` remains
the only executable provider.

## Provider Architecture

The `argorix_provider` crate defines:

```rust
pub struct AdapterContract {
    pub name: String,
    pub kind: ProviderKind,
    pub enabled: bool,
    pub dry_run_only: bool,
    pub requires_feature_flag: bool,
    pub requires_explicit_approval: bool,
    pub allowed_targets: Vec<String>,
    pub allowed_capabilities: Vec<String>,
}
```

`ProviderKind` has `Simulated` and `External` variants. `SimulatedProvider`
continues implementing `Provider`.

`ProviderRegistry` separates executable providers from declarative contracts:

```rust
pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn Provider>>,
    contracts: HashMap<String, AdapterContract>,
}
```

The executable map contains only `simulated`. External contracts never
implement `Provider` and cannot be returned by the executable-provider lookup.
The contract map supports `register_contract`, `get_contract`,
`validate_contract`, and `is_enabled`.

Registration rejects duplicate contract names and collisions with executable
provider names. In particular, source code cannot declare a contract named
`simulated`.

External contract validation requires:

- `kind == External`
- `enabled == false`
- `dry_run_only == true`
- `requires_feature_flag == true`
- `requires_explicit_approval == true`
- empty `allowed_targets`
- empty `allowed_capabilities`

The two `allowed_*` fields are reserved for v0.12 or later. They exist in Rust,
IR, Bytecode, and JSON for structural stability but have no public v0.11
language syntax.

## Source Language and AST

Providers are optional module-level declarations:

```argx
provider OpenAI {
    kind external
    enabled false
    dry_run_only true
    requires feature_flag
    requires approval
}
```

The parser accepts the fields in the order shown. Boolean fields accept only
`true` or `false`. The AST stores the provider name, kind, booleans, and the
two requirement flags. It does not expose `allowed_targets` or
`allowed_capabilities`.

The parser does not accept public `allowed_targets` or `allowed_capabilities`
blocks in v0.11.

## Semantic Validation

The semantic checker:

- accepts disabled external provider contracts that satisfy every required
  invariant;
- rejects duplicate provider declarations;
- rejects a provider declaration named `simulated`;
- rejects unsupported provider kinds;
- rejects external contracts with `enabled true`;
- rejects external contracts without `dry_run_only true`;
- rejects external contracts without `requires feature_flag`;
- rejects external contracts without `requires approval`;
- continues rejecting tools and models whose executable provider is not
  `simulated`, including names of valid external contracts.

The checker does not validate target or capability allowlists because those
lists have no v0.11 source syntax.

## IR 0.11

IR version `0.11` adds a top-level `providers` collection before assertions:

```json
{
  "ir_version": "0.11",
  "language": "Argorix Lang",
  "module": "Argorix.ProviderContracts",
  "providers": [],
  "assertions": [],
  "failures": [],
  "capabilities": [],
  "tools": [],
  "models": [],
  "agents": [],
  "protocols": []
}
```

Each provider declaration lowers to:

```json
{
  "name": "OpenAI",
  "kind": "external",
  "enabled": false,
  "dry_run_only": true,
  "requires_feature_flag": true,
  "requires_explicit_approval": true,
  "allowed_targets": [],
  "allowed_capabilities": []
}
```

Lowering always supplies empty arrays for the two reserved fields.

## Bytecode 0.11

Bytecode version `0.11` adds a top-level `providers` collection containing the
same stable contract representation as IR. It also adds:

```json
{
  "op": "DeclareProviderContract",
  "name": "OpenAI",
  "kind": "external",
  "enabled": false,
  "dry_run_only": true,
  "requires_feature_flag": true,
  "requires_explicit_approval": true,
  "allowed_targets": [],
  "allowed_capabilities": []
}
```

`DeclareProviderContract` instructions appear before tool, model, agent, and
protocol declarations.

The verifier accepts versions `0.3`, `0.5`, `0.6`, `0.7`, `0.8`, `0.9`,
`0.10`, and `0.11`. Existing bytecode without a provider-contract collection
deserializes with an empty collection. For v0.11 contracts, the verifier
checks duplicate names, executable-name collisions, external-contract
invariants, empty reserved lists, and matching declaration instructions.
Tools and models remain restricted to `simulated`.

## VM Loading and Execution

Before reactive execution, the VM creates an execution-local registry from
the default executable registry. It loads each bytecode contract into the
contract map and validates it before processing injected messages.

For every valid contract, the ledger records:

1. `ProviderContractDeclared`
2. `ProviderContractValidated`

An invalid contract records `ProviderContractRejected`, transitions runtime
state to `failed`, activates an applicable failure mode, and returns an error
without discarding the trace ledger.

Provider selection for tools and models consults only the executable map. If a
requested provider name exists only in the contract map, the runtime records
`ExternalProviderExecutionBlocked`, activates an applicable failure mode,
transitions to `failed`, and returns an error while preserving the ledger.

The source compiler and bytecode verifier normally prevent this execution
path. The runtime check remains mandatory because manually authored or
mutated bytecode must also fail closed.

Bytecode v0.10 and older has no contracts and continues executing with only
the `simulated` provider.

## Audit and Output Contracts

The event model adds:

- `ProviderContractDeclared`
- `ProviderContractValidated`
- `ProviderContractRejected`
- `ExternalProviderExecutionBlocked`

Reactive JSON uses `vm_version: "0.11"`. Its `providers` field lists executable
providers only:

```json
{
  "name": "simulated",
  "kind": "simulated",
  "enabled": true
}
```

The new `provider_contracts` field lists declarative external contracts,
including both reserved empty arrays.

The CLI keeps all existing flags and adds `--provider-contracts`. Text output
separates executable providers from declarative contracts and states:

```text
External provider execution: blocked by design
```

`--json` always includes the machine-readable contract collection. The text
contract report is shown only when `--provider-contracts` is requested.

## Fixtures

The valid fixture `examples/provider_contracts_v011.argx` extends the v0.10
provider-boundary example with a disabled `OpenAI` external contract while
keeping every tool and model on `simulated`. Its generated bytecode fixture is
`examples/provider_contracts_v011.argbc.json`.

Invalid fixtures cover:

- external provider enabled;
- missing feature-flag requirement;
- missing approval requirement;
- model using an external provider;
- tool using an external provider.

Duplicate declarations and invalid dry-run configuration are covered directly
by parser or semantic unit tests.

## Test Strategy

Tests follow red-green-refactor and cover:

- every provider declaration token and field parsed into the AST;
- accepted and rejected semantic contract configurations;
- duplicate and executable-name collision rejection;
- model and tool external-provider rejection;
- IR 0.11 contract serialization and empty reserved arrays;
- Bytecode 0.11 contract serialization and
  `DeclareProviderContract`;
- registry registration, lookup, validation, enablement queries, and strict
  separation of executable providers from contracts;
- VM loading and validation events;
- rejected-contract and blocked-execution events with preserved ledgers;
- JSON `provider_contracts`;
- CLI `--provider-contracts`;
- successful v0.10 bytecode verification and execution.

Final verification runs all required compiler and VM examples, confirms the
invalid examples exit nonzero, and then runs:

```bash
cargo fmt
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Documentation and Versioning

Workspace packages, compiler output, VM output, IR, and newly emitted bytecode
advance to v0.11. README architecture, commands, examples, Bytecode
instructions, compatibility guarantees, and roadmap are updated accordingly.

The release statement is:

```text
v0.10 created the provider boundary.
v0.11 defines external adapter contracts without executing them.
v0.12+ may constrain targets and capabilities.
```

The implementation preserves the project principle:

```text
Rust is the forge.
Argorix Lang is the sword.
```
