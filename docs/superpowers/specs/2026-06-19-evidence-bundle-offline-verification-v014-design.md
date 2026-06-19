# Argorix Lang v0.14 Evidence Bundle and Offline Verification Design

## Objective

Argorix Lang v0.14 packages locally generated execution evidence and verifies
its semantic consistency offline:

```text
Argorix Source
    -> IR 0.14
    -> Bytecode 0.14
    -> VM Execution
    -> ReactiveExecutionTrace
    -> SecurityReport 0.14
    -> EvidenceBundle 0.14
    -> Offline Verification
```

The release remains offline and simulated-only. It performs no network access,
reads no secrets or environment variables, and does not execute external
providers.

The governing principles are:

```text
Allowlisted does not mean executable.
Failed executions must still be reportable.
Security reports are evidence artifacts, not success receipts.
Evidence must be exportable and independently checkable.
Evidence must travel with its verification context.
```

## Public Evidence Module

`argorix_vm::evidence` owns canonical serialization, semantic SHA-256 digests,
bundle construction, relative artifact paths, offline verification, and
verification result reporting. The CLI delegates to this module and contains
only argument parsing, file export, and human/JSON presentation.

Public types:

```rust
pub struct EvidenceBundle {
    pub bundle_version: String,
    pub language: String,
    pub module: String,
    pub bytecode_version: String,
    pub vm_version: String,
    pub report_version: String,
    pub bytecode_digest: String,
    pub trace_digest: Option<String>,
    pub report_digest: String,
    pub ledger_digest: String,
    pub artifacts: EvidenceArtifacts,
}

pub struct EvidenceArtifacts {
    pub bytecode_path: Option<String>,
    pub trace_path: Option<String>,
    pub security_report_path: Option<String>,
}

pub struct EvidenceVerificationResult {
    pub passed: bool,
    pub checks_total: usize,
    pub checks_passed: usize,
    pub checks_failed: usize,
    pub failures: Vec<String>,
}
```

No timestamp participates in v0.14.

## Canonical Semantic Digests

`canonical_digest<T: Serialize>()` serializes the deserialized Rust value with
compact `serde_json::to_vec`, hashes those canonical semantic bytes with
SHA-256, and returns `sha256:<lowercase hex>`.

This rule applies to Bytecode, `ReactiveExecutionTrace`, `SecurityReport`, and
the ordered ledger event vector. Whitespace and pretty-printing changes do not
change a digest. Semantic field or value changes do.

The digest is not a signature, uses no key, promises no authenticity, and does
not prove real-world security. It provides deterministic local integrity and
reproducibility checks.

`SecurityReport.ledger.ledger_digest` uses the same shared canonical digest
function over ordered ledger events.

## Bundle Construction

`EvidenceBundle::from_outcome` receives Bytecode, `ExecutionOutcome`,
`SecurityReport`, the bundle path, and optional artifact paths. It:

- preserves language, module, and all version metadata;
- computes semantic Bytecode, trace, and report digests;
- copies the report ledger digest;
- stores `trace_digest: None` and `trace_path: None` when no trace exists;
- converts artifact paths to portable paths relative to the bundle directory;
- normalizes stored separators to `/`;
- rejects absolute artifact paths that cannot be represented relative to the
  bundle directory without leaving the portable project tree.

Relative paths may contain `..` when referencing a sibling project directory,
such as `../examples/...`. The bundle path itself defines the verification
base.

## Offline Verification

`verify_evidence(bundle_path)` deserializes the bundle, resolves every declared
artifact path from the bundle's parent directory, and returns
`EvidenceVerificationResult`.

Checks include:

- digest syntax for every present digest;
- existence and deserialization of each declared artifact;
- semantic Bytecode digest;
- semantic trace digest when declared;
- semantic SecurityReport digest;
- report ledger digest equals bundle ledger digest;
- report version equals bundle report version;
- report Bytecode version equals bundle Bytecode version;
- report VM version equals bundle VM version;
- Bytecode language/module/version match bundle metadata;
- trace VM version matches bundle VM version when a trace exists;
- `trace_path` and `trace_digest` are either both present or both absent.

Missing, absolute, malformed, or unreadable artifact paths produce named
failures. Verification continues where possible so one invocation reports all
independent mismatches. `passed` is true only when every recorded check passes.

## CLI

`run` adds:

```text
--security-report <path>
--trace-out <path>
--evidence-bundle <path>
```

Reactive execution flow:

```text
execute VM
-> preserve ExecutionOutcome
-> build SecurityReport
-> write requested report
-> write requested trace when available
-> build and write requested EvidenceBundle
-> print trace or propagate VM error
```

Parent directories are created. Text mode prints artifact-written messages.
JSON mode writes only the trace JSON on successful execution and no partial
JSON on failed execution. Artifact messages never contaminate JSON stdout.
Failed VM execution still exits nonzero after writing all possible evidence.

`verify-evidence <bundle.json> [--json]` resolves artifacts relative to the
bundle. Text success prints `Evidence verification: passed`. Text failure
prints `Evidence verification: failed` followed by failures. JSON prints only
the serialized verification result. Failed verification exits nonzero.

## Versions and Compatibility

Newly emitted versions:

- workspace: `0.14.0`;
- IR: `0.14`;
- Bytecode: `0.14`;
- reactive VM trace: `0.14`;
- SecurityReport: `0.14`;
- EvidenceBundle: `0.14`.

The verifier accepts Bytecode `0.3`, `0.5`, `0.6`, `0.7`, `0.8`, `0.9`,
`0.10`, `0.11`, `0.12`, `0.13`, and `0.14`. Bytecode 0.13 remains executable.
External providers remain declarative and non-executable; `simulated` remains
the only executable provider.

## Fixtures and Generated Artifacts

Create:

- `examples/provider_allowlists_v014.argx`;
- `examples/provider_allowlists_v014.argbc.json`.

Generated artifacts remain ignored:

```gitignore
reports/*.security.json
reports/*.bundle.json
reports/*.trace.json
```

The committed Bytecode fixture must structurally equal fresh compiler output.

## Test Strategy

Tests follow red-green-refactor and cover deterministic and changed semantic
digests, successful and failed bundle construction, nullable trace evidence,
relative path normalization and resolution, missing/tampered artifacts,
metadata and ledger mismatches, clean JSON CLI output, nonzero failure exits,
parent-directory creation, Bytecode 0.13 compatibility, Bytecode 0.14 default
emission, fixture equality, README conflict-marker removal, and preservation of
the simulated-only provider boundary.

## Documentation and Verification

README documents what a bundle is and is not, offline operation, semantic
digests, portable paths, failure evidence, commands, compatibility, and all
four project principles. Existing merge-conflict markers are removed.

Final verification:

```bash
cargo fmt
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Acceptance also parses all generated JSON, verifies intact evidence, rejects a
tampered artifact, confirms clean JSON stdout, compares the v0.14 fixture with
compiler output, and confirms external providers remain non-executable.
