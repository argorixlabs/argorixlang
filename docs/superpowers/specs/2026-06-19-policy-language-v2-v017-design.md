# Argorix Lang v0.17 Policy Language v2 Design

## Objective

Argorix Lang v0.17 replaces isolated policy assertions as the only policy
authoring surface with named, composable top-level policy blocks:

```argx
policy ProviderSafety {
    deny external_execution

    on violation {
        action block
        trace required
    }
}
```

Legacy assertions remain valid and retain their existing AST, IR, Bytecode,
and runtime representation:

```argx
assert no_unhandled_messages
assert all_tool_calls_traced
assert runtime_status completed
```

The governing v0.17 principle is:

```text
Security policy must be declared as code, compiled as intent, and enforced as evidence.
```

The release remains local, deterministic, offline, and simulated-only. It
performs no network access, reads no secrets, and does not execute OpenAI,
Anthropic, MCP, A2A, or any other external provider. `simulated` remains the
only executable provider.

## Source Syntax

Policies are top-level declarations:

```argx
policy PolicyName {
    require no_unhandled_messages
    deny external_execution

    on violation {
        action review
        trace required
    }
}
```

Each policy has:

- one normal identifier name;
- zero or more `require` or `deny` rules;
- at most one optional `on violation` block.

The violation block accepts exactly:

```text
action block
action review
action warn
trace required
```

The action is mandatory when `on violation` is present. `trace required` is
optional and may appear at most once. No other items are accepted.

`runtime_status completed` is parsed as one policy rule with a fixed
`completed` argument, not as two unrelated rules.

## AST

`Program` gains:

```rust
pub policies: Vec<PolicyDecl>
```

The policy declarations use source-fidelity-oriented types:

```rust
pub struct PolicyDecl {
    pub name: Spanned<String>,
    pub rules: Vec<PolicyRuleDecl>,
    pub violation: Option<PolicyViolationDecl>,
}

pub enum PolicyRuleDecl {
    Require { rule: Spanned<PolicyRule> },
    Deny { rule: Spanned<PolicyRule> },
}

pub enum PolicyRule {
    NoUnhandledMessages,
    AllToolCallsTraced,
    AllModelCallsTraced,
    AllIntrinsicsTraced,
    AllProviderCallsTraced,
    HaltRequiresTrace,
    RuntimeStatusCompleted,
    ProviderContractsDeclared,
    ProviderAllowlistsValid,
    ExternalExecution,
    EvidenceBundleVerified,
    SecurityReportGenerated,
    Unknown(String),
}

pub struct PolicyViolationDecl {
    pub action: Spanned<PolicyViolationAction>,
    pub trace_required: bool,
}

pub enum PolicyViolationAction {
    Block,
    Review,
    Warn,
    Unknown(String),
}
```

Exact field placement may follow existing span conventions, but unknown rule
and action text must survive parsing so semantic diagnostics can identify the
original token.

The parser detects structural syntax errors, including malformed blocks and a
second `on violation` block. Semantic validity belongs to the semantic checker.
Unknown policy rules and unknown violation actions therefore parse into
`Unknown(String)` and fail during semantic checking.

## Initial Policy Rule Registry

v0.17 recognizes:

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

`require X` means the evidence predicate for `X` must be true.

`deny X` means the evidence predicate for `X` must be false.

Rules are compared by semantic identity. `require external_execution` and
`deny external_execution` are contradictory even though their effects differ.

## Semantic Validation

Whole-program semantic checking validates:

1. policy names are globally unique;
2. every rule is recognized;
3. a policy does not repeat the same effect and rule;
4. a policy does not contain both `require` and `deny` for the same rule;
5. a policy contains at most one violation block;
6. violation actions are `block`, `review`, or `warn`;
7. `runtime_status` is accepted only as `runtime_status completed`;
8. policies imported from modules participate in the merged program;
9. duplicate policy names across modules fail deterministically;
10. legacy assertions keep their v0.9 validation and behavior.

Duplicate and contradictory diagnostics identify the policy and rule. Unknown
rule and action diagnostics preserve the source spelling.

Parser-level duplicate violation detection is retained because the AST has a
single optional slot and silently discarding a second block would lose source
intent. The semantic checker still validates the resulting invariant for
programs constructed directly through the public AST.

## Package and Module Integration

Module resolution remains import-driven and unchanged. `merge_package`
appends each reachable module's policies in the same deterministic order used
for existing declarations. Whole-package semantic checking then detects
cross-module duplicate names.

Package IR and Bytecode contain the complete merged policy set plus existing
module and import metadata. An unimported policy module does not enter the
program.

## IR 0.17

`IrProgram` gains:

```rust
pub policies: Vec<IrPolicy>
```

The serialized shape is:

```json
{
  "policies": [
    {
      "name": "ProviderSafety",
      "rules": [
        {
          "effect": "deny",
          "rule": "external_execution"
        }
      ],
      "on_violation": {
        "action": "block",
        "trace_required": true
      }
    }
  ]
}
```

Policy rule and action values use stable snake-case strings. Legacy assertions
remain in the existing `assertions` field. Newly compiled IR uses
`ir_version: "0.17"`.

## Bytecode 0.17

`BytecodeProgram` gains a defaulted top-level collection:

```rust
#[serde(default)]
pub policies: Vec<BytecodePolicy>
```

`BytecodePolicy` preserves:

- policy name;
- ordered `require` and `deny` rules;
- optional violation action;
- `trace_required`.

Policies are metadata only in v0.17. Lowering does not emit duplicate
`DeclarePolicy`, `PolicyRule`, or `PolicyViolationMode` instructions.

The verifier accepts:

```text
0.3, 0.5, 0.6, 0.7, 0.8, 0.9, 0.10, 0.11, 0.12, 0.13, 0.14, 0.15, 0.16, 0.17
```

Verifier rules:

- non-empty policies require Bytecode 0.17;
- policy names must be non-empty and unique;
- effects must be `require` or `deny`;
- rules must be from the v0.17 registry;
- rules cannot be duplicated or contradicted within a policy;
- actions must be `block`, `review`, or `warn`;
- `trace_required` is meaningful only when `on_violation` exists;
- existing v0.16 module metadata remains valid under v0.17;
- defaulted empty `policies` preserves deserialization and verification of old
  Bytecode.

Newly lowered Bytecode uses `bytecode_version: "0.17"`.

## Common Runtime Rule Evaluator

The VM uses one predicate evaluator for legacy assertions and Policy v2 rules.
The evaluator receives:

- final runtime status and mailboxes;
- reactive steps;
- ordered ledger events;
- tool, model, intrinsic, and provider call summaries;
- provider contracts and allowlists;
- externally supplied artifact-verification evidence when available.

Legacy assertions retain their existing result type and names. Policy blocks
wrap predicate evaluation with an effect:

```text
require predicate -> pass when predicate is true
deny predicate    -> pass when predicate is false
```

Rule evidence:

- `no_unhandled_messages`: all final mailboxes are empty;
- `all_tool_calls_traced`: every tool call has corresponding trace evidence;
- `all_model_calls_traced`: every model call has corresponding trace evidence;
- `all_intrinsics_traced`: every intrinsic execution has corresponding ledger
  evidence;
- `all_provider_calls_traced`: every provider call has request/response or
  boundary-denial evidence;
- `halt_requires_trace`: every halted step contains traced evidence;
- `runtime_status completed`: final runtime status is completed before policy
  actions are applied;
- `provider_contracts_declared`: every non-simulated provider reference has a
  declared provider contract;
- `provider_allowlists_valid`: runtime provider contract allowlists were
  accepted by compilation and Bytecode verification;
- `external_execution`: true when external provider execution was attempted,
  including `ExternalProviderExecutionBlocked` evidence;
- `evidence_bundle_verified`: true only when trusted input context records a
  previously completed bundle verification;
- `security_report_generated`: true only when trusted input context records a
  previously completed SecurityReport generation.

The last two rules consume prior evidence; they do not let an artifact attest
to its own creation or verification. The default VM and CLI context supplies
no prior artifact evidence, so these predicates are false unless a caller uses
the public runtime API to provide verified evidence from an earlier pipeline
stage. This avoids report and bundle digest cycles and does not fabricate
evidence from command-line intent.

## Policy Report

`ReactiveExecutionTrace.policy_report` evolves without removing legacy data:

```json
{
  "status": "failed",
  "legacy_assertions": [],
  "policy_blocks": [
    {
      "name": "ProviderSafety",
      "passed": false,
      "status": "failed",
      "require_rules": [],
      "deny_rules": ["external_execution"],
      "violations": [
        {
          "rule": "external_execution",
          "effect": "deny",
          "reason": "external provider execution was attempted"
        }
      ],
      "action": "block",
      "trace_required": true
    }
  ],
  "actions": [
    {
      "policy": "ProviderSafety",
      "action": "block"
    }
  ]
}
```

For JSON compatibility, the existing `assertions` field may be retained as an
alias or legacy field during v0.17, but new output and public APIs expose
`legacy_assertions` explicitly. Policy block results separately preserve
required rules, denied rules, violations, action, trace requirement, and
status.

Overall policy status is:

- `passed` when all evaluated assertions and policy blocks pass;
- `failed` when a legacy assertion or block-action policy fails;
- `review_required` when no block failure exists and at least one review
  action is activated;
- `warning` when no block or review result exists and at least one warn action
  is activated;
- `violated` when a policy without `on violation` fails and no stronger state
  applies.

## Violation Actions and VM Outcome

When a policy rule is violated:

### `action block`

- policy result fails;
- `PolicyActionActivated` is recorded;
- runtime status becomes failed after preserving prior state and ledger;
- the VM returns a policy-violation error;
- CLI exits nonzero;
- SecurityReport and EvidenceBundle remain generable from `ExecutionOutcome`.

### `action review`

- policy result records `review_required`;
- runtime may remain completed;
- VM returns a successful trace;
- report verdict is medium with concrete `review required` evidence.

### `action warn`

- policy result records `warning`;
- runtime may remain completed;
- VM returns a successful trace;
- report verdict uses warning or informational severity without exaggeration.

### No violation block

- the policy result is violated;
- no runtime action is activated;
- runtime may remain completed;
- VM returns a successful trace;
- the report states the failed policy without inventing a block, review, or
  warning action.

If multiple policies fail, all rule results and violations are evaluated and
recorded before action precedence is applied:

```text
block > review > warn > no action
```

All activated policy actions remain visible even when a block determines the
final runtime error.

## Ledger Events

Add stable event kinds:

```text
PolicyDeclared
PolicyEvaluated
PolicyViolation
PolicyActionActivated
```

Events include concrete policy name, rule, effect, result, reason, action, and
trace requirement in deterministic detail strings. Existing assertion events
remain unchanged.

The ledger records every policy declaration before reactive execution and
every rule evaluation after execution evidence exists. Violations precede
their action events. `PolicyReportGenerated` remains the final policy-report
event. Block actions then record the existing VM failure evidence.

This ordering makes the reason for a final policy result reconstructable from
the ledger alone.

## SecurityReport 0.17

`PolicySummary` separates:

- legacy assertion totals, pass/fail counts, and failed names;
- policy block totals, pass/fail counts, and named results;
- require and deny rule counts;
- violations;
- activated actions;
- review-required and warning state.

The report version becomes `0.17`. Report verdict precedence is:

1. external execution or provider-boundary runtime failure: `high`;
2. block policy action or other failed runtime: `high`;
3. review policy action: `medium`;
4. failed policy without block, or denied tool/model calls: `medium`;
5. warn policy action: `warning`;
6. completed execution without evaluated policy: `informational`;
7. completed execution with passing policy: `pass`.

Reasons remain concrete evidence statements and do not imply that a successful
policy proves real-world safety.

## EvidenceBundle 0.17

New bundles use `bundle_version: "0.17"` and preserve existing metadata,
module information, artifact paths, and digest chain.

Policy v2 results are covered by:

- the trace digest, because `policy_report` is part of the trace;
- the report digest, because Policy v2 summaries are part of the report;
- the ledger digest, because policy declaration, evaluation, violation, and
  action events are part of the ledger.

Offline verification accepts bundle versions `0.14`, `0.15`, `0.16`, and
`0.17`. Existing v0.16 bundles remain parseable and verifiable.

Artifact-aware rules use only prior trusted evidence supplied before execution.
The current bundle never claims that the same bundle has already verified
itself, which avoids a digest and causality cycle.

## CLI

Existing commands and flags remain valid. `argorix-vm run --policy` renders:

- legacy assertions;
- each named policy block;
- require and deny results;
- violations;
- activated action;
- overall policy status.

No new policy-selection flag is required because v0.17 evaluates every policy
compiled into the program. Existing `--policy` controls output visibility, not
enforcement.

The CLI writes requested reports and bundles before returning a block-policy
error, matching existing failed-execution evidence behavior.

Compiler, VM, report, trace, bundle, and workspace versions advance to 0.17.

## Fixtures

Create:

```text
examples/policy_v017.argx
examples/policy_v017.argbc.json
examples/policy_project/
  argorix.toml
  src/main.argx
  src/agents/research.argx
  src/policies/default.argx
  src/providers/contracts.argx
examples/invalid_policies/
  unknown_policy_rule.argx
  duplicate_policy_rule.argx
  duplicate_policy_name.argx
  contradictory_policy.argx
  duplicate_violation_block.argx
  invalid_violation_action.argx
```

The valid single-file fixture covers require, deny, block/review/warn metadata,
legacy assertions, providers, and a deterministic reactive execution. The
package fixture proves imported policy aggregation.

Invalid fixtures fail at the intended parser or semantic stage with stable,
specific diagnostics.

## Conformance Suite 0.17

Create `conformance/suite.v017.json` and retain the data-driven runner design.
The official category set gains `policy_v2`.

Positive cases cover:

- policy parsing and semantic checking;
- imported policies;
- legacy assertion compatibility;
- passing require and deny rules;
- IR and Bytecode policy metadata;
- SecurityReport Policy v2 results;
- EvidenceBundle digest verification with Policy v2 results;
- review and warn completion behavior.

Negative and expected-failure cases cover:

- unknown rule;
- duplicate rule;
- duplicate policy name;
- contradictory effects;
- duplicate violation block;
- invalid violation action;
- denied external execution detected from blocked-attempt evidence;
- block action returning nonzero;
- expected VM policy failure represented in `ConformanceResult`.

The runner gains only generic assertions or stages needed to validate artifact
content and VM expected failures. It does not infer behavior from case IDs.
Expected failures remain explicit suite data.

The suite version and result version become `0.17`. The official v0.16 suite
may remain executable if its validation contract supports historical suite
versions; otherwise README documents v0.17 as the current official suite while
preserving v0.16 fixtures.

## Versioning and Compatibility

Advance:

```text
workspace:        0.17.0
IR:               0.17
Bytecode:         0.17
VM trace:         0.17
SecurityReport:   0.17
EvidenceBundle:   0.17
ConformanceSuite: 0.17
```

Preserve:

- single-file source compilation;
- package compilation and deterministic module graph;
- Bytecode 0.16 verification and execution;
- EvidenceBundle 0.16 offline verification;
- legacy assertions and failure declarations;
- all existing CLI flags;
- external provider fail-closed behavior;
- `simulated` as the only executable provider.

## Test Strategy

Implementation follows red-green-refactor in this order:

1. parser and AST;
2. semantic checker;
3. module/package merge;
4. IR 0.17;
5. Bytecode 0.17 and verifier;
6. common VM rule evaluator;
7. ledger events and action precedence;
8. SecurityReport;
9. EvidenceBundle;
10. CLI behavior;
11. fixtures;
12. Conformance Suite 0.17;
13. README and version inventory.

Required tests cover:

- policy, rule, and violation-block parsing;
- unknown rule/action preservation followed by semantic rejection;
- duplicate and contradictory rule diagnostics;
- global duplicate policy names;
- imported policies;
- legacy assertion compatibility;
- IR and Bytecode metadata;
- verifier acceptance of 0.16 and 0.17;
- require/deny predicate behavior;
- blocked external execution detection;
- block/review/warn/no-action outcomes;
- ledger preservation and event ordering;
- SecurityReport Policy v2 summary and verdict;
- EvidenceBundle v0.17 verification and v0.16 compatibility;
- compiler and VM CLI behavior;
- all invalid fixtures;
- text and JSON Conformance Suite output.

## Documentation

README adds `Argorix Lang v0.17 Policy Language v2` and explains:

- legacy assertions versus named policies;
- `require` and `deny`;
- violation blocks and each action;
- `trace required`;
- module-imported policies;
- runtime and report behavior;
- evidence limitations;
- no external execution, network, real OpenAI, Anthropic, MCP, or A2A;
- the v0.17 governing principle.

## Final Verification

Run fresh:

```bash
cargo fmt
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Acceptance also confirms:

- single-file v0.17 compilation;
- package v0.17 compilation;
- imported policy behavior;
- structural equality of fresh and committed v0.17 Bytecode;
- VM trace, SecurityReport, and EvidenceBundle v0.17 generation;
- offline evidence verification;
- Conformance Suite v0.17 in text and JSON;
- invalid policy fixture failures;
- Bytecode 0.16 compatibility;
- preserved v0.16 module/package behavior;
- preserved legacy assertions;
- external providers remain non-executable;
- `simulated` remains the only executable provider.
