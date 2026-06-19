# Argorix Lang v0.17 Policy Language v2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add named Policy Language v2 blocks to Argorix Lang while preserving legacy assertions, historical Bytecode and EvidenceBundle compatibility, deterministic package behavior, and simulated-only execution.

**Architecture:** Policies are source-preserving AST declarations lowered to top-level IR and Bytecode metadata. A shared VM predicate evaluator serves legacy assertions and Policy v2, while action handling, ledger events, SecurityReport summaries, and evidence digests preserve distinct policy-block results.

**Tech Stack:** Rust 2021 workspace, serde/serde_json, clap, sha2, custom parser/semantic checker/VM/conformance runner.

---

## File Map

- `crates/argorix_parser/src/ast.rs`: Policy v2 source types and `Program.policies`.
- `crates/argorix_parser/src/parser.rs`: policy grammar and structural diagnostics.
- `crates/argorix_semantics/src/checker.rs`: global policy registry, duplicates, contradictions, unknown values.
- `crates/argorix_module/src/merge.rs`: deterministic policy aggregation across imported modules.
- `crates/argorix_ir/src/ir.rs`: IR 0.17 policy metadata.
- `crates/argorix_bytecode/src/bytecode.rs`: Bytecode policy schema and verifier compatibility.
- `crates/argorix_bytecode/src/lower.rs`: IR-to-Bytecode policy lowering.
- `crates/argorix_vm/src/policy.rs`: shared evidence predicate evaluator and action precedence.
- `crates/argorix_vm/src/trace.rs`: Policy v2 report/result types and ledger events.
- `crates/argorix_vm/src/runtime.rs`: policy declaration events and runtime state.
- `crates/argorix_vm/src/vm.rs`: evaluation integration and block outcome behavior.
- `crates/argorix_vm/src/errors.rs`: block-policy VM error.
- `crates/argorix_vm/src/security_report.rs`: Policy v2 summaries and verdict precedence.
- `crates/argorix_vm/src/evidence.rs`: EvidenceBundle 0.17 and 0.16 compatibility.
- `crates/argorix-vm/src/main.rs`: policy output and failed-execution artifact behavior.
- `crates/argorixc/src/main.rs`: compiler version output and compiler acceptance tests.
- `crates/argorix_conformance/src/{types,validation,runner}.rs`: suite 0.17 and policy result checks.
- `examples/policy_v017.*`, `examples/policy_project/**`, `examples/invalid_policies/**`: acceptance fixtures.
- `conformance/suite.v017.json` and policy fixtures: official suite.
- `README.md`, workspace `Cargo.toml`, crate manifests and lockfile: documentation/versioning.

### Task 1: Parser and AST

**Files:**
- Modify: `crates/argorix_parser/src/ast.rs`
- Modify: `crates/argorix_parser/src/parser.rs`

- [ ] **Step 1: Write parser tests for the desired source model**

Add tests that parse:

```rust
let program = parse_source(r#"
    module main
    policy ProviderSafety {
        require runtime_status completed
        deny external_execution
        on violation {
            action block
            trace required
        }
    }
"#).unwrap();
assert_eq!(program.policies.len(), 1);
assert_eq!(program.policies[0].name.value, "ProviderSafety");
assert_eq!(program.policies[0].rules.len(), 2);
assert!(program.policies[0].violation.as_ref().unwrap().trace_required);
```

Also assert that unknown rules/actions produce `Unknown(String)`, and a second
`on violation` block fails with `duplicate on violation block`.

- [ ] **Step 2: Run parser tests and verify RED**

Run:

```bash
cargo test -p argorix_parser policy -- --nocapture
```

Expected: compilation/test failure because Policy v2 AST and parser support do
not exist.

- [ ] **Step 3: Add minimal AST and parser implementation**

Add `PolicyDecl`, `PolicyRuleDecl`, `PolicyRule`, `PolicyViolationDecl`, and
`PolicyViolationAction`. Implement stable helpers:

```rust
impl PolicyRule {
    pub fn source_name(&self) -> String { /* stable source spelling */ }
}

impl PolicyRuleDecl {
    pub fn effect(&self) -> &'static str { /* require or deny */ }
    pub fn rule(&self) -> &Spanned<PolicyRule> { /* contained rule */ }
}
```

Parse `runtime_status completed` as `RuntimeStatusCompleted`; preserve unknown
tokens and invalid actions as `Unknown`.

- [ ] **Step 4: Run parser tests and full parser crate**

```bash
cargo test -p argorix_parser
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/argorix_parser
git commit -m "feat(parser): parse policy language v2"
```

### Task 2: Semantic Checker

**Files:**
- Modify: `crates/argorix_semantics/src/checker.rs`

- [ ] **Step 1: Write failing semantic tests**

Cover:

```text
unknown policy rule
unknown policy action
duplicate require rule
duplicate deny rule
contradictory require/deny
duplicate policy name
legacy assertion remains valid
```

Diagnostics must contain the policy/rule/action spelling.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p argorix_semantics policy -- --nocapture
```

Expected: FAIL because policies are not checked.

- [ ] **Step 3: Implement semantic policy validation**

Use two sets per policy:

```rust
let mut required = HashSet::new();
let mut denied = HashSet::new();
```

Reject `Unknown`, duplicate entries in the same set, intersections between
sets, invalid actions, and globally duplicate names. Leave legacy assertion
checking unchanged.

- [ ] **Step 4: Verify GREEN**

```bash
cargo test -p argorix_semantics
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/argorix_semantics/src/checker.rs
git commit -m "feat(semantics): validate policy language v2"
```

### Task 3: Module and Package Integration

**Files:**
- Modify: `crates/argorix_module/src/merge.rs`
- Modify: `crates/argorix_module/tests/resolver.rs`

- [ ] **Step 1: Add failing package tests**

Build temporary packages proving:

- an imported policy enters `merge_package`;
- two imported modules declaring the same policy name fail `check_package`;
- unimported policy files are absent.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p argorix_module policy -- --nocapture
```

- [ ] **Step 3: Merge policies deterministically**

Initialize `Program.policies` and extend it beside assertions/providers in
entry-first, sorted-module order.

- [ ] **Step 4: Verify GREEN**

```bash
cargo test -p argorix_module
```

- [ ] **Step 5: Commit**

```bash
git add crates/argorix_module
git commit -m "feat(module): merge imported policies"
```

### Task 4: IR 0.17

**Files:**
- Modify: `crates/argorix_ir/src/ir.rs`
- Modify: `crates/argorix_ir/src/lib.rs`

- [ ] **Step 1: Write failing IR tests**

Assert `ir_version == "0.17"` and JSON:

```json
{
  "name": "ProviderSafety",
  "rules": [{"effect":"deny","rule":"external_execution"}],
  "on_violation":{"action":"block","trace_required":true}
}
```

Also assert legacy assertions remain in `assertions`.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p argorix_ir policy -- --nocapture
```

- [ ] **Step 3: Add IR policy types and lowering**

Create `IrPolicy`, `IrPolicyRule`, and `IrPolicyViolation`; use stable
snake-case values and `Option<IrPolicyViolation>`.

- [ ] **Step 4: Verify GREEN**

```bash
cargo test -p argorix_ir
```

- [ ] **Step 5: Commit**

```bash
git add crates/argorix_ir
git commit -m "feat(ir): preserve policy v2 metadata"
```

### Task 5: Bytecode 0.17 and Verifier

**Files:**
- Modify: `crates/argorix_bytecode/src/bytecode.rs`
- Modify: `crates/argorix_bytecode/src/lower.rs`
- Modify: `crates/argorix_bytecode/src/lib.rs`

- [ ] **Step 1: Write failing Bytecode tests**

Assert:

- lowering emits `bytecode_version: "0.17"` and top-level policies;
- no policy declaration instructions are emitted;
- v0.16 with empty/defaulted policies verifies;
- v0.17 validates duplicate names/rules, contradictions, unknown effects,
  rules and actions;
- policies on v0.16 produce `PoliciesRequireV017`.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p argorix_bytecode policy -- --nocapture
```

- [ ] **Step 3: Implement schema, lowering and verification**

Add defaulted `policies`, `BytecodePolicy`, `BytecodePolicyRule`, and
`BytecodePolicyViolation`. Extend accepted versions and module/provider
feature ranges through 0.17.

- [ ] **Step 4: Verify GREEN**

```bash
cargo test -p argorix_bytecode
```

- [ ] **Step 5: Commit**

```bash
git add crates/argorix_bytecode
git commit -m "feat(bytecode): add policy v2 metadata and verification"
```

### Task 6: Common VM Rule Evaluator and Policy Reports

**Files:**
- Create: `crates/argorix_vm/src/policy.rs`
- Modify: `crates/argorix_vm/src/lib.rs`
- Modify: `crates/argorix_vm/src/trace.rs`
- Modify: `crates/argorix_vm/src/vm.rs`

- [ ] **Step 1: Write failing predicate and report tests**

Test passing `require`, passing `deny`, violation reasons, separate
`legacy_assertions` and `policy_blocks`, and trusted prior artifact evidence.
Use real `RuntimeState`, steps, and ledger events rather than mocks.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p argorix_vm policy -- --nocapture
```

- [ ] **Step 3: Implement shared predicate evaluation**

Create:

```rust
pub struct PolicyEvidenceContext {
    pub security_report_generated: bool,
    pub evidence_bundle_verified: bool,
}

pub fn evaluate_rule(
    rule: &str,
    state: &RuntimeState,
    steps: &[ReactiveStep],
    context: &PolicyEvidenceContext,
) -> RuleEvaluation
```

Legacy assertions call the same predicate function. Policy rules invert the
predicate only for `deny`.

- [ ] **Step 4: Build structured report types**

Add `PolicyBlockResult`, `PolicyRuleResult`, `PolicyViolation`,
`PolicyActionResult`, and a `PolicyReport` containing legacy and v2 sections.
Keep `assertions` as a serde-compatible alias or retained field if required by
old consumers.

- [ ] **Step 5: Verify GREEN**

```bash
cargo test -p argorix_vm policy
cargo test -p argorix_vm
```

- [ ] **Step 6: Commit**

```bash
git add crates/argorix_vm
git commit -m "feat(vm): evaluate policy v2 rules"
```

### Task 7: Ledger Events and Action Precedence

**Files:**
- Modify: `crates/argorix_vm/src/trace.rs`
- Modify: `crates/argorix_vm/src/runtime.rs`
- Modify: `crates/argorix_vm/src/vm.rs`
- Modify: `crates/argorix_vm/src/errors.rs`

- [ ] **Step 1: Write failing runtime behavior tests**

Prove:

- declaration/evaluation/violation/action event ordering;
- `block` returns `VmError::PolicyViolation`, leaves ledger intact, and sets
  state failed;
- `review` returns a completed trace with `review_required`;
- `warn` returns a completed trace with `warning`;
- no-action violation returns a completed trace with `violated`;
- precedence is block > review > warn > no action while all actions remain
  recorded.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p argorix_vm policy_action -- --nocapture
```

- [ ] **Step 3: Implement events and precedence**

Add:

```rust
PolicyDeclared
PolicyEvaluated
PolicyViolation
PolicyActionActivated
```

Evaluate every policy before applying the final block result. For block,
construct the trace/report evidence first, preserve it in `ExecutionOutcome`,
then return the policy error without discarding runtime state.

- [ ] **Step 4: Verify GREEN and regressions**

```bash
cargo test -p argorix_vm
```

- [ ] **Step 5: Commit**

```bash
git add crates/argorix_vm
git commit -m "feat(vm): enforce policy violation actions"
```

### Task 8: SecurityReport and EvidenceBundle 0.17

**Files:**
- Modify: `crates/argorix_vm/src/security_report.rs`
- Modify: `crates/argorix_vm/src/evidence.rs`
- Modify: `crates/argorix_vm/tests/security_report.rs`
- Modify: `crates/argorix_vm/tests/evidence.rs`

- [ ] **Step 1: Write failing report/evidence tests**

Assert Policy v2 totals, rules, violations, actions, review/warning flags,
verdict precedence, `report_version == "0.17"`, `bundle_version == "0.17"`,
and successful verification of both new v0.17 and existing v0.16 bundles.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p argorix_vm security_report evidence -- --nocapture
```

- [ ] **Step 3: Implement Policy v2 report summaries**

Extend `PolicySummary` without removing legacy counts. Derive failed-execution
policy evidence from ledger events when no successful trace is available.

- [ ] **Step 4: Extend evidence compatibility**

Emit 0.17; accept `0.14 | 0.15 | 0.16 | 0.17`. Keep trace/report/ledger digest
checks unchanged so Policy v2 content is covered automatically.

- [ ] **Step 5: Verify GREEN**

```bash
cargo test -p argorix_vm
```

- [ ] **Step 6: Commit**

```bash
git add crates/argorix_vm
git commit -m "feat(evidence): report and verify policy v2"
```

### Task 9: CLI Behavior and Versioning

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/*/Cargo.toml` where explicit metadata requires updates
- Modify: `crates/argorixc/src/main.rs`
- Modify: `crates/argorix-vm/src/main.rs`
- Modify: `crates/argorix-vm/tests/security_report_cli.rs`
- Modify: `crates/argorix-vm/tests/evidence_cli.rs`

- [ ] **Step 1: Write failing CLI tests**

Test compiler/VM v0.17 output, `--policy` legacy and block sections,
block-action nonzero exit, and report/bundle creation before returning the
error.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p argorixc
cargo test -p argorix-vm
```

- [ ] **Step 3: Implement rendering and version updates**

Render each policy name, rule effect/status, violation reason, action and
overall status. Do not add selection flags; all compiled policies enforce.
Advance workspace to 0.17.0.

- [ ] **Step 4: Verify GREEN**

```bash
cargo test -p argorixc
cargo test -p argorix-vm
```

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates
git commit -m "feat(cli): expose policy language v2 results"
```

### Task 10: Fixtures and Package Acceptance

**Files:**
- Create: `examples/policy_v017.argx`
- Create: `examples/policy_v017.argbc.json`
- Create: `examples/policy_project/**`
- Create: `examples/invalid_policies/**`
- Modify: `tests/compiler_tests.rs`
- Modify: `crates/argorixc/tests/package_cli.rs`

- [ ] **Step 1: Add fixture-driven failing tests**

Assert valid single-file and package compilation, imported policy presence,
stable invalid diagnostics, structural Bytecode fixture equality, and
preserved v0.16 package commands.

- [ ] **Step 2: Verify RED**

```bash
cargo test --test compiler_tests policy -- --nocapture
cargo test -p argorixc --test package_cli policy -- --nocapture
```

- [ ] **Step 3: Create fixtures and regenerate committed Bytecode**

Generate with:

```bash
cargo run -q -p argorixc -- emit-bytecode examples/policy_v017.argx
```

Store pretty JSON exactly matching fresh compiler output.

- [ ] **Step 4: Verify GREEN and invalid exits**

Run all six invalid fixtures through `argorixc check`; each must exit nonzero
with the expected parser/semantic diagnostic.

- [ ] **Step 5: Commit**

```bash
git add examples tests crates/argorixc/tests
git commit -m "test: add policy v2 fixtures and package coverage"
```

### Task 11: Conformance Suite v0.17

**Files:**
- Modify: `crates/argorix_conformance/src/types.rs`
- Modify: `crates/argorix_conformance/src/validation.rs`
- Modify: `crates/argorix_conformance/src/runner.rs`
- Modify: `crates/argorix_conformance/tests/**`
- Create: `conformance/suite.v017.json`
- Create: `conformance/sources/policy_*.argx`
- Create: `conformance/modules/policy_project/**`
- Create: `conformance/bytecode/policy_v017.argbc.json`

- [ ] **Step 1: Write failing suite validation/runner tests**

Require suite version 0.17 and category `policy_v2`. Add generic JSON-pointer
or policy-result expectations only if needed; do not branch on case IDs.
Prove expected VM block failure becomes a passing conformance case with a
failed `run_vm` stage.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p argorix-conformance policy -- --nocapture
```

- [ ] **Step 3: Extend generic conformance behavior**

Keep explicit expected failures. If `run_vm` receives a block-policy outcome,
return its concrete error while retaining state for later report/evidence
stages only when suite ordering explicitly requests those stages before the
expected terminal failure.

- [ ] **Step 4: Build official suite**

Copy v0.16 cases forward and add all required `policy_v2` positive/negative
cases. Update historical unit fixtures to 0.17 while preserving dedicated
compatibility cases.

- [ ] **Step 5: Verify text and JSON**

```bash
cargo run -q -p argorix-conformance -- run conformance/suite.v017.json
cargo run -q -p argorix-conformance -- run conformance/suite.v017.json --json
```

Expected: exit 0, all cases passed, JSON stdout parses as one
`ConformanceResult`.

- [ ] **Step 6: Commit**

```bash
git add crates/argorix_conformance conformance
git commit -m "feat(conformance): add policy v2 suite v0.17"
```

### Task 12: README and Final Acceptance

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add documentation assertions/checklist**

Confirm README contains the v0.17 principle, syntax, legacy distinction,
require/deny semantics, actions, module imports, evidence behavior, limitations
and all current commands.

- [ ] **Step 2: Update README**

Document Policy Language v2 and update version inventories without deleting
historical v0.9-v0.16 explanations.

- [ ] **Step 3: Run formatting**

```bash
cargo fmt
cargo fmt --all -- --check
```

- [ ] **Step 4: Run mandatory workspace verification**

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 5: Run acceptance commands**

```bash
cargo run -q -p argorixc -- check examples/policy_v017.argx
cargo run -q -p argorixc -- emit-ir examples/policy_v017.argx
cargo run -q -p argorixc -- emit-bytecode examples/policy_v017.argx
cargo run -q -p argorixc -- check-package examples/policy_project/argorix.toml
cargo run -q -p argorixc -- emit-bytecode-package examples/policy_project/argorix.toml
cargo run -q -p argorixc -- verify-bytecode examples/provider_allowlists_v016.argbc.json
cargo run -q -p argorix-conformance -- run conformance/suite.v017.json
cargo run -q -p argorix-conformance -- run conformance/suite.v017.json --json
```

Run the VM fixture with report, trace, bundle and `verify-evidence`; compare
fresh Bytecode JSON structurally with the committed fixture. Confirm invalid
fixtures exit nonzero and external provider attempts remain blocked.

- [ ] **Step 6: Review the complete diff**

```bash
git diff --check
git status --short
git diff origin/main...HEAD --stat
```

- [ ] **Step 7: Commit documentation/final adjustments**

```bash
git add README.md
git commit -m "docs: document policy language v2"
```
