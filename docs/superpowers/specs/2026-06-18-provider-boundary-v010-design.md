# Argorix Lang v0.10 Provider Boundary Design

## Objective

Argorix Lang v0.10 introduces a synchronous, audited provider boundary between
the VM and model/tool implementations. The only provider is `simulated`; it
performs no network, secret, environment, async, or external execution.

## Source and compilation contract

- Models continue to require `provider simulated`.
- Tools accept an optional `provider`.
- `ToolDecl.provider` remains `Option<Spanned<String>>`, preserving source.
- Semantics accepts only `simulated`; an omitted tool provider is valid.
- IR resolves omitted tool providers to `simulated`.
- IR 0.10 and Bytecode 0.10 always contain explicit providers.
- Bytecode 0.3 through 0.9 remains readable. Legacy tools missing provider
  deserialize with the `simulated` default.

## Provider crate

`argorix_provider` owns:

- `Provider`, the synchronous model/tool invocation interface.
- Provider request and response contracts.
- `ProviderKind` and `ProviderCallStatus`.
- `ProviderError`.
- `ProviderRegistry`, with `simulated` registered by default.
- `SimulatedProvider`, which accepts only `dry_run: true`, preserves call and
  output types, marks responses simulated, and performs no external work.

The crate does not depend on the VM.

## VM integration

`Vm` owns a `ProviderRegistry`. Reactive tool/model calls retain existing
authorization and capability checks, then:

1. Resolve the explicit provider from bytecode.
2. Select it from the registry.
3. Create the typed provider request.
4. Invoke the provider synchronously.
5. Validate call ID, output type, allowed status, and simulated result.
6. Store a provider-call summary and emit the existing dry-run result.

Boundary failures are fail-closed. The VM records denial/failure events,
preserves the ledger, marks runtime failed, and activates `ToolDenied`,
`ModelDenied`, or `PolicyViolation` when available.

## Audit and output

New events:

- `ProviderRegistered`
- `ProviderSelected`
- `ProviderRequestCreated`
- `ProviderResponseReceived`
- `ProviderDryRunEnforced`
- `ProviderBoundaryDenied`

Reactive JSON adds `providers` and `provider_calls`. Text output adds
`--providers`, showing the registry and ordered provider calls. Existing tool,
model, state, policy, and JSON behavior remains available.

## Verification

Tests cover provider contracts, parser fidelity, semantic rules, default
resolution, IR/bytecode propagation, VM routing, audit events, JSON, CLI, and
v0.9 compatibility. Final gates are `cargo fmt --check`,
`cargo test --workspace`, and
`cargo clippy --workspace --all-targets -- -D warnings`.
