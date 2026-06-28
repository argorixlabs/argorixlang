<div align="center">
  <img width="520" src="https://argorix-lang.org/assets/argorix-lockup.png" alt="Argorix Lang" />

  <br />
  <br />

  <strong>Secure. Verifiable. Programmable.</strong>

  <br />
  <br />

  <a href="https://argorix-lang.org">Website</a>
  ·
  <a href="https://github.com/argorixlabs/argorixlang">Repository</a>
  ·
  <a href="#build-and-verify">Build</a>
  ·
  <a href="#roadmap">Roadmap</a>
  ·
  <a href="./LICENSE">Apache-2.0</a>
</div>

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![CI](https://github.com/argorixlabs/argorixlang/actions/workflows/ci.yml/badge.svg)](https://github.com/argorixlabs/argorixlang/actions/workflows/ci.yml)
[![Security & licenses](https://github.com/argorixlabs/argorixlang/actions/workflows/security.yml/badge.svg)](https://github.com/argorixlabs/argorixlang/actions/workflows/security.yml)
[![DCO](https://github.com/argorixlabs/argorixlang/actions/workflows/dco.yml/badge.svg)](https://github.com/argorixlabs/argorixlang/actions/workflows/dco.yml)
[![Conventional Commits](https://img.shields.io/badge/Conventional%20Commits-1.0.0-%23FE5196.svg)](https://conventionalcommits.org)

<div align="center">

<!-- GitHub strips <iframe>, so this clickable thumbnail is the embed that renders on the repo page. -->
<a href="https://www.youtube.com/watch?v=ZhQMps17CFo">
  <img width="560" src="https://img.youtube.com/vi/ZhQMps17CFo/maxresdefault.jpg" alt="Watch the Argorix Lang video" />
</a>

<!-- Renders on sites that allow iframes (e.g. argorix-lang.org). -->
<iframe width="560" height="315" src="https://www.youtube.com/embed/ZhQMps17CFo?si=6UW1u-i3evlLdN04" title="YouTube video player" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>

</div>

# Argorix Lang

**Argorix Lang** is a compiled language for secure, verifiable communication between AI agents.

It is currently implemented in **Rust**, with a long-term path toward progressive self-hosting. The project explores language-level infrastructure for AI-agent systems where security, traceability, provider boundaries, and runtime evidence are part of the execution model rather than afterthoughts.

Argorix Lang is early-stage infrastructure, but the direction is explicit:

```text
source language
  -> parser
  -> semantic and security verification
  -> Argorix IR
  -> Argorix Bytecode
  -> Argorix VM
  -> deterministic scheduling
  -> controlled tool/model calls
  -> provider boundary validation
  -> global policy verification
  -> trace ledger
```

## Current status

**Version:** `1.0`
**Status:** Secure Multi-Agent Runtime MVP
**License:** Apache-2.0  
**Implementation:** Rust  
**Execution modes:** `dry_run`, `simulated`, and governed `sandboxed_external`

Version 1.0 adds the Secure Multi-Agent Runtime MVP. Its governing rule is:

```text
Runtime may execute only what governance explicitly permits.
```

The top-level `runtime_execution_profile` binds named agents and a provider to
Runtime Hardening, a Threat Model, an ATrust Evidence Map, Governance Profile,
explicit allowed/denied actions, audit evidence, SecurityReport generation,
and `fail_closed true`. The top-level `sandboxed_provider_adapter` binds that
runtime to a bounded operation set and redacted endpoint/secret references.

The three modes have deliberately different behavior:

- `dry_run` produces trace and evidence without provider execution.
- `simulated` uses only the existing deterministic in-process `simulated`
  provider.
- `sandboxed_external` is blocked unless the adapter, operation, policy,
  hardening, evidence, governance, audit, and fail-closed guards all validate
  and the caller supplies `--sandboxed-external`.

In v1.0, the explicit flag creates an auditable external-call plan; core
Argorix performs no HTTP request and ships no mandatory OpenAI SDK. An
OpenAI-compatible adapter may declare references such as
`env:OPENAI_BASE_URL` and `env:ARGORIX_PROVIDER_TOKEN`, but Argorix never reads or
prints their values. Bytecode stores the reference names with
`endpoint_value: null`, `secret_value: null`, and `redacted: true`.

```powershell
argorix-vm run examples/runtime_mvp_v100.argbc.json `
  --runtime ChatbotRuntime `
  --adapter OpenAISandbox `
  --operation responses.create `
  --sandboxed-external `
  --json
```

This is not a free runtime. External providers remain non-executable by
default, network is never opened by default, tools and agents cannot execute
freely, and secret/key material is never embedded. MCP and A2A remain
declarative bridge contracts. DID, VC, credential, handshake, signature, and
blockchain verification remain outside the runtime. SecurityReport v1.0 and
EvidenceBundle v1.0 record the profile, adapter, redaction, policy result, and
blocked/planned execution events while offline verification retains v0.36 and
older compatibility.

See
[`examples/runtime_mvp_v100.argx`](examples/runtime_mvp_v100.argx),
[`examples/runtime_mvp_v100.argbc.json`](examples/runtime_mvp_v100.argbc.json),
[`examples/runtime_mvp_project`](examples/runtime_mvp_project), and
[`conformance/suite.v100.json`](conformance/suite.v100.json).

## v0.36 specification freeze

Version 0.36 adds Spec Freeze + v1.0 Release Candidate metadata. A top-level
`spec_freeze` pins the frozen feature surface, accumulated compatible versions,
required conformance suites, evidence requirements, and closed runtime
boundaries. A top-level `release_candidate` binds that freeze to required local
artifacts, release checks, a compatibility matrix, and known limitations.

Spec freeze does not mean production runtime. Release-candidate metadata does
not mean production, legal, compliance, regulator, or security certification.
Both declarations require `runtime_status disabled`, `network denied`,
`external_execution disabled`, `provider_execution disabled`,
`tool_execution disabled`, `agent_execution disabled`, `env_access denied`,
`filesystem_access denied`, `secret_material denied`, `key_material denied`,
`security_claims none`, `legal_claims none`, and `certification none`.

The freeze extends Runtime Hardening, Public Conformance, Governance Profiles,
ATrust Evidence Mapping, Trust Ledger, Policy v2, SecurityReport, and
EvidenceBundle. It does not change their prior semantics. SecurityReport and
EvidenceBundle advance to v0.36 while offline compatibility with v0.34 and
v0.35 remains explicit.

### v1.0 RC boundaries

Argorix v1.0 RC is still declarative. Runtime execution remains disabled.
External providers remain non-executable unless a future sandbox explicitly
enables them; `simulated` remains the only executable provider today. OpenAI
API keys and OpenAI API support are not part of core Argorix Lang. MCP/A2A
remain bridge contracts, not live bridges. DID, VC, credential, and handshake
verification remains non-real and declared-only. No network, environment,
filesystem, secret, key, signature, blockchain, regulator, or certification
capability is introduced by the release candidate.

See
[`examples/spec_freeze_v036.argx`](examples/spec_freeze_v036.argx),
[`examples/spec_freeze_v036.argbc.json`](examples/spec_freeze_v036.argbc.json),
and [`examples/spec_freeze_project`](examples/spec_freeze_project).

Version 0.35 adds Runtime Hardening + Threat Model. A top-level
`runtime_hardening_profile` binds deny-by-default enforcement, sandbox,
network, provider, tool, agent, filesystem, environment, secret, and key
boundaries to the existing evidence, governance, and public-conformance
artifacts. A top-level `threat_model` maps assets, threats, mitigations,
residual risk, and risk acceptance to that profile.

Both declarations are offline metadata. They do not enable a runtime, execute
agents or tools, call providers, open network connections, read environment
variables, secrets, or keys, simulate attacks, execute exploits, verify a
third party, certify security, eliminate risk, or provide legal certification.
Their VM events and Policy v2 rules preserve those boundaries fail-closed.
SecurityReport v0.35 summarizes hardening profiles and threat models;
EvidenceBundle v0.35 covers the resulting bytecode, trace, report, and ledger
while retaining verification compatibility with v0.33 and v0.34 artifacts.

See
[`examples/runtime_hardening_v035.argx`](examples/runtime_hardening_v035.argx),
[`examples/runtime_hardening_v035.argbc.json`](examples/runtime_hardening_v035.argbc.json),
and
[`examples/runtime_hardening_project`](examples/runtime_hardening_project).

Version 0.34 adds Third-Party Verification / Public Conformance. A top-level
`third_party_verifier` declares reviewer identity metadata, organization,
jurisdiction, independence, bounded review scopes, and explicitly disallowed
claims. A top-level `public_conformance_report` binds that verifier to a local
conformance suite, source and bytecode artifacts, ATrust Evidence Map,
Governance Profile, Regulatory Mapping, Trust Ledger, SecurityReport, trace,
EvidenceBundle, review result, reproducibility mode, and individually mapped
claims.

The governing rule is: public conformance must be reproducible before it can be
trusted. These declarations are audit artifacts. A declared third-party
verifier is not an externally authenticated legal auditor; a passed public
conformance report is not regulator approval or legal certification; a mapped
claim is not legally certified; and reproducible artifacts are not
cryptographic endorsement. Published metadata does not mean a remote audit
occurred and a passed suite does not mean risk was eliminated.

Both declarations are fail-closed with `legal_claims none`, `certification
none`, `network denied`, `external_execution disabled`, `secret_material
denied`, `key_material denied`, `execution disabled`, and `security_claims
none`. Argorix performs no network calls, secret/key reads, external verifier
execution, signing, signature verification, DID/credential verification,
remote attestation, regulator submission, or real MCP/A2A runtime.

Policy v2 evaluates verifier scope, artifact bindings, evidence/governance/
regulatory relationships, reproducibility, and absent runtime/legal/security
claims offline. SecurityReport v0.34 summarizes both declaration families.
EvidenceBundle v0.34 semantically covers the bytecode, trace, report, and
ledger. This extends Governance Profiles, Regulatory Mapping, ATrust Evidence
Mapping, Trust Ledger, and Policy v2 without converting any of them into legal
or cryptographic proof.

See
[`examples/public_conformance_v034.argx`](examples/public_conformance_v034.argx),
[`examples/public_conformance_v034.argbc.json`](examples/public_conformance_v034.argbc.json),
and
[`examples/public_conformance_project`](examples/public_conformance_project)
for the single-file, bytecode, and package examples.

Version 0.33 adds Governance Profiles + Regulatory Mapping. A top-level
`governance_profile` records scope, ownership, jurisdiction, framework,
ATrust Evidence Map and Trust Ledger bindings, policies, controls, risk level,
review status, and assurance. A top-level `regulatory_mapping` maps declared
obligations to those controls and their evidence for audit review.

Governance must be declared before compliance can be assessed. These blocks are
metadata and audit aids: a governance profile is not a compliance
certification; a regulatory mapping is not legal advice; an obligation mapped
is not an obligation legally satisfied; a control mapped is not an externally
audited control; and a declared risk level does not mean risk was eliminated.

Both blocks remain fail-closed: `legal_claims none`, `certification none`,
`network denied`, `external_execution disabled`, `secret_material denied`,
`key_material denied`, `execution disabled`, and `security_claims none`.
Argorix does not claim regulator approval or legal compliance. Policy v2
evaluates structural governance bindings offline. SecurityReport v0.33
summarizes profiles, controls, mappings, obligations, and denied runtime
boundaries. EvidenceBundle v0.33 covers the bytecode, trace, report, and ledger
digests without identity, credential, handshake, signature, blockchain,
MCP/A2A, network, or legal verification.

See
[`examples/governance_mapping_v033.argx`](examples/governance_mapping_v033.argx)
for the complete single-file example.

Version 0.32 adds ATrust Evidence Mapping: a top-level
`atrust_evidence_map` block that links an agent passport, ATrust identity,
credential contract, handshake, trust ledger, MCP/A2A bridge contracts,
policies, SecurityReport, trace, and EvidenceBundle as declared evidence
metadata.

The core rule is: evidence must be mapped before trust can be evaluated. A map
can say an identity is declared, a credential is declared, a dry-run handshake is
declared, a ledger contains the referenced event, bridge contracts are declared,
and an evidence bundle covers those pieces. It never says identity verified,
credential verified, handshake executed or secure, MCP/A2A connected, signature
verified, tamper-proof, blockchain verified, or post-quantum secure.

`atrust_evidence_map` is locked to non-runtime boundaries: `mapping_mode
declared_only` or `evidence_only`, `verification declared_only` or `disabled`,
`resolution disabled`, `network denied`, `external_execution disabled`,
`secret_material denied`, `key_material denied`, `execution disabled`, and
`security_claims none`.

SecurityReport v0.32 includes an `atrust_evidence_maps` summary with totals,
names, required coverage, non-verifying mode counts, denied network/execution
counts, `security_claims none`, and identity/credential/handshake/ledger/bridge
link totals. EvidenceBundle v0.32 covers the resulting bytecode, trace, report,
and ledger digests. Policy v2 adds `atrust_evidence_map_*` rules for declared
maps, bound links, required coverage, disabled resolution/execution, denied
network/material, and absent security claims.

Version 0.31 adds MCP / A2A Bridge Contracts: two top-level blocks,
`mcp_bridge_contract` and `a2a_bridge_contract`, that declare *how* an agent
could interoperate with external MCP tools/resources/prompts or with another
agent over A2A. A bridge contract describes an allowed interoperability surface;
it does **not** open network access, start an MCP server, send A2A messages,
execute tools or agents, read API keys, complete OAuth, resolve DIDs, or verify
credentials. **A bridge may be declared before it is connected** — a declared
bridge is never a connected bridge. v0.31 only declares, validates, lowers to
IR/bytecode, reports, and produces evidence for these contracts.

Version 0.30 added the Trust Ledger Hash Chain: a top-level `trust_ledger` block
that preserves an ordered, auditable hash chain linking earlier trust evidence
(identities, credential contracts, handshakes) and the evidence bundle. It is an
audit structure, not a blockchain and not a cryptographic trust guarantee — there
is no consensus, mining, networking, signing, signature verification, key/secret
handling, or DID/credential verification. Trust evidence may be linked before it
is trusted; no trust event becomes authority merely because it is chained.

This builds on the ATrust line: v0.26 ATrust Boundary Contracts, v0.27 ATrust
Identity Dry-Run, v0.28 ATrust Credential Contracts, and v0.29 ATrust Handshake
Dry-Run — each declarative, compilable, auditable metadata only. Earlier layers
are all preserved: Crypto Primitive Registry and Crypto Boundary + Post-Quantum
Readiness (v0.24–v0.25); the Adapter Framework and Declarative Adapter Profiles
(v0.22–v0.23); Feature Flags + Secret Boundary (v0.21) and Sandboxed Provider
Harness (v0.20) governance metadata; the Agent Passport / Sovereign Agent
Identity block (v0.19); and Typed Message Contracts, Policy Language v2, and the
Module / Package System (v0.16–v0.18). No version reads environment variables,
stores secret material, opens a vault, resolves a DID, or makes a network call.

```text
argorix.toml + src/*.argx
  -> module resolution (deterministic graph)
  -> whole-package semantic and security verification
  -> lexer / parser / AST
  -> Argorix IR 0.31 (with MCP/A2A bridge contracts, trust ledger, ATrust handshake/credential/identity/boundary, DID method, crypto, adapter, feature, secret, harness, passport, typed message, policy and module metadata)
  -> Argorix Bytecode 0.31 (with MCP/A2A bridge contracts, trust ledger, ATrust handshake/credential/identity/boundary, DID method, crypto, adapter, feature, secret, harness, passport, typed message, policy and module metadata)
  -> Argorix VM
  -> agent mailboxes
  -> deterministic scheduler
  -> reactive handlers
  -> agent state and causal guards
  -> controlled tool calls
  -> controlled model calls
  -> provider registry
  -> external adapter contract validation
  -> simulated provider boundary
  -> legacy assertion and Policy v2 verification
  -> declared failure modes
  -> trace ledger
  -> deterministic security report
```

> The VM does not call LLMs, tools, MCP, A2A, networks, shells, or other external systems.  
> It validates bytecode and simulates protocol message flow only.

## Why Argorix Lang?

Most AI-agent systems today are built on fragile layers of prompts, wrappers, tools, provider-specific logic, and scattered runtime permissions.

That can work for prototypes.

It becomes harder to reason about when systems need:

- security guarantees,
- traceable execution,
- auditable behavior,
- provider boundaries,
- controlled tool/model calls,
- deterministic runtime state,
- policy verification,
- and evidence suitable for inspection.

Argorix Lang explores a different path: **structured, auditable, programmable execution for AI-agent systems.**

## Why Rust?

Argorix Lang is implemented in Rust because infrastructure for AI safety should start from a secure systems foundation.

Rust provides:

- memory safety,
- strong typing,
- predictable performance,
- explicit control,
- concurrency safety,
- and a strong base for compiler, bytecode, and VM infrastructure.

Rust is not just an implementation choice for Argorix Lang.

It reflects the project’s design philosophy: secure infrastructure should be built on secure foundations.

## Requirements

- Stable Rust toolchain
- Cargo

## Build and verify

```bash
cargo fmt
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Compiler commands

```bash
cargo run -p argorixc -- check examples/prompt_defense_v02.argx
cargo run -p argorixc -- emit-ir examples/prompt_defense_v02.argx
cargo run -p argorixc -- graph examples/prompt_defense_v02.argx
cargo run -p argorixc -- capabilities examples/prompt_defense_v02.argx
cargo run -p argorixc -- emit-bytecode examples/prompt_defense_v02.argx
cargo run -p argorixc -- verify-bytecode examples/prompt_defense_v02.argx
```

Package commands (multi-file projects, v0.16):

```bash
cargo run -p argorixc -- check-package examples/module_project/argorix.toml
cargo run -p argorixc -- emit-ir-package examples/module_project/argorix.toml
cargo run -p argorixc -- emit-bytecode-package examples/module_project/argorix.toml
cargo run -p argorixc -- graph-package examples/module_project
```

Each package command also accepts a directory and looks for `argorix.toml`.

Latest provider allowlist example:

```bash
cargo run -p argorixc -- check examples/provider_allowlists_v012.argx
cargo run -p argorixc -- emit-ir examples/provider_allowlists_v012.argx
cargo run -p argorixc -- emit-bytecode examples/provider_allowlists_v012.argx
```

## VM commands

```bash
cargo run -p argorix-vm -- run examples/prompt_defense.argbc.json --dry-run
cargo run -p argorix-vm -- run examples/prompt_defense.argbc.json --dry-run --json
cargo run -p argorix-vm -- run examples/prompt_defense.argbc.json --dry-run --mailboxes
```

Reactive execution example:

```bash
cargo run -p argorix-vm -- run examples/prompt_defense_v06.argbc.json \
  --dry-run \
  --reactive \
  --inject User:PromptScanner:tell:UserPrompt \
  --state
```

Controlled tool-call example:

```bash
cargo run -p argorix-vm -- run examples/tool_call_v07.argbc.json \
  --dry-run \
  --reactive \
  --inject User:ResearchAgent:tell:UserPrompt \
  --state \
  --tools
```

Controlled model-call example:

```bash
cargo run -p argorix-vm -- run examples/model_call_v08.argbc.json \
  --dry-run \
  --reactive \
  --inject User:ResearchAgent:tell:UserPrompt \
  --state \
  --tools \
  --models
```

Provider boundary example:

```bash
cargo run -p argorix-vm -- run examples/provider_boundary_v010.argbc.json \
  --dry-run \
  --reactive \
  --inject User:ResearchAgent:tell:UserPrompt \
  --state \
  --tools \
  --models \
  --policy \
  --providers
```

Provider contract allowlist example:

```bash
cargo run -p argorix-vm -- run examples/provider_allowlists_v012.argbc.json \
  --dry-run \
  --reactive \
  --inject User:ResearchAgent:tell:UserPrompt \
  --state \
  --tools \
  --models \
  --policy \
  --providers \
  --provider-contracts
cargo run -p argorix-vm -- run examples/prompt_defense_v05.argbc.json --dry-run --reactive --inject User:PromptScanner:tell:UserPrompt
cargo run -p argorix-vm -- run examples/prompt_defense_v05.argbc.json --dry-run --reactive --inject User:PromptScanner:tell:UserPrompt --json
cargo run -p argorixc -- check examples/prompt_defense_v06.argx
cargo run -p argorixc -- emit-ir examples/prompt_defense_v06.argx
cargo run -p argorixc -- emit-bytecode examples/prompt_defense_v06.argx
cargo run -p argorix-vm -- run examples/prompt_defense_v06.argbc.json --dry-run --reactive --inject User:PromptScanner:tell:UserPrompt --state
cargo run -p argorixc -- check examples/tool_call_v07.argx
cargo run -p argorixc -- emit-ir examples/tool_call_v07.argx
cargo run -p argorixc -- emit-bytecode examples/tool_call_v07.argx
cargo run -p argorix-vm -- run examples/tool_call_v07.argbc.json --dry-run --reactive --inject User:ResearchAgent:tell:UserPrompt --state --tools
cargo run -p argorixc -- check examples/model_call_v08.argx
cargo run -p argorixc -- emit-ir examples/model_call_v08.argx
cargo run -p argorixc -- emit-bytecode examples/model_call_v08.argx
cargo run -p argorix-vm -- run examples/model_call_v08.argbc.json --dry-run --reactive --inject User:ResearchAgent:tell:UserPrompt --state --tools --models
cargo run -p argorixc -- check examples/policy_assertions_v09.argx
cargo run -p argorixc -- emit-bytecode examples/policy_assertions_v09.argx
cargo run -p argorix-vm -- run examples/policy_assertions_v09.argbc.json --dry-run --reactive --inject User:ResearchAgent:tell:UserPrompt --policy
cargo run -p argorixc -- check examples/provider_boundary_v010.argx
cargo run -p argorixc -- emit-ir examples/provider_boundary_v010.argx
cargo run -p argorixc -- emit-bytecode examples/provider_boundary_v010.argx
cargo run -p argorix-vm -- run examples/provider_boundary_v010.argbc.json --dry-run --reactive --inject User:ResearchAgent:tell:UserPrompt --state --tools --models --policy --providers
cargo run -p argorixc -- check examples/provider_contracts_v011.argx
cargo run -p argorixc -- emit-ir examples/provider_contracts_v011.argx
cargo run -p argorixc -- emit-bytecode examples/provider_contracts_v011.argx
cargo run -p argorix-vm -- run examples/provider_contracts_v011.argbc.json --dry-run --reactive --inject User:ResearchAgent:tell:UserPrompt --state --tools --models --policy --providers --provider-contracts
cargo run -p argorixc -- check examples/provider_allowlists_v012.argx
cargo run -p argorixc -- emit-ir examples/provider_allowlists_v012.argx
cargo run -p argorixc -- emit-bytecode examples/provider_allowlists_v012.argx
cargo run -p argorix-vm -- run examples/provider_allowlists_v012.argbc.json --dry-run --reactive --inject User:ResearchAgent:tell:UserPrompt --state --tools --models --policy --providers --provider-contracts
cargo run -p argorixc -- emit-bytecode examples/provider_allowlists_v013.argx
cargo run -p argorix-vm -- run examples/provider_allowlists_v013.argbc.json --dry-run --reactive --inject User:ResearchAgent:tell:UserPrompt --security-report reports/provider-allowlists.security.json
cargo run -p argorix-vm -- run examples/provider_allowlists_v013.argbc.json --dry-run --reactive --inject User:ResearchAgent:tell:UserPrompt --json --security-report reports/provider-allowlists.security.json
```

## Security report export v0.13

Reactive execution now uses `run_reactive_outcome()`, which always preserves the final `RuntimeState` and ordered trace ledger. `run_reactive()` remains a compatibility wrapper.

Use `run --security-report <path>` to write a pretty JSON evidence artifact. The CLI creates the required parent directory and writes the report before propagating a VM error. Failed executions therefore still exit nonzero, keep stderr diagnostics, and remain reportable. In `--json` mode stdout remains exactly the existing trace JSON; failed executions without a trace print no partial JSON.

The public `SecurityReport` includes execution, policy, provider-boundary, call, intrinsic, ledger, and verdict summaries. Counts come from actual runtime evidence. For the three-agent v0.13 fixture, the intrinsic summary is `facu_checkpoints: 3`, `marron_guards: 3`, and `intrinsic_events_total: 6`.

`ledger_digest` is `sha256:` plus SHA-256 of compact JSON for the ordered ledger events. It supports deterministic integrity checks and reproducible audits. It is not a signature, uses no key, and does not prove real-world safety.

Verdicts follow evidence: blocked external execution or runtime/provider-boundary failure is `high`; assertion or completed-runtime denial evidence is `medium`; completion without assertions is `informational`; completion with passing policy is `pass`.

## Argorix Lang v0.14 Evidence Bundle + Offline Verification

An `EvidenceBundle` is a portable manifest connecting the semantic content of
Argorix Bytecode, a `ReactiveExecutionTrace`, a `SecurityReport`, and its
ledger digest. It is generated locally and can be checked without network
access:

```bash
cargo run -p argorix-vm -- run examples/provider_allowlists_v014.argbc.json \
  --dry-run \
  --reactive \
  --inject User:ResearchAgent:tell:UserPrompt \
  --state \
  --tools \
  --models \
  --policy \
  --providers \
  --provider-contracts \
  --security-report reports/provider_allowlists_v014.security.json \
  --trace-out reports/provider_allowlists_v014.trace.json \
  --evidence-bundle reports/provider_allowlists_v014.bundle.json

cargo run -p argorix-vm -- verify-evidence reports/provider_allowlists_v014.bundle.json
cargo run -p argorix-vm -- verify-evidence reports/provider_allowlists_v014.bundle.json --json
```

Artifact paths are stored relative to the bundle directory with `/`
separators. Verification resolves them from that directory, so a complete
portable tree can be moved and checked offline.

Digests use `sha256:<hex>` over compact serialization of deserialized Rust
types. Formatting and whitespace changes do not alter semantic evidence;
content changes do. These digests are not signatures, use no keys, provide no
authenticity claim, and do not prove real-world safety.

Failed executions remain reportable. When execution fails before producing a
reactive trace, the report and bundle are still written when possible,
`trace_path` and `trace_digest` are `null`, and the process still exits
nonzero.

The governing rules remain:

```text
Allowlisted does not mean executable.
Failed executions must still be reportable.
Security reports are evidence artifacts, not success receipts.
Evidence must be exportable and independently checkable.
```

## Argorix Lang v0.15 Conformance Suite

The official Conformance Suite validates the Argorix stack directly through
library APIs:

```text
source -> parser -> semantics -> IR -> Bytecode -> verifier -> VM
       -> SecurityReport -> EvidenceBundle -> offline verification
```

Run it in text or JSON mode:

```bash
cargo run -p argorix-conformance -- run conformance/suite.v015.json
cargo run -p argorix-conformance -- run conformance/suite.v015.json --json
cargo run -p argorix-conformance -- run conformance/suite.v015.json \
  --workdir target/custom-conformance
```

The suite is local, deterministic, data-driven, and offline. It does not use
network access, secrets, environment variables, real tools, real models,
OpenAI, Anthropic, MCP, A2A, or executable external providers. Passing the
suite demonstrates conformance with the declared Argorix behavior; it does not
prove real-world security.

Each case declares:

```json
{
  "id": "unknown-capability-rejected",
  "name": "Unknown capability is rejected",
  "category": "semantics",
  "source_path": "sources/unknown_capability.argx",
  "stages": ["parse", "semantic_check"],
  "expected_failure_stage": "semantic_check",
  "expected_failure_contains": "Unknown capability"
}
```

`stages` defines what executes. `expected_failure_stage` explicitly defines
where a negative case must fail. The expected stage remains `failed`, later
stages become `skipped`, and the case passes when the diagnostic matches.

VM-dependent cases declare an explicit injection:

```json
"injection": "User:ResearchAgent:tell:UserPrompt"
```

Evidence-tampering cases use a declarative mutation applied only to the case
copy under the workdir:

```json
"mutation": {
  "before_stage": "verify_evidence",
  "artifact": "security_report",
  "json_pointer": "/module",
  "value": "Tampered"
}
```

Fixture paths resolve from the directory containing `suite.v015.json`, not
from the shell working directory. Generated artifacts are isolated under
`<workdir>/<case-id>/`. To add a case, add a portable fixture under
`conformance/sources` or `conformance/bytecode`, then add a JSON case with a
category, ordered stages, and any explicit injection, expected failure, or
mutation.

The v0.15 principles are:

```text
A secure language must be independently testable.
Conformance must make expected failure explicit.
Conformance must not depend on fixture-specific inference.
Conformance cases must be data-driven, not runner-driven.
Conformance paths resolve from the suite, not from the shell.
```

Security reports are evidence artifacts, not success receipts. `Allowlisted does not mean executable`: `simulated` remains the only executable provider, and external allowlists remain future permissions only.

## Argorix Lang v0.22 Adapter Framework + Adapter Conformance Suite

**Principle:** Adapter conformance comes before adapter execution.

v0.22 adds top-level `adapter` declarations. An adapter is declarative governance metadata that binds a provider contract, feature flag, secret boundary and harness. It records `kind`, `vendor`, `mode`, `execution disabled`, boundary restrictions (`network denied`, `secrets denied`, `filesystem none|read_only`), typed contracts and a `conformance` list.

Adapters are **never executed** in v0.22. `simulated` remains the only executable provider. External providers remain non-executable. No network, env, secrets, or external SDKs are used.

New Policy v2 rules:
`adapters_declared`, `adapters_execution_disabled`, `adapters_network_denied`, `adapters_secrets_denied`, `adapters_provider_harnessed`, `adapters_feature_gated`, `adapters_secret_boundaried`, `adapters_conformance_declared`, `adapters_evidence_required`.

SecurityReport and EvidenceBundle include adapter summaries. Conformance Suite v0.22 validates the adapter framework.

See `examples/adapter_framework_v022.argx` and `examples/adapter_framework_project/`.

All prior v0.16–v0.21 features are preserved. Bytecode/EvidenceBundle 0.21 remain verifiable.

## Argorix Lang v0.21 Feature Flags + Secret Boundary

The v0.21 principle is:

> Secrets must be declared before they can be accessed.

Extended:

> No secret crosses a boundary without evidence.

v0.21 adds two top-level declarations — `feature` and `secret` — that prepare
Argorix for future real adapters **without executing any provider**. They declare
and audit a frontier; they do not cross it. External providers remain
non-executable, `simulated` remains the only executable provider, and the VM
still makes no network calls, reads no environment variables, reads no API keys,
and opens no vaults.

### Feature flags

A `feature` declares an experimental or future capability, typically tied to an
external provider adapter:

```argx
feature OpenAIAdapter {
  provider OpenAI            // optional: links the feature to a declared provider
  status experimental        // experimental | preview | stable | deprecated
  default disabled           // disabled | enabled
  requires approval          // required when status is experimental or preview
  purpose "future-openai-adapter"
}
```

Required fields: `status`, `default`. Optional: `provider`, `requires approval`,
`purpose`. Rules:

- A feature linked to an **external** provider must declare `default disabled`.
- A feature whose `status` is `experimental` or `preview` must declare
  `requires approval`.
- Unknown values fail in semantics; missing required fields fail as
  `missing required field` (no silent defaults).

A feature flag never enables real execution in v0.21. It is governance metadata.

### Secret boundaries

A `secret` declares the **boundary** of a future secret. It records the handle,
scope, and denied access — never the secret value:

```argx
secret OpenAISecret {
  handle "ARGORIX_PROVIDER_TOKEN"     // expected future handle — metadata, not a value
  provider OpenAI             // optional link to a declared provider
  required_by OpenAIAdapter   // optional link to a declared feature
  scope adapter               // provider | adapter | model | tool | runtime
  access denied               // only `denied` is allowed in v0.21
  source none                 // only `none` is allowed in v0.21
}
```

Required fields: `handle`, `scope`, `access`, `source`. Optional: `provider`,
`required_by`.

**Secret handle vs secret value.** The `handle` is the *name* of a secret that
*would* be needed in the future (e.g. `ARGORIX_PROVIDER_TOKEN`). It is metadata. Argorix
stores no secret material: the fields `value`, `secret_value`, `token`,
`api_key_value`, `raw`, and `plaintext` are forbidden inside a `secret` and cause
a compile error. `access` may only be `denied` and `source` may only be `none` in
v0.21 — `allowed`, `guarded`, `approved`, `env`, `vault`, `file`, and `remote`
are intentionally not yet accepted.

### Harness links

A `harness` may optionally reference a declared feature and secret:

```argx
harness OpenAIHarness {
  provider OpenAI
  feature OpenAIAdapter
  secret OpenAISecret
  mode dry_run
  network denied
  secrets denied
  filesystem none
}
```

The semantic checker enforces coherence: referenced feature/secret must be
declared, and when providers are present on the harness, feature, and secret they
must agree; when a harness names both a feature and a secret whose `required_by`
is set, they must match.

### Policy v2 integration

v0.21 adds eight Policy v2 rules, evaluated offline against declared metadata:

- `feature_flags_declared` — at least one feature is declared.
- `features_default_disabled` — every feature defaults to disabled.
- `experimental_features_require_approval` — every experimental/preview feature
  requires approval.
- `secret_boundaries_declared` — at least one secret boundary is declared.
- `secret_access_denied` — every secret denies access.
- `secret_values_absent` — no secret declaration contains secret material
  (always true by construction; the schema has no value field).
- `external_provider_feature_gated` — every external provider is referenced by a
  disabled, approval-gated feature.
- `external_provider_secret_boundary_declared` — every external provider is
  referenced by a `denied`/`none` secret boundary.

All earlier rules (provider harness, agent passport, provider boundary) are
preserved.

### SecurityReport and EvidenceBundle integration

The SecurityReport (now `0.21`) gains a `feature_flags` summary (totals, statuses,
defaults, approval count, linked providers) and a `secret_boundaries` summary
(totals, scopes, access, sources, linked providers, `required_by`, and
`values_present` which is always `false`). The handle is reported as metadata; no
secret value, environment-variable content, or real material ever appears. Having
a feature flag or secret boundary does **not** inflate the verdict — it only
proves the frontier was declared, validated, and preserved as evidence.

The EvidenceBundle (now `0.21`) covers features and secrets through digests of the
bytecode, trace, and security report. Offline verification still accepts bundles
back to `0.14`, so older bundles continue to verify.

### Hard boundary

Feature flags do not enable real provider execution in v0.21. Secret boundaries
do not contain secret values. Argorix does not read environment variables, does
not read API keys, does not read vaults, and does not open network connections.
External providers remain non-executable; `simulated` remains the only executable
provider. First the boundary is declared. Then, some day, it is crossed with
evidence.

## Argorix Lang v0.20 Sandboxed Provider Harness

The v0.20 principle is:

> Before execution comes containment.

A provider contract declares what an external integration would be allowed to
target. A provider harness separately declares how that provider must be
contained during offline preparation and audit:

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

Required fields are `provider`, `mode`, `network`, `secrets`, and `filesystem`.
No required field receives a silent default. Supported values are:

- `mode dry_run` or `mode simulated`;
- `network denied`;
- `secrets denied`;
- `filesystem none` or `filesystem read_only`.

`max_steps` and `timeout_ms` are optional positive integers.
`input_contract` and `output_contract` optionally reference declared message
types. `attestations` may be absent or empty, but every supplied string must be
non-empty.

### Provider contract, harness, and executable provider

- A **provider contract** describes a disabled external boundary, future
  allowlists, feature-flag requirement, and explicit approval requirement.
- A **provider harness** is containment/governance metadata associated with a
  declared provider contract.
- The **simulated provider** is the only executable provider implementation.
- An **external provider** remains non-executable even when a valid harness is
  present.

Harnesses are top-level IR and Bytecode 0.20 metadata. They do not emit
`DeclareHarness`, `SandboxProvider`, or any other VM instruction.

### Policy v2 integration

The following offline rules inspect verified Bytecode metadata:

```txt
provider_harness_declared
provider_harness_sandboxed
provider_network_denied
provider_secrets_denied
provider_filesystem_restricted
external_provider_harnessed
```

Dimension-specific rules use universal evaluation. To require at least one
harness, also require `provider_harness_declared`.

### Trace, SecurityReport, and EvidenceBundle

Reactive traces preserve `provider_harnesses` and ledger events record
declaration, validation, and structural sandbox acceptance. SecurityReport
0.20 summarizes providers, modes, network/secrets/filesystem declarations,
contract references, and attestation totals. This is structural containment
evidence; it is not proof of real-world sandbox security.

EvidenceBundle 0.20 covers harness metadata through the existing canonical
digests of Bytecode, trace, SecurityReport, and trace ledger. Offline
verification remains compatible with bundle version 0.19.

### Hard boundary

The harness does not execute external providers. It does not call APIs, open
network connections, resolve DNS, read secrets, load API keys from environment
variables, create processes, or access files on behalf of a provider. Version
0.20 adds no real OpenAI, Anthropic, MCP, A2A, or NANDA adapter.

See `examples/provider_harness_v020.argx` and
`examples/provider_harness_project/`.

## Argorix Lang v0.19 Agent Passport / Sovereign Agent Identity

The v0.19 principle is:

```text
Agents must carry sovereign identity before they can participate in an open agentic web.
```

An **Agent Passport** is a top-level `passport` block that declares the sovereign
identity of an agent: who it is, where it is registered, what it is allowed to do,
and what evidence backs it. It is the agent's portable, auditable identity card.

```argx
passport RiskAnalyzerPassport {
  agent ResearchAgent
  agent_name "Risk Analyzer"

  // Global identity
  global_id "argx:agent:01HZX9RISKANALYZER"
  identity  "did:argorix:risk-analyzer-v1"
  provider  "Argorix"
  version   "1.0.0"

  // Optional discovery name — no network resolution in v0.19
  ans_name "argx://riskAnalyzer.RiskAnalysis.Argorix.v1.sovereign"

  // Jurisdiction and sovereignty
  country        "CL"
  jurisdiction   "CL"
  data_residency ["CL", "EU"]

  // Network / infrastructure registration metadata
  asn {
    registry "LACNIC"
    number   "AS-PLACEHOLDER"
    holder   "Argorix Labs"
    country  "CL"
  }

  // Model and risk metadata
  model      "frontier-compatible"
  risk_level "high"
  data_scope ["internal", "confidential"]

  // Intent / purpose
  intent         "risk_analysis"
  intended_use   ["policy-review", "risk-assessment"]
  prohibited_use ["external-execution", "credential-access"]

  // Verification and evidence
  attestations ["redteam", "policy-check", "evidence-bundle"]
}
```

### Field meaning

- **`global_id`** — a stable, globally unique identifier for the agent (an opaque
  string, e.g. `argx:agent:...`). It is not resolved against any registry.
- **`identity`** — a DID-like identity string (e.g. `did:argorix:...`). v0.19 stores
  it verbatim; it performs **no DID resolution**.
- **`agent_name`** — a human-readable display name.
- **`country` / `jurisdiction`** — the agent's legal sovereignty. `country` must use
  a 2-letter ISO-like code; `jurisdiction` must be non-empty.
- **`data_residency`** — the regions where the agent's data may reside (required,
  non-empty).
- **`asn`** — optional network registration metadata: `registry` (one of `LACNIC`,
  `ARIN`, `RIPE`, `APNIC`, `AFRINIC`, `UNKNOWN`), `number` (an `AS`-prefixed value
  or explicit placeholder), `holder`, and `country`. **No ASN lookup is performed.**
- **`intent` / `intended_use` / `prohibited_use`** — the declared purpose, allowed
  uses, and prohibited uses of the agent.
- **`attestations`** — references to evidence/verifications associated with the agent.

### `intent` vs `attestations`

These are different concepts and must not be conflated:

```text
intent         = the agent's declared purpose
intended_use   = permitted or expected uses
prohibited_use = forbidden uses
attestations   = evidence/verifications (internal or external) attached to the agent
```

`attestations` are **evidence, not intention**. Writing
`attestations ["risk_analysis"]` is syntactically allowed but semantically wrong —
`risk_analysis` is an `intent`, not an attestation.

### Passport vs provider contract vs policy vs evidence bundle

- **Passport** — *who the agent is*: sovereign identity, jurisdiction, residency,
  intent, attestations.
- **Provider contract** — *what external providers may be reached* (still
  non-executable; `simulated` remains the only executable provider).
- **Policy** — *what runtime evidence must hold* (Policy v2 rules evaluated against
  the trace).
- **Evidence bundle** — *the signed digest chain* over Bytecode, Trace, and
  SecurityReport that makes a run offline-verifiable.

### Required vs optional fields

```text
required: agent, agent_name, global_id, identity, provider, version,
          country, jurisdiction, data_residency, intent, risk_level
optional: ans_name, asn, model, data_scope, intended_use, prohibited_use, attestations
```

### Policy v2 integration

v0.19 adds four optional Policy v2 rules, evaluated offline against declared
passport metadata:

```text
agent_passport_declared        — every agent has a declared passport
agent_identity_declared        — every passport has a non-empty identity
agent_data_residency_declared  — every passport has non-empty data residency
agent_passport_attested        — every passport has at least one attestation
```

```argx
policy SovereignAgentPolicy {
  require agent_passport_declared
  require agent_identity_declared
  require agent_data_residency_declared
  require agent_passport_attested

  on violation {
    action review
    trace required
  }
}
```

### SecurityReport and EvidenceBundle integration

The SecurityReport gains an `agent_passports` summary (totals, linked agents,
countries, jurisdictions, data residency, risk levels, attestation count, and
intents). The EvidenceBundle covers passports through the existing digest chain
(Bytecode, Trace, SecurityReport) — no new artifact is added.

> **Holding a passport does not prove real-world safety.** It improves
> traceability, declared identity, and structural evidence only. The security
> verdict is **not** inflated by the presence of a passport.

### Limits (v0.19 does not)

```text
- no network calls, DNS resolution, or remote registry
- no real DID verification
- no real ASN verification
- no country verification beyond a basic ISO-like format check
- no certificates or secrets
- external providers remain non-executable; simulated remains the only executable provider
```

```bash
cargo run -p argorixc -- check examples/agent_passport_v019.argx
cargo run -p argorixc -- emit-ir examples/agent_passport_v019.argx
cargo run -p argorixc -- emit-bytecode examples/agent_passport_v019.argx
cargo run -p argorixc -- check-package examples/agent_passport_project/argorix.toml
cargo run -p argorix-conformance -- run conformance/suite.v019.json
```

## Argorix Lang v0.18 Typed Message Contracts

The v0.18 principle is:

```text
Agent communication must be typed before it can be trusted.
```

```argx
type ReviewResult {
    approved: bool
    score: int
    explanation: string
    confidence: float
}
```

`type Message`, `type Message {}`, and typed contracts are valid. Fields are
ordered metadata preserved in IR, Bytecode, VM trace, SecurityReport, and the
EvidenceBundle digest chain. Imported contracts participate in whole-package
checking.

Declared enum/type field references remain compatible as legacy nominal
contracts. Unknown references and duplicate fields fail semantic checking.
SecurityReport records total, typed, untyped, and field counts without treating
structural typing as proof of real-world safety.

v0.18 does not execute payload values and adds no arrays, maps, generics,
optional fields, unions, nested literals, validation expressions, network
access, secrets, or real providers. `simulated` remains the only executable
provider.

```bash
cargo run -p argorixc -- check examples/typed_messages_v018.argx
cargo run -p argorixc -- emit-bytecode examples/typed_messages_v018.argx
cargo run -p argorixc -- check-package examples/typed_message_project/argorix.toml
cargo run -p argorix-conformance -- run conformance/suite.v018.json
```

## Argorix Lang v0.17 Policy Language v2

The v0.17 principle is:

```text
Security policy must be declared as code, compiled as intent, and enforced as evidence.
```

Legacy assertions remain intact:

```argx
assert no_unhandled_messages
assert all_tool_calls_traced
assert runtime_status completed
```

Named policies add explicit `require` and `deny` effects:

```argx
policy ProviderSafety {
    require provider_contracts_declared
    require provider_allowlists_valid
    deny external_execution

    on violation {
        action block
        trace required
    }
}
```

`require X` passes only when the runtime evidence predicate for `X` is true.
`deny X` passes only when that predicate is false. `runtime_status completed`
is one rule.

Supported rules are:

```text
no_unhandled_messages
all_tool_calls_traced
all_model_calls_traced
all_intrinsics_traced
all_provider_calls_traced
halt_requires_trace
runtime_status completed
provider_contracts_declared
provider_allowlists_valid
external_execution
evidence_bundle_verified
security_report_generated
```

Unknown rules and actions are preserved by the parser for precise semantic
diagnostics. The semantic checker rejects duplicate policy names, duplicate
rules, contradictory `require`/`deny` effects, invalid actions, and duplicates
across imported modules.

Violation behavior:

- `action block`: records evidence, preserves the ledger, writes requested
  reports/bundles, and returns a nonzero VM/CLI result.
- `action review`: runtime may complete; the report verdict is
  `medium`/review required.
- `action warn`: runtime may complete; the report verdict is `warning`.
- no `on violation`: the policy is `violated` without activating a runtime
  action.

The trace separates `legacy_assertions` from `policy_blocks`. Policy events are
recorded as `PolicyDeclared`, `PolicyEvaluated`, `PolicyViolation`, and
`PolicyActionActivated`. SecurityReport 0.17 summarizes rules, violations and
actions. EvidenceBundle 0.17 covers the resulting trace, report and ledger
through the existing digest chain.

Policies can live in imported modules:

```argx
module main
import policies.default
```

Only reachable imported policies enter the merged package. Duplicate names
across modules fail whole-package checking.

Try the single-file and package examples:

```bash
cargo run -p argorixc -- check examples/policy_v017.argx
cargo run -p argorixc -- emit-ir examples/policy_v017.argx
cargo run -p argorixc -- emit-bytecode examples/policy_v017.argx
cargo run -p argorixc -- check-package examples/policy_project/argorix.toml
```

Run and export evidence:

```bash
cargo run -p argorix-vm -- run examples/policy_v017.argbc.json \
  --dry-run \
  --reactive \
  --inject User:ResearchAgent:tell:UserPrompt \
  --policy \
  --security-report reports/policy_v017.security.json \
  --trace-out reports/policy_v017.trace.json \
  --evidence-bundle reports/policy_v017.bundle.json

cargo run -p argorix-vm -- verify-evidence reports/policy_v017.bundle.json
cargo run -p argorix-conformance -- run conformance/suite.v017.json
```

Policy v2 does not execute external providers, open network connections, call
OpenAI or Anthropic, connect MCP/A2A, read secrets, or replace evidence with a
declaration. `simulated` remains the only executable provider.

## Argorix Lang v0.16 Module / Package System

Version 0.16 lets a protocol grow from a single file into a structured,
multi-file project without making any dependency implicit.

```text
Secure agent protocols must be modular without becoming implicit.
```

### What is a module?

A module is a single `.argx` file that declares exactly one `module` name. The
name is a dotted identifier (`agents.research`) that must match the file's path
relative to `src/`:

```text
src/agents/research.argx   ->   module agents.research
src/policies/default.argx  ->   module policies.default
src/main.argx              ->   module main      (or module app.main)
```

Module names match `[a-zA-Z_][a-zA-Z0-9_]*(.[a-zA-Z_][a-zA-Z0-9_]*)*`.

### What is a local package?

A package is a directory with an `argorix.toml` manifest and a `src/` tree:

```toml
[package]
name = "argorix-example"
version = "0.16.0"

[entry]
main = "src/main.argx"
```

`argorix.toml` is optional for compiling a single file, and required for
multi-file compilation by package root. `entry.main` names the entry file, and
every path is relative to the manifest directory. There are no absolute paths
and no external dependencies.

### Imports

Imports are declared at the top level, right after the `module` declaration:

```argx
module app.main

import agents.research
import agents.reviewer
import policies.default
import providers.contracts
import tools.search

protocol ProviderDefense {
    User -> ResearchAgent: tell UserPrompt
    ResearchAgent -> PolicyJudge: propose ToolResult
    PolicyJudge -> RuntimeGate: commit Decision
}
```

Each `import agents.research` resolves deterministically to
`src/agents/research.argx`. After resolution, the top-level declarations of
every reachable module (types, enums, agents, tools, models, providers,
policies, protocols) become globally visible. A protocol in one module may
reference agents defined in imported modules, and an imported provider contract
or policy applies to the whole package.

### How imports resolve

Resolution starts from the entry module and walks imports into a deterministic
graph. The resolver rejects:

- unknown imports (no matching file under `src/`),
- cyclic imports,
- duplicate modules,
- a module whose declared name does not match its path,
- files outside the project root,
- duplicate global symbols across modules (no silent shadowing).

Diagnostics never contain absolute paths and never depend on the current
working directory.

### Compiling a single file

```bash
cargo run -p argorixc -- check examples/provider_allowlists_v016.argx
cargo run -p argorixc -- emit-bytecode examples/provider_allowlists_v016.argx
```

### Compiling a package

```bash
cargo run -p argorixc -- check-package examples/module_project/argorix.toml
cargo run -p argorixc -- emit-ir-package examples/module_project/argorix.toml
cargo run -p argorixc -- emit-bytecode-package examples/module_project/argorix.toml
cargo run -p argorixc -- graph-package examples/module_project
```

`emit-ir-package` and `emit-bytecode-package` attach module metadata:

```json
{
  "ir_version": "0.16",
  "module": "app.main",
  "modules": [{ "name": "agents.research", "path": "src/agents/research.argx" }],
  "imports": [{ "from": "app.main", "to": "agents.research" }]
}
```

The VM, security report, and evidence bundle preserve this module metadata when
it is present, so multi-file evidence remains independently verifiable.

### Viewing the module graph

```text
app.main
├── agents.research
├── agents.reviewer
├── policies.default
├── providers.contracts
└── tools.search
```

### Why no remote package registry yet?

v0.16 is deliberately offline. A remote registry, package downloads, external
dependencies, and secrets are explicitly out of scope: a secure agent protocol
must remain independently auditable, and remote resolution would make
dependencies implicit and unverifiable. The module system is the local,
deterministic foundation those features would later build on.

### Security rules and limitations

- No relative imports (`import ./agents/research`).
- No import aliases (`import agents.research as research`).
- No remote registry, package downloads, or external dependencies.
- No absolute paths in manifests.
- `simulated` remains the only executable provider; external providers stay
  disabled and non-executable in multi-file projects exactly as in single-file
  ones.

```text
Secure agent protocols must be modular without becoming implicit.
```

## Provider contract allowlists v0.12

External provider contracts may declare future target and capability permissions:

```argx
provider OpenAI {
    kind external
    enabled false
    dry_run_only true
    requires feature_flag
    requires approval

    allowed_targets { GuardModel }
    allowed_capabilities { model.invoke }
}
```

The two optional blocks may appear in either order after the requirement clauses, at most once each.

Duplicate blocks fail during parsing.

Duplicate elements fail during semantic validation at the repeated element.

Targets must resolve to a global tool or model. A name shared by a tool and model is an ambiguous allowlist target.

Capabilities must exist globally. Every allowlisted target must match at least one listed capability when the capability list is populated.

Empty lists mean **zero future permissions**. They are never wildcards.

Contracts without blocks remain compatible with v0.11 source and lower to empty arrays.

> Allowlisted does not mean executable.

Tools and models still use only `simulated`. Attempts to execute an external contract remain fail-closed and emit:

```text
ExternalProviderExecutionBlocked
```

Use `--provider-contracts` to print indented allowlists.

Empty lists are shown as `none`.

JSON preserves list order in `provider_contracts`.

## External adapter contracts v0.11

Module-level provider declarations describe future external adapters without making them executable:

```argx
provider OpenAI {
    kind external
    enabled false
    dry_run_only true
    requires feature_flag
    requires approval
}
```

`ProviderRegistry` keeps two separate maps:

- executable providers,
- declarative adapter contracts.

`simulated` is registered by default as the only executable provider and must not be declared as a provider contract.

External contracts never implement `Provider`.

Every external contract must be:

- disabled,
- dry-run-only,
- feature-flag gated,
- explicitly approved.

Tools and models still accept only `simulated`.

Attempted external execution is blocked fail-closed and leaves the trace ledger available for inspection.

In IR and Bytecode v0.11, the top-level `providers` collection represents declarative provider contracts, not executable provider instances.

Executable providers are runtime registry entries and appear separately in VM output.

Bytecode loads contracts before scheduling and emits:

- `ProviderContractDeclared`
- `ProviderContractValidated`
- `ProviderContractRejected`

A blocked call emits:

- `ExternalProviderExecutionBlocked`

Use `--provider-contracts` for the separated textual report.

Reactive JSON always includes `provider_contracts`; `providers` contains only executable providers.

## Provider boundary v0.10

The standalone `argorix_provider` crate defines:

- synchronous provider contracts,
- typed tool/model requests and responses,
- provider errors,
- provider registry.

`ProviderRegistry::default()` registers only `simulated`.

Tools may omit their provider in source:

```argx
tool WebSearch {
    capability web.search
    input UserPrompt
    output ToolResult
}
```

The AST preserves this omission as `None`.

Semantic validation permits only `simulated`.

IR resolves the omitted value to `simulated`.

IR and Bytecode 0.10 therefore always carry an explicit provider for both tools and models.

Reactive calls follow:

```text
VM -> ProviderRegistry -> SimulatedProvider -> response -> trace ledger
```

`SimulatedProvider` accepts only `dry_run: true`, performs no network or external execution, and returns typed simulated responses.

Unknown providers, provider errors, or invalid responses fail closed, preserve the runtime ledger, and activate an applicable failure mode.

Use `--providers` to print registered providers and ordered calls.

Reactive JSON includes `providers` and `provider_calls`.

Audit events include:

- `ProviderRegistered`
- `ProviderSelected`
- `ProviderRequestCreated`
- `ProviderResponseReceived`
- `ProviderDryRunEnforced`
- `ProviderBoundaryDenied`

## Global policies and failure modes v0.9

Policies are module-level assertions verified after deterministic reactive execution:

```argx
assert no_unhandled_messages
assert all_tool_calls_traced
assert all_model_calls_traced
assert all_intrinsics_traced
assert halt_requires_trace
assert runtime_status completed

failure PolicyViolation { action block trace required }
failure ToolDenied { action review trace required }
failure ModelDenied { action review trace required }
```

The compiler rejects:

- unknown assertions,
- unsupported runtime status targets,
- invalid failure actions,
- duplicate failures,
- failure declarations without `trace required`.

Failure actions are limited to:

- `block`
- `review`
- `halt`

IR and Bytecode 0.9 preserve these declarations and emit:

- `DeclareAssertion`
- `DeclareFailure`
- `VerifyAssertion`
- `PolicyReport`

The VM evaluates every assertion against runtime state and the trace ledger, emits verification events, activates the declared failure mode on violation, and returns a structured `policy_report`.

Use `--policy` for the text report or `--json` for the complete machine-readable report.

## Simulated model adapter v0.8

Models are module-level contracts with provider, capability, input, and output:

```argx
model GuardModel {
    provider simulated
    capability model.invoke
    input ToolResult
    output Decision
}
```

Agents authorize models in `models` and invoke them with:

```argx
ask ModelName with binding
```

Only provider `simulated` is accepted.

The compiler checks:

- model uniqueness,
- provider,
- capability,
- type contracts,
- agent authorization,
- approval,
- binding,
- handler input compatibility.

IR and Bytecode 0.8 add model registries plus:

- `DeclareModel`
- `AuthorizeModel`
- `AskModel`

The VM creates a `ModelCallEnvelope`, checks authorization and capability again, and records requested, allowed/denied, and dry-run-result events.

No API, network, or real model is called.

## Controlled tools v0.7

Tools are module-level contracts:

```argx
tool WebSearch {
    capability web.search
    input UserPrompt
    output ToolResult
}
```

Agents explicitly authorize tools and call them only from handlers:

```argx
tools { WebSearch }

on UserPrompt as prompt {
    call WebSearch with prompt
}
```

The compiler verifies:

- tool uniqueness,
- capability contracts,
- type contracts,
- agent authorization,
- required capability,
- approval,
- handler binding,
- input message compatibility.

IR 0.7 includes tools and call instructions.

Bytecode 0.7 lowers these contracts to:

- `DeclareTool`
- `AuthorizeTool`
- `CallTool`

The VM never executes a real tool in v0.7.

It creates a `ToolCallEnvelope`, checks authorization and capability again, and records:

- `ToolCallRequested`
- `ToolCallAllowed`
- `ToolCallDenied`
- `ToolCallDryRunResult`

The `--tools` flag prints the resulting controlled call ledger.

## Runtime intrinsics v0.6

Handlers may invoke two built-in runtime operations:

```argx
on Decision as decision {
    marron(decision)
    facu(decision)
    trace decision
    halt
}
```

`facu(binding)` requires `state.write`.

It updates the agent's handled-message metadata and creates a deterministic checkpoint containing:

- message ID,
- message type,
- binding,
- checkpoint index.

`marron(binding)` requires `runtime.guard`.

It verifies that the current envelope:

- was delivered by the scheduler,
- belongs to the active handler,
- contains non-empty `id`,
- contains non-empty `from`,
- contains non-empty `to`,
- contains non-empty `act`,
- contains non-empty `message_type`.

Failures transition the runtime to `failed` while retaining the trace ledger.

Only `facu` and `marron` are recognized.

Both must use the exact binding declared by the enclosing handler.

## Reactive handlers v0.5

Agents can react to received message types:

```argx
agent PromptScanner {
    receives UserPrompt
    sends Finding to PolicyJudge

    on UserPrompt as prompt {
        emit Finding to PolicyJudge
    }
}
```

Handlers support only:

- `emit MessageType to AgentName`
- `trace binding`
- `halt`

The compiler verifies:

- input types,
- matching `receives` contracts,
- matching `sends` contracts,
- destinations,
- trace bindings,
- duplicate handlers,
- `runtime.halt` capability,
- approval policy.

Reactive execution requires an initial message in this format:

```text
--inject FROM:TO:ACT:MESSAGE_TYPE
```

Payloads are `{}` in v0.5.

The scheduler delivers the injected envelope, executes the matching handler, queues emitted messages, and repeats until `halt` or until no pending messages remain.

## Bytecode

`argorix_bytecode` lowers validated IR into JSON-serializable bytecode:

```json
{
  "bytecode_version": "0.12",
  "language": "Argorix Lang",
  "module": "Argorix.Security",
  "providers": [],
  "agents": [],
  "capabilities": [],
  "instructions": [
    {
      "op": "SendMessage",
      "from": "PromptScanner",
      "to": "PolicyJudge",
      "act": "propose",
      "message_type": "Finding"
    },
    {
      "op": "End"
    }
  ]
}
```

The instruction model supports:

- `DeclareAgent`
- `DeclareProviderContract`
- `DeclareCapability`
- `DeclareProtocol`
- `DeclareHandler`
- `EmitMessage`
- `TraceValue`
- `HandlerHalt`
- `EndHandler`
- `InvokeIntrinsic`
- `DeclareTool`
- `AuthorizeTool`
- `CallTool`
- `DeclareModel`
- `AuthorizeModel`
- `AskModel`
- `DeclareAssertion`
- `DeclareFailure`
- `VerifyAssertion`
- `PolicyReport`
- `SendMessage`
- `RequireCapability`
- `RequireApproval`
- `Trace`
- `Halt`
- `End`

Lowering emits declarations and security requirements before protocol message instructions.

`Halt` is supported by the format and causes dry-run execution to stop with an error. The compiler does not emit it for a valid protocol merely because a capability happens to be named `runtime.halt`.

## Bytecode verification

The verifier requires:

- Bytecode version `0.17` for newly compiled programs. Versions `0.3`, `0.5`,
  `0.6`, `0.7`, `0.8`, `0.9`, `0.10`, `0.11`, `0.12`, `0.13`, `0.14`, and `0.15`
  remain accepted for compatibility. Module metadata (`modules`/`imports`)
  requires version `0.16`.
- At least one agent.
- At least one protocol or `SendMessage`.
- Complete, non-empty message fields.
- Known or explicitly external senders and receivers.
- Existing agents for approval and capability requirements.
- A final `End` instruction.

Allowed external entities remain:

- `User`
- `System`
- `Runtime`
- `Memory`
- `Tool`

## VM runtime

The VM verifies bytecode again before execution and initializes one FIFO mailbox for every internal agent.

The deterministic scheduler converts each `SendMessage` into a serializable message envelope:

```json
{
  "id": "msg_001",
  "from": "User",
  "to": "PromptScanner",
  "act": "tell",
  "message_type": "UserPrompt",
  "payload": {}
}
```

Each internal message is scheduled, delivered to the receiver mailbox, and processed in bytecode order.

External entities do not receive internal mailboxes.

No network calls, tools, LLMs, or concurrent tasks are executed.

Example text output:

```text
Argorix VM v0.17

Loaded bytecode: examples/prompt_defense.argbc.json
Execution mode: dry-run

Step 1: User --tell UserPrompt--> PromptScanner
Step 2: PromptScanner --propose Finding--> PolicyJudge
Step 3: PolicyJudge --commit Decision--> RuntimeGate

Security checks: passed
Trace: generated
Status: completed
```

With `--mailboxes`, the CLI shows initialization and the three scheduler phases for each message.

With `--json`, execution returns runtime state summaries and the complete ledger:

```json
{
  "vm_version": "0.12",
  "status": "completed",
  "mode": "reactive-dry-run",
  "scheduler": "deterministic",
  "steps": [
    {
      "index": 1,
      "from": "User",
      "to": "PromptScanner",
      "act": "tell",
      "message_type": "UserPrompt",
      "status": "ok"
    }
  ],
  "mailboxes": [
    {
      "agent": "PromptScanner",
      "delivered": 1,
      "processed": 1
    }
  ],
  "events": [],
  "security_checks": "passed"
}
```

Runtime status progresses through:

- `initialized`
- `running`
- `completed`
- `failed`

Reactive JSON uses `vm_version: "0.17"` and
`mode: "reactive-dry-run"`. Each step records the agent, handled message,
emitted messages, traced bindings, and whether the handler halted execution.

The public `RuntimeState` retains:

- agents,
- mailboxes,
- pending messages,
- completed-step count,
- status,
- `TraceLedger`.

The ledger records:

- `VmStarted`
- declarations,
- message scheduling,
- delivery,
- processing,
- `VmCompleted`
- `VmFailed`

Because the scheduler mutates a caller-owned state, failure diagnostics do not discard the ledger.

Reactive JSON uses:

```text
vm_version: "0.12"
mode: "reactive-dry-run"
```

Each step records:

- agent,
- handled message,
- emitted messages,
- traced bindings,
- whether the handler halted execution.

Tool-aware JSON includes `tool_calls`, with:

- agent,
- tool,
- capability,
- authorization status,
- dry-run mode.

Model-aware JSON includes `model_calls`, with:

- agent,
- model,
- simulated provider,
- capability,
- authorization status,
- dry-run mode.

Policy-aware JSON includes:

- `policy_report.status`,
- one result per assertion,
- activated failure modes.

The trace ledger also records assertion and failure declarations, assertion verification or failure, failure-mode activation, and policy-report generation.

## Source security model

Argorix v0.2 security remains enforced before bytecode generation:

- Capabilities have `safe`, `restricted`, or `dangerous` levels.
- Restricted and dangerous capabilities require `approval granted`.
- Every used capability must exist in the module registry.
- Protocol steps must match agent `sends` and `receives` contracts.

Registry-free v0.1 sources require explicit compatibility mode:

```bash
cargo run -p argorixc -- --legacy-capabilities check examples/prompt_defense.argx
```

## Workspace

```text
crates/argorixc          Source compiler CLI
crates/argorix_parser    Lexer, parser, AST, spans, diagnostics
crates/argorix_semantics Source-level security and protocol verifier
crates/argorix_ir          Argorix IR 0.17 with policy and module metadata
crates/argorix_bytecode    IR lowering and Bytecode 0.3 through 0.17 verifier
crates/argorix_module      Manifest parsing and deterministic module resolution
crates/argorix_conformance Official direct-API Conformance Suite runner
crates/argorix_provider  Executable providers, adapter contracts, and registry
crates/argorix_vm        VM, preserved outcomes, ledger, security reports
crates/argorix-vm        Bytecode VM CLI
examples                 Source and bytecode fixtures
tests                    End-to-end compiler tests
```

## Examples

### Valid source and bytecode fixtures

- `prompt_defense_v02.argx`: valid secure source program.
- `prompt_defense_v05.argx`: valid reactive source program.
- `prompt_defense_v05.argbc.json`: generated reactive Bytecode 0.5.
- `prompt_defense_v06.argx`: reactive program with state and causal guards.
- `prompt_defense_v06.argbc.json`: generated Bytecode 0.6 fixture.
- `tool_call_v07.argx`: valid controlled-tool source program.
- `tool_call_v07.argbc.json`: generated Bytecode 0.7 fixture.
- `model_call_v08.argx`: valid simulated-model source program.
- `model_call_v08.argbc.json`: generated Bytecode 0.8 fixture.
- `policy_assertions_v09.argx`: valid global-policy source program.
- `policy_assertions_v09.argbc.json`: generated Bytecode 0.9 fixture.
- `provider_boundary_v010.argx`: valid provider-boundary source program.
- `provider_boundary_v010.argbc.json`: generated Bytecode 0.10 fixture.
- `provider_contracts_v011.argx`: valid disabled external adapter contract.
- `provider_contracts_v011.argbc.json`: generated Bytecode 0.11 fixture.
- `provider_allowlists_v012.argx`: valid model allowlist contract.
- `provider_allowlists_v012.argbc.json`: generated Bytecode 0.12 model fixture.
- `provider_allowlists_v013.argx`: v0.12-compatible allowlist source compiled by v0.13.
- `provider_allowlists_v013.argbc.json`: generated Bytecode 0.13 security-report fixture.
- `provider_allowlists_v014.argx`: Evidence Bundle and offline-verification source fixture.
- `provider_allowlists_v014.argbc.json`: generated Bytecode 0.14 evidence fixture.
- `provider_allowlists_v015.argx`: Conformance Suite release source fixture.
- `provider_allowlists_v015.argbc.json`: generated Bytecode 0.15 fixture.
- `provider_allowlists_v016.argx`: single-file v0.16 source fixture.
- `provider_allowlists_v016.argbc.json`: generated Bytecode 0.16 fixture.
- `module_project/`: multi-file v0.16 package (`argorix.toml` + `src/`).
- `atrust_handshake_v029.argx`: valid v0.29 ATrust Handshake Dry-Run source program.
- `atrust_handshake_v029.argbc.json`: generated Bytecode 0.29 handshake fixture.
- `bridge_contracts_v031.argx`: valid v0.31 MCP / A2A Bridge Contracts source program.
- `bridge_contracts_project/`: multi-file v0.31 package with imported `mcp_bridge_contract` and `a2a_bridge_contract` blocks.
- `invalid_bridge_contracts/`: rejected bridge contract forms (open network, enabled execution, secret/key material, api_key auth, security claims, unbound references, duplicate names).
- `trust_ledger_v030.argx`: valid v0.30 Trust Ledger Hash Chain source program.
- `trust_ledger_v030.argbc.json`: generated Bytecode 0.30 trust ledger fixture.
- `trust_ledger_project/`: multi-file v0.30 package with an imported `trust_ledger`.
- `invalid_trust_ledgers/`: fixtures that must fail semantic checking.
- `policy_v017.argx`: single-file Policy Language v2 source fixture.
- `policy_v017.argbc.json`: generated Bytecode 0.17 policy fixture.
- `policy_project/`: multi-file v0.17 package with an imported policy.
- `invalid_policies/`: stable parser and semantic policy diagnostics.
- `invalid_modules/`: package fixtures for each module-resolution failure.
- `conformance/suite.v016.json`: official portable v0.16 suite.
- `conformance/suite.v017.json`: official portable v0.17 Policy v2 suite.
- `provider_allowlists_tools_v012.argx`: valid tool allowlist contract.
- `provider_allowlists_tools_v012.argbc.json`: generated Bytecode 0.12 tool fixture.

### Failure fixtures

- `provider_allowlist_unknown_target.argx`: unknown target failure.
- `provider_allowlist_unknown_capability.argx`: unknown capability failure.
- `provider_allowlist_duplicate_target.argx`: duplicate target failure.
- `provider_allowlist_duplicate_capability.argx`: duplicate capability failure.
- `provider_allowlist_incompatible_capability.argx`: target/capability mismatch.
- `provider_allowlist_external_execution_still_blocked.argx`: allowlisted external execution failure.
- `provider_external_enabled.argx`: enabled external-contract failure.
- `provider_external_missing_feature_flag.argx`: missing feature gate failure.
- `provider_external_missing_approval.argx`: missing approval gate failure.
- `provider_external_used_by_model.argx`: external model-provider failure.
- `provider_external_used_by_tool.argx`: external tool-provider failure.
- `tool_invalid_provider.argx`: unsupported tool provider failure.
- `model_invalid_provider_v010.argx`: unsupported model provider failure.
- `assert_unknown.argx`: unknown assertion failure.
- `failure_invalid_action.argx`: unsupported failure action.
- `failure_missing_trace.argx`: missing mandatory failure trace.
- `invalid_bytecode_missing_end.argbc.json`: verifier failure fixture.
- `restricted_without_approval.argx`: source approval failure.
- `unknown_capability.argx`: undeclared capability failure.

## Argorix Lang v0.31 MCP / A2A Bridge Contracts

v0.31 adds two top-level declarations, `mcp_bridge_contract` and
`a2a_bridge_contract`, that describe **allowed interoperability surfaces** for an
agent. They declare *how* an agent could interoperate with external MCP servers
(tools, resources, prompts) or with another agent over A2A — but declaring a
bridge never connects it.

Guiding principle: **a bridge may be declared before it is connected.** Extended:
*bridge contracts describe allowed interoperability surfaces; they do not open
network access by themselves.*

- **`mcp_bridge_contract`** binds an `agent`, its `passport`, its
  `atrust_identity`, and an `atrust_boundary`, then declares the MCP `tools`,
  `resources`, and `prompts` the agent could reach. `protocol` is always `mcp`;
  `transport` is `declared_only` or `disabled`; `direction` is
  `inbound`/`outbound`/`bidirectional`.
- **`a2a_bridge_contract`** binds an `initiator` and `responder` (distinct
  agents), their passports and identities, a prior `atrust_handshake`, the
  `trust_ledger` that records that handshake, and an `atrust_boundary`, then
  declares the `message_contracts` (declared message types) and `capabilities`
  the bridge could exchange. `protocol` is always `a2a`.

Both blocks pin a closed security boundary that semantic analysis, the bytecode
verifier, and Policy v2 all enforce:

- `network denied` — no runtime network is opened.
- `external_execution disabled`, `tool_execution disabled` (MCP),
  `agent_execution disabled` (A2A) — nothing is executed.
- `secret_material denied`, `key_material denied` — no secret or key material.
- `authentication none | declared_only` — no API key, OAuth, or bearer token is
  used.
- `authorization policy_bound | declared_only`, `evidence required`,
  `security_claims none`.

A declared bridge makes **no** connectivity claim:

- **MCP bridge declared ≠ MCP connected** — no MCP server exists and no tool runs.
- **A2A bridge declared ≠ A2A messages sent** — no agent communication occurred
  and no agent is executed.
- `network denied` means there is no runtime network.
- `authentication none`/`declared_only` means no API key, OAuth, or bearer token
  was used.

v0.31 explicitly **does not** add HTTP/websocket/SSE clients, a stdio or JSON-RPC
MCP runtime, an A2A runtime, OpenAI/Anthropic/Google API calls, external tool or
shell execution, environment-variable or secret/API-key reads, OAuth, wallets,
real DID resolution, real credential or handshake execution, signing, signature
verification, encryption, blockchain, or consensus. There are **no** executable
bridge instructions (`OpenMcpConnection`, `CallMcpTool`, `SendA2AMessage`,
`ExecuteAgent`, `OpenNetwork`, `ReadApiKey`, …); bridge contracts are
metadata/evidence only. The simulated provider remains the only executable
provider.

Bridge contracts relate to the rest of the language as governance metadata:

- relation with the **Agent Passport** (v0.19): a bridge binds the agent's
  declared passport, and the passport must belong to that agent.
- relation with **ATrust Identity** (v0.27): a bridge binds the agent's
  `atrust_identity`, whose subject must be the agent and whose boundary must
  match the bridge boundary.
- relation with **ATrust Handshake** (v0.29): an A2A bridge references a prior
  dry-run handshake that must bind its initiator and responder.
- relation with the **Trust Ledger** (v0.30): the A2A bridge's `trust_ledger`
  must include a `handshake` entry for the referenced handshake.
- relation with the **Evidence Bundle** (v0.14): bridge metadata flows into the
  SecurityReport and EvidenceBundle and verifies offline; bundles remain
  compatible with 0.29 and 0.30.
- relation with **Policy v2** (v0.17): the rules `mcp_bridge_contracts_declared`,
  `a2a_bridge_handshakes_bound`, `a2a_bridge_trust_ledgers_bound`,
  `mcp_bridge_network_denied`, `a2a_bridge_agent_execution_disabled`, and the
  other `*_bridge_*` rules require the declared, closed-boundary surface.

The SecurityReport (v0.31) summarizes declared bridges under
`mcp_bridge_contracts` and `a2a_bridge_contracts` (`total`, `names`, `protocols`,
`directions`, `network.denied`, `external_execution.disabled`,
`tool_execution.disabled` / `agent_execution.disabled`, `security_claims.none`).
It never emits `mcp_connected`, `a2a_connected`, `tool_verified`,
`agent_verified`, or `secure_bridge`. **Bridge declared does not mean bridge
connected.**

```argx
mcp_bridge_contract ResearchMcpBridge {
  agent ResearchAgent
  passport ResearchPassport
  identity ResearchIdentity
  boundary AgentTrustBoundary

  transport declared_only
  protocol mcp
  direction outbound

  tools ["search.read", "memory.read"]
  resources ["docs.public", "kb.public"]
  prompts ["research.summary"]

  network denied
  external_execution disabled
  tool_execution disabled
  secret_material denied
  key_material denied

  authentication none
  authorization policy_bound
  evidence required
  security_claims none

  purpose ["mcp", "bridge-contract", "dry-run"]
  notes "metadata only; no MCP runtime"
}

a2a_bridge_contract ResearchA2ABridge {
  initiator ResearchAgent
  responder VerifierAgent

  initiator_passport ResearchPassport
  responder_passport VerifierPassport

  initiator_identity ResearchIdentity
  responder_identity VerifierIdentity

  handshake ResearchHandshake
  trust_ledger ATrustLedger
  boundary AgentTrustBoundary

  protocol a2a
  transport declared_only
  direction bidirectional

  message_contracts ["ResearchRequest", "ResearchResponse"]
  capabilities ["ask.llm", "respond.safe"]

  network denied
  external_execution disabled
  agent_execution disabled
  secret_material denied
  key_material denied

  authentication none
  authorization policy_bound
  evidence required
  security_claims none

  purpose ["a2a", "bridge-contract", "dry-run"]
  notes "metadata only; no A2A runtime"
}
```

See `examples/bridge_contracts_v031.argx` (single file) and
`examples/bridge_contracts_project/` (multi-file package) for complete programs,
and `examples/invalid_bridge_contracts/` for the rejected forms.

## Argorix Lang v0.30 Trust Ledger Hash Chain

v0.30 adds a top-level `trust_ledger` declaration that preserves an ordered,
auditable **hash chain** of trust evidence linking the earlier ATrust artifacts
(identities, credential contracts, handshakes) and the evidence bundle.

A `trust_ledger` is an **audit structure, not a blockchain and not a cryptographic
trust guarantee**. The guiding principle: *trust evidence may be linked before it
is trusted — no trust event becomes authority merely because it is chained.*

```argx
trust_ledger ATrustLedger {
  scope local
  mode dry_run
  hash_algorithm sha256
  chain_policy append_only

  entries [
    {
      id "entry-001"
      kind identity
      subject ResearchIdentity
      previous_hash "GENESIS"
      entry_hash "sha256:declared-entry-001"
      evidence_ref "bundle:identity"
    },
    {
      id "entry-002"
      kind handshake
      subject ResearchHandshake
      previous_hash "sha256:declared-entry-001"
      entry_hash "sha256:declared-entry-002"
      evidence_ref "bundle:handshake"
    }
  ]

  chain_root "sha256:declared-entry-002"

  network denied
  key_material denied
  secret_material denied
  execution disabled
  evidence required
  security_claims none

  purpose ["trust-ledger", "evidence-chain", "dry-run"]
}
```

### Hash chain vs. blockchain vs. immutability

A declared hash chain links entries by recording each entry's `previous_hash` and
`entry_hash`, with `chain_root` pinned to the final entry. The compiler checks the
linking is consistent (`previous_hash` of the first entry is `GENESIS`, every later
entry's `previous_hash` equals the prior `entry_hash`, and `chain_root` matches the
last `entry_hash`). That is **all** it does. v0.30 explicitly **does not**:

- implement a blockchain, consensus, mining, staking, or peer-to-peer networking;
- broadcast to a network or open any connection;
- sign entries, verify signatures, generate keys, read keys, or read secrets;
- resolve DIDs, verify credentials, verify presentations, or execute handshakes;
- compute real cryptographic digests as a security guarantee.

Therefore: **trust ledger declared does not mean immutable ledger; hash chain
declared does not mean tamper-proof, blockchain, identity verified, credential
verified, or handshake secure. `post_quantum_ready` does not mean
`post_quantum_secure`.**

### Fields and allowed values

| field | allowed values |
| --- | --- |
| `scope` | `local`, `package`, `bundle` |
| `mode` | `dry_run`, `declared_only` |
| `hash_algorithm` | a declared `crypto` of kind `hash` that is not `denied` |
| `chain_policy` | `append_only`, `declared_only` |
| `network` | `denied` |
| `key_material` / `secret_material` | `denied` |
| `execution` | `disabled` |
| `evidence` | `required` |
| `security_claims` | `none` |

Each entry requires `id`, `kind` (`identity`, `credential`, `handshake`, `evidence`,
`policy`, or `custom`), `subject`, `previous_hash`, `entry_hash`, and `evidence_ref`.
An `identity`/`credential`/`handshake`/`policy` entry's `subject` must reference a
declared artifact of that kind; `entry_hash` must use the `hash_algorithm` prefix
(e.g. `sha256:`). `purpose` is a required, non-empty array of non-empty strings.

### Policy v2, SecurityReport & EvidenceBundle

v0.30 adds Policy v2 rules including `trust_ledgers_declared`,
`trust_ledger_hash_algorithm_declared`, `trust_ledger_chain_valid`,
`trust_ledger_entries_bound`, `trust_ledger_append_only`, the boundary rules
(`trust_ledger_network_denied`, `…_key_material_denied`, `…_secret_material_denied`,
`…_execution_disabled`, `…_evidence_required`), and the absence rules
(`trust_ledger_security_claims_absent`, `trust_ledger_blockchain_absent`,
`trust_ledger_signature_absent`). See
[`examples/trust_ledger_v030.argx`](examples/trust_ledger_v030.argx).

The SecurityReport (v0.30) reports the count of declared ledgers under
`trust_ledgers` and never emits any `immutable` / `tamper_proof` /
`blockchain_verified` / `identity_verified` / `credential_verified` /
`handshake_secure` / `post_quantum_secure` claim. The EvidenceBundle (v0.30)
covers the ledger metadata through the Bytecode, trace, report, and ledger
digests, and still verifies every bundle from `0.14` onward.

Versions advance together to `0.30` (workspace, IR, Bytecode, VM trace,
SecurityReport, EvidenceBundle, ConformanceSuite) while Bytecode `0.29` and
EvidenceBundle `0.29` — and every earlier feature — remain fully supported.

## Argorix Lang v0.29 ATrust Handshake Dry-Run

v0.29 adds a top-level `atrust_handshake` declaration that lets a program declare
and **simulate** an ATrust handshake flow between two agents. It is the natural
successor to the ATrust boundary (`v0.26`), identity dry-run (`v0.27`), and
credential contracts (`v0.28`).

A handshake binds together everything that must exist *before* any trust exchange
could ever run:

- an `initiator` agent and a distinct `responder` agent,
- an `initiator_identity` and `responder_identity` (`atrust_identity` declarations
  whose `subject` must match the corresponding agent),
- one or more `credential_contracts` (`atrust_credential_contract` declarations,
  each bound to a participant identity),
- an `atrust_boundary` shared by both identities and all referenced credentials,
- a `did_method` shared by both identities and all referenced credentials and
  allowed by the boundary's `did_methods`.

```argx
atrust_handshake ResearchHandshake {
  initiator ResearchAgent
  responder VerifierAgent

  initiator_identity ResearchIdentity
  responder_identity VerifierIdentity

  credential_contracts ["ResearchCredential"]

  boundary AgentTrustBoundary
  method argorix

  mode dry_run
  direction mutual

  challenge declared_only
  response declared_only
  transcript evidence_only

  verification declared_only
  resolution disabled
  network denied

  key_material denied
  secret_material denied
  execution disabled

  evidence required
  security_claims none

  purpose ["handshake", "identity-link", "credential-contract", "dry-run"]
  notes "metadata only; no real handshake"
}
```

### What a handshake dry-run is — and is not

A `dry_run` handshake is **evidence of a declared trust flow, not proof of secure
communication**. The compiler validates the declared shape and bindings, lowers the
metadata into IR, Bytecode, the VM trace, the SecurityReport, and the EvidenceBundle,
and stops there. v0.29 explicitly **does not**:

- execute a real handshake, or emit any `RunHandshake`/`HandshakeInit`/`HandshakeAck`
  instruction;
- generate nonces or real challenges, sign challenges, or verify responses;
- verify credentials, presentations, or real identities;
- resolve DIDs, query ledgers, or open network connections;
- sign, verify signatures, encrypt, decrypt, generate keys, read keys, or read secrets.

Therefore: **handshake dry-run does not mean handshake executed, agents
authenticated, credential verified, or secure channel established. `post_quantum_ready`
does not mean `post_quantum_secure`.**

### Allowed field values

| field | allowed values |
| --- | --- |
| `mode` | `dry_run` |
| `direction` | `one_way`, `mutual` |
| `challenge` | `declared_only`, `disabled` |
| `response` | `declared_only`, `disabled` |
| `transcript` | `metadata_only`, `evidence_only` |
| `verification` | `declared_only`, `disabled` |
| `resolution` | `disabled`, `embedded`, `local` |
| `network` | `denied` |
| `key_material` / `secret_material` | `denied` |
| `execution` | `disabled` |
| `evidence` | `required` |
| `security_claims` | `none` |

`purpose` is a required, non-empty array of non-empty strings; `notes` is optional
but, when present, must be non-empty.

### Policy v2 integration

v0.29 adds named handshake rules to Policy Language v2, e.g.
`atrust_handshake_declared`, `atrust_handshake_mode_dry_run`,
`atrust_handshake_challenge_declared_only`, `atrust_handshake_network_denied`,
`atrust_handshake_execution_disabled`, `atrust_handshake_evidence_required`, and
`atrust_handshake_security_claims_absent` (see
[`examples/atrust_handshake_v029.argx`](examples/atrust_handshake_v029.argx) for the
full set).

### SecurityReport & EvidenceBundle

The SecurityReport (v0.29) reports the number of declared handshakes under
`atrust_handshakes` and never emits any `handshake_secure` / `identity_verified` /
`credential_verified` / `presentation_verified` / `post_quantum_secure` claim. The
EvidenceBundle (v0.29) covers the handshake metadata through the Bytecode, trace,
report, and ledger digests, while still verifying every bundle from `0.14` onward.

Versions advance together to `0.29` (workspace, IR, Bytecode, VM trace,
SecurityReport, EvidenceBundle, ConformanceSuite) while Bytecode `0.28` and
EvidenceBundle `0.28` — and every earlier feature — remain fully supported.

## Roadmap

1. `v0.1` — compiled structure.
2. `v0.2` — compiled security.
3. `v0.3` — compiled execution through bytecode and dry-run VM.
4. `v0.4` — agent mailboxes, deterministic scheduling, runtime state, trace ledger.
5. `v0.5` — declarative handlers and reactive dry-run execution.
6. `v0.6` — controlled agent state, deterministic checkpoints, causal guards.
7. `v0.7` — declared, authorized, capability-controlled tool calls.
8. `v0.8` — declared, authorized, simulated model invocation.
9. `v0.9` — compiled global policies, failure modes, and runtime reports.
10. `v0.10` — audited provider boundary and simulated provider registry.
11. `v0.11` — disabled external adapter contracts and conformance checks.
12. `v0.12` — declarative provider target/capability allowlists.
13. `v0.13` — preserved execution outcomes and deterministic security reports.
14. `v0.14` — portable Evidence Bundles and offline semantic verification.
15. `v0.15` — official portable, data-driven Conformance Suite.
16. `v0.16` — local Module / Package System with deterministic resolution.
17. `v0.17` — Policy Language v2 with named require/deny rules and evidence-backed actions.
17. `v0.17+` — sandboxed provider work.
18. `v0.26` — ATrust Boundary Contracts.
19. `v0.27` — ATrust Identity Dry-Run.
20. `v0.28` — ATrust Credential Contracts.
21. `v0.29` — ATrust Handshake Dry-Run (declared, simulated trust flow; no real crypto/network).
22. `v0.30` — Trust Ledger Hash Chain (declared, auditable evidence chain; no blockchain/consensus/signing).
23. Optional WASM/native backends.
24. Progressive self-hosting in Argorix Lang.

## Security posture

Argorix Lang is designed to fail closed.

Current versions do not execute real tools, real models, network calls, MCP/A2A calls, shells, or external provider systems.

The VM validates bytecode, simulates protocol message flow, records runtime evidence, and preserves the trace ledger for inspection.

External provider contracts are declarative only until sandboxed provider work is introduced in later versions.

## Project philosophy

Secure AI-agent systems should be:

- explicit,
- inspectable,
- testable,
- traceable,
- policy-aware,
- governed at the runtime boundary.

Argorix Lang is an open-source exploration of that direction.

> Rust is the forge. Argorix Lang is the sword.

## Demo1

A chatbot governed by Argorix Lang v1.0 — contracts, policy, evidence and
fail-closed enforcement (including input-boundary prompt-injection / secret
exfiltration blocking). Source: [`demo/argorix-chatbot-runtime/`](demo/argorix-chatbot-runtime/).

<video src="https://github.com/argorixlabs/argorixlang/raw/main/videodemo/Demo1.mp4" controls width="720"></video>

▶️ If the player does not load above, [watch Demo1 directly](videodemo/Demo1.mp4).
