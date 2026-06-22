# Argorix Lang v0.20 Sandboxed Provider Harness Design

## Principle

`Before execution comes containment.`

External providers must have declarative containment metadata before they can
be considered for any future execution work. Version 0.20 does not execute,
connect to, authenticate with, or resolve external providers.

## Source Model and AST

Version 0.20 adds a top-level declaration:

```argx
harness OpenAIHarness {
  provider OpenAI
  mode dry_run
  network denied
  secrets denied
  filesystem none
  max_steps 10
  timeout_ms 1000
  input_contract UserPrompt
  output_contract DraftAnswer
  attestations ["dry-run", "policy-check", "evidence-bundle"]
}
```

The AST gains `Program.harnesses: Vec<ProviderHarnessDecl>` and source-faithful
enums:

```rust
pub enum HarnessMode {
    DryRun,
    Simulated,
    Unknown(String),
}

pub enum HarnessNetwork {
    Denied,
    Unknown(String),
}

pub enum HarnessSecrets {
    Denied,
    Unknown(String),
}

pub enum HarnessFilesystem {
    None,
    ReadOnly,
    Unknown(String),
}
```

Required fields are non-optional AST values. The parser uses empty
source-preserving sentinels when a required field is absent, matching the v0.19
passport pattern. It never supplies a valid default. Optional numeric and
contract-reference fields remain `Option`.

`attestations` is represented as a vector. An absent field and an explicit
empty array both produce an empty vector and are valid. An attestation entry
whose string is empty is invalid.

## Lexer and Parser Boundary

The lexer adds unsigned integer literals for `max_steps` and `timeout_ms`.
Floats, signed values, numeric separators and units are outside v0.20.

The parser validates only structural syntax:

- declaration shape, braces, arrays and integer token placement;
- duplicate fields inside one harness;
- numeric literal conversion to `u64`;
- unknown harness field names.

The parser preserves unsupported enum-like values as `Unknown(String)`.
Missing required fields, unknown values, disallowed values, reference
resolution and positive-value constraints belong to semantic analysis.

## Semantic Rules

Whole-program semantic checking validates:

- globally unique harness names;
- required `provider`, `mode`, `network`, `secrets` and `filesystem` fields;
- provider references resolve to declared provider contracts;
- modes are `dry_run` or `simulated`;
- network and secrets are `denied`;
- filesystem is `none` or `read_only`;
- optional `max_steps` and `timeout_ms` are greater than zero;
- optional input/output contracts resolve to declared types;
- attestation entries are non-empty strings.

Existing provider-contract validation remains authoritative. An external
provider referenced by a harness must still be disabled, dry-run-only, guarded
by a feature flag, explicitly approved in its contract and non-executable.

Semantic failure stops lowering. Missing-field sentinels and `Unknown` values
must never reach IR, Bytecode, VM traces, reports or evidence bundles.

## Modules and Package Merge

Package resolution continues to include only reachable imported modules.
Deterministic merge order remains entry module first followed by sorted module
names. Harness declarations are merged in that order. Whole-package semantic
checking detects duplicate harness names and resolves provider/type references
across imported modules.

## IR and Bytecode 0.20

IR and Bytecode advance to `0.20` and add ordered top-level
`provider_harnesses` metadata containing:

```json
{
  "name": "OpenAIHarness",
  "provider": "OpenAI",
  "mode": "dry_run",
  "network": "denied",
  "secrets": "denied",
  "filesystem": "none",
  "max_steps": 10,
  "timeout_ms": 1000,
  "input_contract": "UserPrompt",
  "output_contract": "DraftAnswer",
  "attestations": ["dry-run", "policy-check", "evidence-bundle"]
}
```

Optional fields serialize as JSON `null`; attestations serialize as an array.
Harnesses add no VM instruction because they are containment metadata rather
than executable operations.

The Bytecode verifier accepts historical versions through 0.20. Harness
metadata is only valid in Bytecode 0.20. It validates unique names, non-empty
providers, allowed enum values, positive optional limits, declared optional
contract references when type metadata is available, and non-empty
attestation entries.

## VM Trace and Ledger

The VM validates Bytecode before execution, copies harness metadata to
`ReactiveExecutionTrace`, and records declaration/validation evidence in the
existing ledger. Events are limited to:

- `ProviderHarnessDeclared`;
- `ProviderHarnessValidated`;
- `ProviderHarnessSandboxed`;
- `ProviderHarnessRejected` when rejected Bytecode can be represented in the
  existing early-failure path.

No event represents provider execution. Harness processing performs no file
access, process creation, environment lookup, DNS, socket, HTTP or provider
registry execution.

## Policy Language v2

Version 0.20 adds these require/deny rule names:

- `provider_harness_declared`;
- `provider_harness_sandboxed`;
- `provider_network_denied`;
- `provider_secrets_denied`;
- `provider_filesystem_restricted`;
- `external_provider_harnessed`.

Rules evaluate offline against verified Bytecode metadata:

- `provider_harness_declared` requires at least one harness;
- `provider_harness_sandboxed` requires every harness to use an allowed mode,
  denied network/secrets and restricted filesystem;
- network, secrets and filesystem rules validate their named dimension for
  every harness;
- `external_provider_harnessed` requires every declared external provider to
  have at least one associated harness.

For universal rules, an empty harness collection passes the dimension-specific
rule by vacuous truth; callers that require existence must also require
`provider_harness_declared`. `external_provider_harnessed` passes when no
external provider contracts exist.

## SecurityReport and EvidenceBundle

SecurityReport 0.20 adds a `provider_harnesses` summary with total declarations,
sorted unique providers and contracts, per-value counts and total
attestations. It reports structural evidence only and does not claim real-world
sandbox security.

Harness metadata alone does not increase verdict severity. Policy violations
retain existing action semantics: `review` yields `review_required`, `block`
fails, and attempted external execution keeps highest provider-boundary
precedence.

EvidenceBundle 0.20 uses the existing canonical digest chain over Bytecode,
trace, SecurityReport and ledger-bearing trace data. Offline verification
accepts bundle versions 0.14 through 0.20 and preserves v0.19 verification.

## Fixtures, Conformance and Documentation

The release adds:

- a valid single-file v0.20 fixture and generated Bytecode fixture;
- a multi-module provider-harness package;
- invalid fixtures for every semantic and policy failure;
- `conformance/suite.v020.json` with a `provider_harness` category;
- text and JSON conformance coverage;
- README documentation distinguishing provider contracts, harnesses,
  simulated providers and blocked external providers.

## Compatibility and Security Boundary

Bytecode 0.19, EvidenceBundle 0.19, Agent Passport, Typed Message Contracts,
Policy Language v2, modules/packages, provider contracts, allowlists, legacy
assertions, nominal types and existing CLI flags remain supported.

Version 0.20 adds no network client, socket, server, DNS lookup, secret or API
key access, environment-based credential loading, remote identity lookup,
provider adapter, executable feature flag or real provider call. `simulated`
remains the only executable provider.

## Acceptance

The implementation is complete only when targeted red-green tests and the
full workspace verify:

```bash
cargo fmt
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The single-file and package fixtures must compile, v0.20 Bytecode must verify,
trace/report/evidence must preserve harness metadata, Conformance Suite 0.20
must pass in text and JSON modes, invalid fixtures must fail, and v0.19
Bytecode/Evidence compatibility must remain covered by automated tests.
