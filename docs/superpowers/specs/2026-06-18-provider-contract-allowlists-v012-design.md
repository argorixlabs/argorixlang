# Argorix Lang v0.12 Provider Contract Allowlists Design

## Objective

Argorix Lang v0.12 activates the provider-contract fields reserved by v0.11:

```text
External Adapter Contract
        -> allowed_targets / allowed_capabilities
        -> Semantic Validation
        -> Argorix IR 0.12
        -> Argorix Bytecode 0.12
        -> VM Contract Validation
        -> Provider Contract Report
```

The release remains fully offline. It performs no network access, reads no
secrets or environment variables, and integrates no real model or tool
provider. External providers remain disabled declarative contracts.
`simulated` remains the only executable provider.

The defining rule is:

```text
Allowlisted does not mean executable.
```

## Source Syntax

Provider contracts may declare optional target and capability allowlists:

```argx
provider OpenAI {
    kind external
    enabled false
    dry_run_only true
    requires feature_flag
    requires approval

    allowed_targets {
        GuardModel
    }

    allowed_capabilities {
        model.invoke
    }
}
```

The fixed provider prefix remains:

1. `kind`
2. `enabled`
3. `dry_run_only`
4. `requires feature_flag`
5. `requires approval`

After the requirement clauses, `allowed_targets` and
`allowed_capabilities` may appear in either order. Each block is optional and
may appear at most once. A duplicate block is a parser error.

`allowed_targets` accepts simple identifiers naming global tools or models.
`allowed_capabilities` accepts capability identifiers, including dotted names
such as `model.invoke`, `web.search`, and `file.read`.

The blocks are legal only inside a provider declaration. Their absence
produces empty arrays and preserves v0.11 source compatibility.

## AST

`ProviderDecl` adds:

```rust
pub allowed_targets: Vec<Spanned<String>>,
pub allowed_capabilities: Vec<Spanned<String>>,
```

The parser preserves source order, values, and spans exactly. It does not
deduplicate list elements. Duplicate elements remain in the AST so semantic
diagnostics can point to the repeated occurrence.

## Semantic Validation

The compiler keeps existing provider invariants:

- provider names are unique;
- `simulated` cannot be declared as a contract;
- contracts use `kind external`;
- external contracts have `enabled false`;
- external contracts have `dry_run_only true`;
- external contracts require `feature_flag`;
- external contracts require explicit approval;
- tools and models can execute only through `simulated`.

Allowlist validation operates directly against `Program.tools`,
`Program.models`, and the global capability registry. `Symbols` is not
expanded and no separate allowlist-policy module is introduced.

For every provider contract:

- duplicate entries in `allowed_targets` are rejected at the duplicate span;
- duplicate entries in `allowed_capabilities` are rejected at the duplicate
  span;
- each target must resolve to a global tool or model;
- a name resolving to both a tool and a model is rejected as
  `ambiguous allowlist target`;
- each allowed capability must exist in the global capability registry;
- when `allowed_capabilities` is non-empty, every allowed target's declared
  capability must occur in that list;
- model compatibility uses the model declaration's capability;
- tool compatibility uses the tool declaration's capability.

An empty allowlist means zero future permissions. It is never a wildcard.
Therefore an empty `allowed_capabilities` list does not create a compatibility
error for an otherwise valid target, but it grants no future permission.

Allowlisting an external target never authorizes execution. A model or tool
whose provider is an external contract remains a semantic error even when its
name and capability appear in that contract's allowlists.

## IR 0.12

Newly compiled programs emit IR version `0.12`:

```json
{
  "ir_version": "0.12",
  "language": "Argorix Lang",
  "module": "Argorix.ProviderAllowlists",
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

The top-level `providers` collection continues representing declarative
provider contracts, not executable provider instances.

Lowering preserves allowlist order and contents:

```json
{
  "name": "OpenAI",
  "kind": "external",
  "enabled": false,
  "dry_run_only": true,
  "requires_feature_flag": true,
  "requires_explicit_approval": true,
  "allowed_targets": ["GuardModel"],
  "allowed_capabilities": ["model.invoke"]
}
```

Contracts without source allowlist blocks lower to empty arrays.

## Bytecode 0.12

Newly compiled programs emit Bytecode version `0.12`.
`DeclareProviderContract` preserves both allowlists:

```json
{
  "op": "DeclareProviderContract",
  "name": "OpenAI",
  "kind": "external",
  "enabled": false,
  "dry_run_only": true,
  "requires_feature_flag": true,
  "requires_explicit_approval": true,
  "allowed_targets": ["GuardModel"],
  "allowed_capabilities": ["model.invoke"]
}
```

The verifier accepts Bytecode versions already supported by v0.11 plus
`0.12`.

For Bytecode `0.11`:

- provider-contract allowlists must remain empty;
- existing v0.11 fixtures continue to verify and execute.

For Bytecode `0.12`, the verifier:

- permits populated allowlists;
- rejects duplicate provider contracts;
- rejects duplicate target entries;
- rejects duplicate capability entries;
- rejects unknown targets;
- rejects ambiguous targets that name both a tool and a model;
- rejects unknown capabilities;
- rejects target/capability incompatibility;
- retains simulated-only tool/model provider validation;
- requires exact correspondence between the top-level `providers` collection
  and every `DeclareProviderContract` instruction, including order and list
  contents.

Verifier compatibility uses global Bytecode tool, model, and capability
declarations. It does not delegate language-level resolution to
`ProviderRegistry`.

## Provider Registry

`ProviderRegistry` retains separate maps:

```rust
pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn Provider>>,
    contracts: HashMap<String, AdapterContract>,
}
```

Only `simulated` may be registered as executable. External contracts never
implement `Provider`.

`validate_contract()` continues validating structural invariants:

- non-empty contract name;
- `kind == External`;
- `enabled == false`;
- `dry_run_only == true`;
- `requires_feature_flag == true`;
- `requires_explicit_approval == true`.

It no longer requires `allowed_targets` or `allowed_capabilities` to be empty.
It accepts populated lists without interpreting them. The registry has no
dependency on AST, IR, Bytecode, tools, models, or the language capability
registry.

## VM and Fail-Closed Runtime

Before reactive execution:

1. Bytecode verification passes.
2. Provider contracts are loaded into an execution-local registry.
3. Registry structural validation passes.
4. Reactive scheduling begins.

The VM copies allowlists without transforming or reordering them. It retains
the v0.11 audit events:

- `ProviderContractDeclared`
- `ProviderContractValidated`
- `ProviderContractRejected`
- `ExternalProviderExecutionBlocked`

Reactive output advances to `vm_version: "0.12"`.

If a tool or model attempts to use a name that exists only as an external
contract, runtime records `ExternalProviderExecutionBlocked`, activates the
applicable failure mode, transitions to failed, and preserves the trace
ledger. This remains true even when the target and capability are validly
allowlisted.

Bytecode v0.11 continues loading and executing with empty contract allowlists.

## CLI and JSON

All existing flags remain available. `--provider-contracts` prints contract
allowlists on indented lines:

```text
Provider contracts:
- OpenAI: external, disabled, dry-run-only, requires feature_flag, requires approval
  allowed_targets: GuardModel
  allowed_capabilities: model.invoke

External provider execution: blocked by design
```

Empty lists print `none`:

```text
  allowed_targets: none
  allowed_capabilities: none
```

CLI documentation states that `none` means zero future permissions, not a
wildcard.

Reactive JSON preserves array order:

```json
{
  "vm_version": "0.12",
  "provider_contracts": [
    {
      "name": "OpenAI",
      "kind": "external",
      "enabled": false,
      "dry_run_only": true,
      "requires_feature_flag": true,
      "requires_explicit_approval": true,
      "allowed_targets": ["GuardModel"],
      "allowed_capabilities": ["model.invoke"]
    }
  ]
}
```

The JSON `providers` field continues listing executable runtime providers
only.

## Fixtures

Valid source and generated Bytecode fixtures:

- `examples/provider_allowlists_v012.argx`
- `examples/provider_allowlists_v012.argbc.json`
- `examples/provider_allowlists_tools_v012.argx`
- `examples/provider_allowlists_tools_v012.argbc.json`

The first allowlists `GuardModel` with `model.invoke`. The second allowlists
`WebSearch` with `web.search`. Their executable tool/model declarations remain
on `simulated`.

Invalid source fixtures:

- `provider_allowlist_unknown_target.argx`
- `provider_allowlist_unknown_capability.argx`
- `provider_allowlist_duplicate_target.argx`
- `provider_allowlist_duplicate_capability.argx`
- `provider_allowlist_incompatible_capability.argx`
- `provider_allowlist_external_execution_still_blocked.argx`

Parser and unit-test-only cases additionally cover duplicate blocks and an
ambiguous target name.

## Test Strategy

Tests follow red-green-refactor and cover:

1. parser recognition of `allowed_targets`;
2. parser recognition of `allowed_capabilities`;
3. both block orders;
4. duplicate-block rejection;
5. v0.11-style source compatibility without blocks;
6. valid model allowlist semantics;
7. valid tool allowlist semantics;
8. unknown target rejection;
9. unknown capability rejection;
10. duplicate target rejection;
11. duplicate capability rejection;
12. ambiguous target rejection;
13. incompatible model capability rejection;
14. incompatible tool capability rejection;
15. continued rejection of external model execution despite allowlisting;
16. continued rejection of external tool execution despite allowlisting;
17. populated IR target and capability arrays;
18. populated Bytecode target and capability arrays;
19. Bytecode 0.11 empty-array compatibility;
20. Bytecode 0.12 populated-array acceptance;
21. exact top-level/instruction correspondence;
22. registry acceptance of populated structural contracts;
23. VM loading of populated allowlists;
24. VM JSON allowlist reporting;
25. CLI populated-list reporting;
26. CLI empty-list `none` reporting;
27. runtime blocking despite a valid allowlist;
28. v0.11 execution compatibility.

Generated fixtures are structurally compared against actual compiler output.
The six invalid fixtures must each exit nonzero with the intended diagnostic.

## Documentation and Versioning

Workspace packages, compiler text, VM text, IR, and newly emitted Bytecode
advance to version `0.12`.

README and technical documentation cover:

- public allowlist syntax;
- arbitrary order of the two optional blocks;
- duplicate-block and duplicate-entry rules;
- compatibility with v0.11 source and Bytecode;
- empty lists as zero future permissions;
- no-wildcard semantics;
- strict separation between executable providers and declarative contracts;
- the rule `Allowlisted does not mean executable`.

The release statement is:

```text
v0.11 defines external adapter contracts without executing them.
v0.12 constrains future external adapters with declarative allowlists.
```

Final verification runs the requested valid compiler and VM commands, all six
invalid checks, fixture comparisons, and:

```bash
cargo fmt
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The project principle remains:

```text
Rust is the forge.
Argorix Lang is the sword.
```
