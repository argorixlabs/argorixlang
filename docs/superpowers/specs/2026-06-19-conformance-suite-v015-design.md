# Argorix Lang v0.15 Conformance Suite Design

## Objective

Argorix Lang v0.15 adds an official, local, reproducible Conformance Suite for
the full language stack:

```text
Argorix Source
    -> Parser
    -> Semantic Checker
    -> IR 0.15
    -> Bytecode 0.15
    -> Bytecode Verifier
    -> VM 0.15
    -> SecurityReport 0.15
    -> EvidenceBundle 0.15
    -> Offline Verification
    -> ConformanceResult 0.15
```

The suite performs no network access, reads no secrets or environment
variables, and does not execute external providers. `simulated` remains the
only executable provider.

The governing principles are:

```text
Allowlisted does not mean executable.
Failed executions must still be reportable.
Security reports are evidence artifacts, not success receipts.
Evidence must be exportable and independently checkable.
A secure language must be independently testable.
Conformance must make expected failure explicit.
Conformance must not depend on fixture-specific inference.
Conformance cases must be data-driven, not runner-driven.
Conformance paths resolve from the suite, not from the shell.
```

## Crate and Binary

Create `crates/argorix_conformance` as both:

- library crate `argorix_conformance`;
- binary `argorix-conformance`.

The library owns suite types, validation, stage execution, artifact management,
mutations, and result construction. The binary is a thin Clap layer that reads
the suite, selects a work directory, invokes the library, renders text or JSON,
and sets the process exit code.

The runner calls parser, semantics, IR, Bytecode, VM, SecurityReport, and
EvidenceBundle APIs directly. It never launches `argorixc`, `argorix-vm`, or
another subprocess.

## Public Suite Types

```rust
pub struct ConformanceSuite {
    pub suite_version: String,
    pub cases: Vec<ConformanceCase>,
}

pub struct ConformanceCase {
    pub id: String,
    pub name: String,
    pub category: String,
    pub source_path: Option<String>,
    pub bytecode_path: Option<String>,
    pub stages: Vec<String>,
    pub injection: Option<String>,
    pub mutation: Option<ConformanceMutation>,
    pub expected_failure_stage: Option<String>,
    pub expected_failure_contains: Option<String>,
}

pub struct ConformanceMutation {
    pub before_stage: String,
    pub artifact: String,
    pub json_pointer: String,
    pub value: serde_json::Value,
}
```

All types are public, serializable, deserializable, cloneable, and equality
comparable where their fields permit it.

## Stages

Supported stage names and dependencies:

| Stage | Input | Output |
| --- | --- | --- |
| `parse` | source fixture | parsed AST |
| `semantic_check` | parsed AST | validated AST |
| `emit_ir` | validated AST | `ir.json` |
| `emit_bytecode` | IR | working `program.argbc.json` |
| `verify_bytecode` | working or fixture Bytecode | verified Bytecode |
| `run_vm` | verified Bytecode + injection | `ExecutionOutcome` |
| `security_report` | Bytecode + outcome | `run.security.json` |
| `trace_out` | successful outcome trace | `run.trace.json` |
| `evidence_bundle` | Bytecode + outcome + report | `run.bundle.json` |
| `verify_evidence` | bundle and referenced artifacts | verification result |

Cases execute their `stages` in listed order. Stages must be unique and respect
the dependency order. A case may start from a committed Bytecode fixture by
using `bytecode_path` with `verify_bytecode` and later stages, without source
stages.

`security_report`, `trace_out`, `evidence_bundle`, and `verify_evidence` depend
on `run_vm`. `evidence_bundle` also requires `security_report`; when a trace is
available it requires `trace_out` so offline verification has the referenced
artifact. `verify_evidence` requires `evidence_bundle`.

Every requested stage produces:

```rust
pub struct ConformanceStageResult {
    pub stage: String,
    pub status: String,
    pub message: Option<String>,
}
```

Status is exactly `passed`, `failed`, or `skipped`.

## Explicit Expected Failures

`stages` means what to execute. `expected_failure_stage` means where execution
must fail.

Rules:

- if the expected stage fails and its diagnostic contains
  `expected_failure_contains` when specified, the case passes;
- the expected failing stage remains `failed`;
- every later requested stage is `skipped`;
- if another stage fails first, the case fails;
- if the expected stage passes, the case fails;
- the expected stage must appear in `stages`;
- `expected_failure_contains` without `expected_failure_stage` is invalid;
- without an expected failure, every listed stage must pass.

Suite validation errors occur before any case executes.

## Injection

`injection` uses:

```text
from:to:act:message_type
```

The parser is shared by the conformance library and the VM CLI so the format is
implemented once. It rejects missing, extra, or blank fields.

Injection is mandatory when any of these stages are requested:

```text
run_vm
security_report
trace_out
evidence_bundle
verify_evidence
```

The runner never infers or hardcodes an injection from a fixture or case ID.

## Declarative Mutations

Before executing `mutation.before_stage`, the runner loads the selected working
artifact as JSON, replaces the value addressed by the standard JSON Pointer,
and writes the modified JSON back to the same workdir artifact.

Allowed artifacts:

```text
bytecode
security_report
bundle
```

Rules:

- `before_stage` must appear in `stages`;
- artifact name must be allowed;
- artifact must exist when the mutation is applied;
- JSON Pointer must be syntactically valid and resolve to an existing value;
- mutation affects only the case copy in the workdir;
- committed source and Bytecode fixtures are never modified;
- no case IDs or mutation scenarios are hardcoded in the runner.

This supports data-driven checks for Bytecode digest mismatch, report digest
mismatch, ledger mismatch, module/version mismatch, and modified bundle
metadata.

## Portable Fixture Paths

`source_path` and `bytecode_path` in `suite.v015.json` are resolved from the
suite file's parent directory, never from the shell current directory.

Official paths use normalized `/` separators:

```json
{
  "source_path": "sources/provider_allowlists_v015.argx",
  "bytecode_path": "bytecode/provider_allowlists_v015.argbc.json"
}
```

Absolute paths and relative paths escaping the portable suite tree are
rejected for official execution. Internal tests may construct explicit
temporary absolute suite paths, but result objects never expose absolute
paths.

Moving the complete `conformance/` folder preserves fixture resolution.

## Work Directory and Artifacts

CLI:

```text
argorix-conformance run <suite.json> [--workdir <path>] [--json]
```

Default workdir:

```text
target/argorix-conformance
```

Each case uses:

```text
<workdir>/<case-id>/
```

Artifacts use fixed relative names:

```text
ir.json
program.argbc.json
run.trace.json
run.security.json
run.bundle.json
```

The runner recreates each case directory before execution so stale artifacts
cannot affect results. It validates case IDs as portable single path segments
and rejects empty IDs, separators, `.` and `..`.

The workdir affects generated artifacts only; it never changes source or
Bytecode fixture resolution.

## Result Types

```rust
pub struct ConformanceResult {
    pub suite_version: String,
    pub passed: bool,
    pub cases_total: usize,
    pub cases_passed: usize,
    pub cases_failed: usize,
    pub case_results: Vec<ConformanceCaseResult>,
    pub failures: Vec<ConformanceFailure>,
}

pub struct ConformanceCaseResult {
    pub id: String,
    pub name: String,
    pub category: String,
    pub passed: bool,
    pub stages: Vec<ConformanceStageResult>,
}

pub struct ConformanceFailure {
    pub case_id: String,
    pub stage: String,
    pub reason: String,
}
```

Result ordering follows suite case order and stage order. No timestamps,
absolute paths, random identifiers, environment data, or nondeterministic maps
are included.

## Suite Validation

Before execution, validate:

- suite version is `0.15`;
- case IDs are non-empty, portable, and unique;
- names and categories are non-empty;
- all required official categories are present;
- stage lists are non-empty, known, unique, and dependency ordered;
- source stages have `source_path`;
- Bytecode-only cases have `bytecode_path`;
- VM-dependent stages have valid injection;
- expected-failure fields are consistent;
- mutations have valid stage, artifact, and JSON Pointer syntax;
- fixture paths are relative, portable, and exist.

Invalid suite input returns a clear library error and the CLI exits nonzero
without emitting a partial `ConformanceResult`.

## Stage Execution and Diagnostics

Each stage returns either its typed output or a deterministic diagnostic
string. Parser and semantic diagnostics are rendered without absolute file
paths. Bytecode verification joins verifier errors in stable order. VM errors,
Evidence errors, and mutation errors preserve their concrete reason.

When a stage fails unexpectedly:

- its stage result is `failed`;
- later stages are `skipped`;
- one `ConformanceFailure` records case ID, stage, and reason;
- the case is failed.

When an expected failure matches, no suite-level `ConformanceFailure` is added
because the case fulfilled its contract.

## CLI Output

Successful text output:

```text
Argorix Conformance Suite v0.15
Cases: 25
Passed: 25
Failed: 0
Conformance: passed
```

Failed text output includes the summary followed by case/stage diagnostics.
The process exits zero only when `ConformanceResult.passed` is true.

`--json` emits only pretty JSON `ConformanceResult` to stdout. Diagnostics
needed before a result exists go to stderr. No progress logs contaminate JSON
stdout.

## Official Suite

Create:

```text
conformance/
  suite.v015.json
  .gitignore
  sources/
  bytecode/
  reports/
  traces/
  bundles/
```

Commit suite JSON, source fixtures, and Bytecode fixtures. Ignore generated
reports, traces, and bundles.

The suite is completely data-driven and covers:

```text
parser
semantics
ir
bytecode
vm
policy
provider_boundary
adapter_contracts
allowlists
security_report
evidence_bundle
offline_verification
compatibility
```

It contains the required positive compatibility cases and negative parser,
semantic, provider, allowlist, Bytecode, runtime-report, and mutated-evidence
cases. Existing repository fixtures are copied into `conformance/sources` and
`conformance/bytecode` only where needed to keep the suite portable and
self-contained.

## Versioning and Compatibility

Advance:

- workspace: `0.15.0`;
- IR: `0.15`;
- Bytecode: `0.15`;
- VM trace: `0.15`;
- SecurityReport: `0.15`;
- EvidenceBundle: `0.15`;
- ConformanceSuite: `0.15`.

The Bytecode verifier accepts:

```text
0.3, 0.5, 0.6, 0.7, 0.8, 0.9, 0.10, 0.11, 0.12, 0.13, 0.14, 0.15
```

Bytecode 0.14 remains verifiable and executable. Existing v0.14 CLI flags and
source behavior remain compatible.

Offline evidence verification accepts both EvidenceBundle 0.14 and 0.15 when
their report, Bytecode, VM, digest, and artifact metadata are internally
consistent. SecurityReport 0.14 remains parseable for verifying a v0.14 bundle.

## Tests

Tests follow red-green-refactor and cover:

1. suite/type deserialization and stable serialization;
2. suite validation rules;
3. relative path resolution independent of cwd;
4. positive parser/check/IR/Bytecode pipeline;
5. positive VM/report/trace/bundle/verification pipeline;
6. matching and non-matching expected failures;
7. skipped stages after expected and unexpected failure;
8. missing/invalid injection;
9. declarative Bytecode, report, and bundle mutations;
10. missing artifact and missing JSON Pointer diagnostics;
11. workdir default and override;
12. no absolute paths in results;
13. Bytecode 0.14 compatibility;
14. EvidenceBundle and SecurityReport 0.14 compatibility;
15. Bytecode 0.15 default emission;
16. clean text and JSON CLI output;
17. zero/nonzero CLI exit behavior;
18. failed VM execution still generating report and evidence where specified;
19. provider boundary and allowlist negative cases;
20. README conformance documentation.

## Documentation and Final Verification

README documents what the suite validates, what it does not prove, offline and
simulated-only constraints, commands, result interpretation, case schema,
expected failures, injections, mutations, path resolution, workdir behavior,
and how to add cases.

Final verification:

```bash
cargo fmt
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Acceptance also verifies:

- v0.15 source and Bytecode fixture generation;
- structural equality between compiler output and committed v0.15 fixture;
- VM/report/trace/bundle v0.15 export;
- intact Evidence verification;
- official Conformance Suite in text and JSON;
- clean JSON stdout;
- expected negative cases;
- Bytecode and EvidenceBundle v0.14 compatibility;
- external providers remain non-executable;
- `simulated` remains the only executable provider.
