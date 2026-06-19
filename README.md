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

**Version:** `0.16`
**Status:** early alpha  
**License:** Apache-2.0  
**Implementation:** Rust  
**Execution mode:** dry-run / simulated runtime only  

Version 0.16 adds a local Module / Package System: multi-file projects with
deterministic, auditable import resolution. Modules are explicit, never
implicit — there is no remote registry, no external dependencies, and no
network. Single-file compilation continues to work unchanged, and external
providers remain disabled and cannot execute.

```text
argorix.toml + src/*.argx
  -> module resolution (deterministic graph)
  -> whole-package semantic and security verification
  -> lexer / parser / AST
  -> Argorix IR 0.16 (with module metadata)
  -> Argorix Bytecode 0.16 (with module metadata)
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
  -> global policy verification
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

- Bytecode version `0.16` for newly compiled programs. Versions `0.3`, `0.5`,
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
Argorix VM v0.16

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

Reactive JSON uses `vm_version: "0.16"` and
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
crates/argorix_ir          Argorix IR 0.16 with module metadata
crates/argorix_bytecode    IR lowering and Bytecode 0.3 through 0.16 verifier
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
- `invalid_modules/`: package fixtures for each module-resolution failure.
- `conformance/suite.v016.json`: official portable v0.16 suite.
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
17. `v0.17+` — sandboxed provider work.
18. Optional WASM/native backends.
19. Progressive self-hosting in Argorix Lang.

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
