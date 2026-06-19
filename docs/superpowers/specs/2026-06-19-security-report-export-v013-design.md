# Argorix Lang v0.13 Security Report Export Design

## Objective

Argorix Lang v0.13 converts real VM execution evidence into a stable,
serializable security-report artifact:

```text
Argorix Source
    -> IR 0.13
    -> Bytecode 0.13
    -> VM Execution
    -> Ledger
    -> Security Report
    -> JSON Export
```

The release remains offline and simulated-only. It performs no network access,
reads no secrets or environment variables, and integrates no real providers.

Project rules remain:

```text
Allowlisted does not mean executable.
Failed executions must still be reportable.
Report what happened, not what the example expected.
Verdict follows evidence, not intention.
Security reports are evidence artifacts, not success receipts.
```

## Execution Outcome

`argorix_vm` adds:

```rust
pub struct ExecutionOutcome {
    pub state: RuntimeState,
    pub result: Result<ReactiveExecutionTrace, VmError>,
}
```

`Vm::run_reactive_outcome()` becomes the primary reactive execution API for
auditing. It always returns `ExecutionOutcome`, preserving runtime state,
status, ledgers, failure activations, calls, and events on success or failure.

`Vm::run_reactive()` remains as a compatibility wrapper that delegates to
`run_reactive_outcome()` and returns only its `result`.

The CLI always executes through `run_reactive_outcome()`. When a report path
is requested, it generates and writes the report before returning a VM error
to the operating system. A failed execution therefore still exits nonzero,
but its evidence is not discarded.

## Security Report Types

`crates/argorix_vm/src/security_report.rs` defines public serializable types:

```rust
pub struct SecurityReport {
    pub report_version: String,
    pub language: String,
    pub module: String,
    pub bytecode_version: String,
    pub vm_version: String,
    pub execution: ExecutionSummary,
    pub policy: PolicySummary,
    pub provider_boundary: ProviderBoundarySummary,
    pub calls: CallSummary,
    pub intrinsics: IntrinsicSummary,
    pub ledger: LedgerSummary,
    pub verdict: SecurityVerdict,
}
```

All report and summary types are publicly re-exported by `argorix_vm`.

`SecurityReport::from_outcome(bytecode, outcome)` derives evidence from:

- `RuntimeState`, always;
- `ReactiveExecutionTrace`, when execution completed far enough to produce it;
- `VmError`, when execution failed;
- the actual ordered trace ledger.

Bytecode supplies only stable metadata: language, module, and bytecode version.
The report does not infer execution activity from raw bytecode.

## Execution Summary

```rust
pub struct ExecutionSummary {
    pub status: String,
    pub completed: bool,
    pub failed: bool,
    pub halted: bool,
    pub steps: usize,
    pub injected_message: Option<InjectedMessageSummary>,
}
```

Rules:

- `status`, `completed`, and `failed` come from final `RuntimeState`;
- `steps` and injected-message fields come from a successful trace;
- if no trace exists, `steps` uses `RuntimeState.completed_steps`;
- `halted` uses the trace when present, otherwise the ledger's `VmHalted`
  event;
- failure never masquerades as completion.

## Policy Summary

```rust
pub struct PolicySummary {
    pub evaluated: bool,
    pub passed: bool,
    pub assertions_total: usize,
    pub assertions_passed: usize,
    pub assertions_failed: usize,
    pub failed_assertions: Vec<String>,
    pub activated_failures: Vec<String>,
}
```

For successful traces, policy counts come from `policy_report`. For failed
executions without a trace, counts come only from ledger events:
`AssertionVerified`, `AssertionFailed`, and `FailureModeActivated`.

`evaluated` is true only when at least one assertion result exists. No policy
is invented. Activated failures include actual runtime failure-mode events,
including provider-boundary failures.

## Provider Boundary Summary

```rust
pub struct ProviderBoundarySummary {
    pub executable_providers: Vec<String>,
    pub declarative_contracts: Vec<ProviderContractSummary>,
    pub external_contracts_total: usize,
    pub external_execution_blocked: bool,
    pub blocked_attempts: usize,
}
```

The existing public `ProviderContractSummary` representation is reused and
preserves v0.12 allowlists exactly, including empty arrays.

Rules:

- `simulated` appears as executable;
- external contracts appear as declarative;
- `blocked_attempts` counts only `ExternalProviderExecutionBlocked` events;
- `external_execution_blocked` is `blocked_attempts > 0`;
- allowlists remain future permissions, never execution authority.

## Call Summary

```rust
pub struct CallSummary {
    pub tool_calls_total: usize,
    pub model_calls_total: usize,
    pub provider_calls_total: usize,
    pub denied_calls_total: usize,
    pub simulated_calls_total: usize,
}
```

Tool and model totals use the runtime call ledgers. Provider-call and simulated
totals use real provider-call summaries. Denials count actual
`ToolCallDenied`, `ModelCallDenied`, and `ProviderBoundaryDenied` events
without executing anything external.

## Intrinsic Summary

```rust
pub struct IntrinsicSummary {
    pub facu_checkpoints: usize,
    pub marron_guards: usize,
    pub intrinsic_events_total: usize,
}
```

Every real ledger event is counted. For the current three-agent fixture:

```json
{
  "facu_checkpoints": 3,
  "marron_guards": 3,
  "intrinsic_events_total": 6
}
```

No values are hardcoded and runtime behavior is not altered to match examples.

## Ledger Summary and Digest

```rust
pub struct LedgerSummary {
    pub events_total: usize,
    pub event_kinds: BTreeMap<String, usize>,
    pub first_event: Option<String>,
    pub last_event: Option<String>,
    pub ledger_digest: String,
}
```

Event-kind names use the stable serialized enum names. First and last values
use those same names.

The digest is:

```text
sha256:<lowercase hexadecimal SHA-256>
```

The digest input is compact JSON serialization of the ordered
`TraceLedger.events` vector. Struct field order and vector order are stable;
no maps participate in the digest input. The same ledger produces the same
digest. Any changed event changes the digest.

This digest is not a signature, contains no key, and does not prove real-world
safety. It is an integrity and reproducibility aid.

## Security Verdict

```rust
pub struct SecurityVerdict {
    pub passed: bool,
    pub severity: String,
    pub reasons: Vec<String>,
}
```

Verdict precedence:

1. `ExternalProviderExecutionBlocked` or provider-boundary failure: `high`;
2. any other failed runtime: `high`;
3. failed assertion: `medium`;
4. denied calls with completed runtime: `medium`;
5. completed runtime without evaluated assertions: `informational`;
6. completed runtime with passing policy: `pass`.

`passed` is false whenever runtime failed, an assertion failed, a call was
denied, or external execution was blocked. Reasons are concrete evidence only:

- `external provider execution blocked`;
- `provider boundary failure`;
- `runtime failed`;
- `policy assertion failed`;
- `tool/model call denied`;
- `completed without policy assertions`;
- `policy passed`.

No reason is added without corresponding state, trace, error, or event
evidence.

## CLI Export

The `run` command adds:

```text
--security-report <path>
```

There is no separate `report` subcommand in v0.13.

CLI flow:

```text
execute VM
-> preserve ExecutionOutcome
-> generate SecurityReport when requested
-> create parent directory and write pretty JSON
-> print successful trace when available
-> propagate VM error when execution failed
```

Writing creates only the required parent directory. Filesystem errors retain
path context and are not hidden.

Text mode appends:

```text
Security report written: <path>
```

JSON mode emits no extra lines or fields. Successful JSON stdout remains the
existing trace JSON exactly. A failed execution without a trace emits no
partial JSON to stdout; stderr receives the VM error and the process exits
nonzero after writing the requested report.

## Versions and Compatibility

Newly emitted versions:

- workspace: `0.13.0`;
- IR: `0.13`;
- Bytecode: `0.13`;
- reactive VM trace: `0.13`;
- Security Report: `0.13`.

The Bytecode verifier accepts:

```text
0.3, 0.5, 0.6, 0.7, 0.8, 0.9, 0.10, 0.11, 0.12, 0.13
```

Bytecode 0.12 remains verifiable and executable. Source v0.12 remains valid.
Provider allowlists, fail-closed behavior, simulated-only execution, and all
existing CLI flags remain unchanged.

## Fixtures and Generated Reports

Create:

- `examples/provider_allowlists_v013.argx`;
- `examples/provider_allowlists_v013.argbc.json`;
- `reports/.gitignore`.

The source is compatible with the v0.12 allowlist fixture but compiles to IR
and Bytecode 0.13.

`reports/.gitignore` contains:

```gitignore
*.security.json
```

Generated security reports are volatile audit artifacts and are not committed
by default.

## Test Strategy

Tests follow red-green-refactor and cover:

1. successful report construction;
2. report version 0.13;
3. module, Bytecode, and VM versions;
4. completed/failed/halted execution summaries;
5. policy assertion and failure counts;
6. simulated executable provider;
7. external declarative contracts;
8. preserved allowlists;
9. high severity for blocked external execution;
10. blocked-attempt event counts;
11. tool/model/provider call totals;
12. real 3/3/6 intrinsic counts;
13. event-kind counts;
14. deterministic digest;
15. changed digest after ledger mutation;
16. pass verdict;
17. informational verdict without policies;
18. medium/high policy-failure verdict;
19. CLI report-file creation;
20. parent-directory creation;
21. clean JSON stdout;
22. Bytecode 0.12 verification/execution;
23. Bytecode 0.13 default emission;
24. README documentation;
25. failed execution report generation;
26. failed ledger preservation;
27. real CLI nonzero exit after successful report write.

The v0.13 Bytecode fixture is structurally compared with fresh compiler output.
The six v0.12 invalid allowlist fixtures continue exiting nonzero.

## Documentation

README explains that a security report:

- is a deterministic audit artifact generated from VM execution;
- includes execution, policies, provider boundary, calls, intrinsics, ledger,
  digest, and verdict;
- is not a signature;
- does not prove real-world safety;
- does not enable external execution;
- preserves `Allowlisted does not mean executable`.

The example report documents real 3/3/6 intrinsic counts.

## Final Verification

Run:

```bash
cargo fmt
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Run the requested compiler and VM commands, including text and JSON report
exports. Confirm:

- report file exists;
- report JSON parses;
- `report_version == "0.13"`;
- `ledger_digest` begins with `sha256:`;
- provider contracts preserve allowlists;
- verdict exists;
- external providers remain non-executable;
- failed executions write reports and still exit nonzero;
- Bytecode 0.12 remains compatible.

The project principle remains:

```text
Rust is the forge.
Argorix Lang is the sword.
```
