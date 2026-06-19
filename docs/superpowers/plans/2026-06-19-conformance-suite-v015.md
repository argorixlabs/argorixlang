# Argorix Lang v0.15 Conformance Suite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a portable, deterministic, data-driven Conformance Suite that validates the Argorix compiler, Bytecode verifier, VM, reports, and evidence pipeline through direct library APIs.

**Architecture:** Create `argorix_conformance` as a library plus the `argorix-conformance` binary. The library validates suite data, resolves fixtures relative to the suite, executes typed stages into isolated case workdirs, applies declarative mutations, and returns deterministic serializable results; the CLI only handles arguments, rendering, and exit status.

**Tech Stack:** Rust 2021, Cargo workspace, serde, serde_json, clap, anyhow, existing Argorix parser/semantics/IR/Bytecode/VM crates.

---

### Task 1: Public conformance model

**Files:**
- Create: `crates/argorix_conformance/Cargo.toml`
- Create: `crates/argorix_conformance/src/lib.rs`
- Create: `crates/argorix_conformance/src/types.rs`
- Create: `crates/argorix_conformance/tests/types.rs`
- Modify: `Cargo.toml`

- [ ] Add failing tests that deserialize `ConformanceSuite`, serialize `ConformanceResult`, and assert stable field names and stage statuses.
- [ ] Run `cargo test -p argorix_conformance --test types` and confirm the crate/types are absent.
- [ ] Add the workspace member and public serializable types exactly matching the approved schema.
- [ ] Run the targeted tests and require all pass.

### Task 2: Suite validation and portable paths

**Files:**
- Create: `crates/argorix_conformance/src/validation.rs`
- Create: `crates/argorix_conformance/tests/validation.rs`
- Modify: `crates/argorix_conformance/src/lib.rs`

- [ ] Add failing tests for unknown/duplicate/dependency-ordered stages, invalid expected failures, missing injection, invalid mutation, invalid case IDs, duplicate IDs, missing categories, and paths escaping the suite tree.
- [ ] Add a test that changes cwd and still resolves `sources/example.argx` from the suite parent.
- [ ] Run targeted tests and confirm validation/resolution APIs are absent.
- [ ] Implement `validate_suite`, stage dependency rules, injection parsing, JSON Pointer syntax checks, portable path resolution, and deterministic diagnostics.
- [ ] Run targeted tests.

### Task 3: Shared VM injection parser

**Files:**
- Create: `crates/argorix_vm/src/injection.rs`
- Modify: `crates/argorix_vm/src/lib.rs`
- Modify: `crates/argorix-vm/src/main.rs`
- Create: `crates/argorix_vm/tests/injection.rs`

- [ ] Add failing library tests for valid and invalid `from:to:act:message_type`.
- [ ] Run targeted tests and confirm the public parser is absent.
- [ ] Move parsing into `argorix_vm::parse_injection`; make the VM CLI delegate to it.
- [ ] Run VM library and CLI tests.

### Task 4: Source and Bytecode stages

**Files:**
- Create: `crates/argorix_conformance/src/runner.rs`
- Create: `crates/argorix_conformance/src/workspace.rs`
- Create: `crates/argorix_conformance/tests/pipeline.rs`
- Modify: `crates/argorix_conformance/src/lib.rs`

- [ ] Add failing tests for parse, semantic check, IR emission, Bytecode emission, fixture Bytecode loading, and Bytecode verification.
- [ ] Assert fixed workdir names `ir.json` and `program.argbc.json`, no absolute paths in results, and isolated `<workdir>/<case-id>/`.
- [ ] Run targeted tests and confirm runner APIs are absent.
- [ ] Implement `run_suite`/`run_case`, typed per-case state, deterministic JSON writers, clean case-directory recreation, and stages through `verify_bytecode`.
- [ ] Run targeted tests.

### Task 5: VM, report, trace, bundle, and evidence stages

**Files:**
- Modify: `crates/argorix_conformance/src/runner.rs`
- Modify: `crates/argorix_conformance/src/workspace.rs`
- Modify: `crates/argorix_conformance/tests/pipeline.rs`

- [ ] Add failing end-to-end tests for `run_vm`, `security_report`, `trace_out`, `evidence_bundle`, and `verify_evidence`.
- [ ] Assert artifacts parse, use relative bundle paths, and remain under the case workdir.
- [ ] Run targeted tests and confirm stage support is absent.
- [ ] Implement the VM/evidence stages with direct APIs and `ExecutionOutcome`.
- [ ] Run targeted tests.

### Task 6: Expected failures and skipped stages

**Files:**
- Modify: `crates/argorix_conformance/src/runner.rs`
- Create: `crates/argorix_conformance/tests/expected_failures.rs`

- [ ] Add failing tests for matched failure, diagnostic mismatch, expected stage unexpectedly passing, unexpected earlier failure, and later stages marked skipped.
- [ ] Run targeted tests.
- [ ] Implement expected-failure evaluation without changing the failing stage from `failed`.
- [ ] Run targeted tests.

### Task 7: Declarative mutations

**Files:**
- Create: `crates/argorix_conformance/src/mutation.rs`
- Modify: `crates/argorix_conformance/src/runner.rs`
- Create: `crates/argorix_conformance/tests/mutation.rs`

- [ ] Add failing tests mutating working Bytecode, SecurityReport, and bundle JSON before `verify_evidence`.
- [ ] Add failing tests for missing artifacts and unresolved JSON Pointers.
- [ ] Assert committed fixtures remain byte-for-byte unchanged.
- [ ] Run targeted tests.
- [ ] Implement allowed-artifact lookup, JSON Pointer replacement, and workdir-only rewrite.
- [ ] Run targeted tests.

### Task 8: CLI text/JSON and exit behavior

**Files:**
- Create: `crates/argorix_conformance/src/main.rs`
- Create: `crates/argorix_conformance/tests/cli.rs`
- Modify: `crates/argorix_conformance/Cargo.toml`

- [ ] Add failing real-binary tests for text summary, JSON-only stdout, default/explicit workdir, zero exit on passing suite, and nonzero exit on failed or invalid suite.
- [ ] Run targeted tests and confirm the binary is absent.
- [ ] Implement `run <suite> [--workdir] [--json]`, thin rendering, and exit behavior.
- [ ] Run targeted CLI tests.

### Task 9: Versions and v0.15 fixtures

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/argorix_ir/src/ir.rs`
- Modify: `crates/argorix_bytecode/src/bytecode.rs`
- Modify: `crates/argorix_bytecode/src/lower.rs`
- Modify: `crates/argorix_vm/src/vm.rs`
- Modify: `crates/argorix_vm/src/security_report.rs`
- Modify: `crates/argorix_vm/src/evidence.rs`
- Modify: `crates/argorixc/src/main.rs`
- Modify: `crates/argorix-vm/src/main.rs`
- Modify: version assertions across tests
- Create: `examples/provider_allowlists_v015.argx`
- Create: `examples/provider_allowlists_v015.argbc.json`

- [ ] Change version tests to expect 0.15 and add explicit Bytecode 0.14/EvidenceBundle 0.14 compatibility tests.
- [ ] Run targeted tests and observe version failures.
- [ ] Advance workspace/emitted versions, accept Bytecode 0.15, and keep 0.14 accepted.
- [ ] Generate the v0.15 Bytecode fixture from compiler APIs and structurally compare it.
- [ ] Run workspace tests.

### Task 10: Official portable suite

**Files:**
- Create: `conformance/.gitignore`
- Create: `conformance/suite.v015.json`
- Create: required files under `conformance/sources/`
- Create: required files under `conformance/bytecode/`

- [ ] Build a data-only suite covering all 13 categories and all mandatory positive/negative behaviors.
- [ ] Use only relative suite paths, explicit injections, expected failures, and declarative mutations.
- [ ] Run the suite and fix fixture/schema issues without adding case-specific runner logic.
- [ ] Assert generated reports/traces/bundles are ignored while suite/source/Bytecode fixtures are tracked.

### Task 11: README

**Files:**
- Modify: `README.md`

- [ ] Document v0.15 purpose, non-goals, direct-library execution, commands, result interpretation, case schema, expected failures, injection, mutations, suite-relative paths, workdir, compatibility, and all v0.15 principles.
- [ ] Update architecture, versions, examples, workspace map, and roadmap.
- [ ] Confirm no conflict markers and no stale current-version statements.

### Task 12: Acceptance and final verification

- [ ] Run compiler check/IR/Bytecode commands for v0.14 and v0.15 fixtures.
- [ ] Structurally compare v0.15 compiler output with the committed fixture.
- [ ] Run VM v0.15 with report, trace, and bundle exports; parse all artifacts.
- [ ] Verify intact evidence.
- [ ] Run official Conformance Suite in text and JSON; parse JSON and confirm stdout purity.
- [ ] Confirm declarative mutation cases pass as expected negative cases.
- [ ] Confirm Bytecode 0.14 verifies/runs and EvidenceBundle 0.14 verifies.
- [ ] Confirm external providers remain non-executable and only `simulated` is executable.
- [ ] Run `cargo fmt`.
- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo test --workspace`.
- [ ] Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] Run `git diff --check` and inspect final status.
