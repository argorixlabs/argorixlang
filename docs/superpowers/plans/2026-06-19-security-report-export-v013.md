# Argorix Lang v0.13 Security Report Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Export deterministic security evidence reports from successful and failed reactive VM executions.

**Architecture:** Add `ExecutionOutcome` so runtime state and ledger survive every execution path. Build a public `SecurityReport` module from outcome evidence, with deterministic SHA-256 ledger digest and evidence-driven verdict. Extend the CLI to write pretty report JSON before returning execution errors while preserving existing stdout formats.

**Tech Stack:** Rust 2021, Cargo workspace, serde, serde_json, sha2, clap, thiserror.

---

### Task 1: ExecutionOutcome and preserved failure state

**Files:**
- Modify: `crates/argorix_vm/src/vm.rs`
- Modify: `crates/argorix_vm/src/lib.rs`
- Modify: `crates/argorix_vm/src/runtime.rs`

- [ ] Add tests for successful and failed `run_reactive_outcome()`, asserting state, status, ledger, and compatibility wrapper behavior.
- [ ] Run targeted VM tests and confirm the API is absent.
- [ ] Implement `ExecutionOutcome`, move reactive execution into `run_reactive_outcome()`, preserve state on every error, and delegate `run_reactive()`.
- [ ] Run targeted VM tests and confirm passes.

### Task 2: SecurityReport types and successful summaries

**Files:**
- Create: `crates/argorix_vm/src/security_report.rs`
- Modify: `crates/argorix_vm/src/lib.rs`
- Modify: `crates/argorix_vm/Cargo.toml`
- Modify: `Cargo.toml`

- [ ] Add tests constructing a report from the successful v0.13 fixture and asserting versions, execution, policy, providers, allowlists, calls, and real 3/3/6 intrinsics.
- [ ] Run tests and confirm missing types/builders.
- [ ] Add public serializable report/summary types and `SecurityReport::from_outcome`.
- [ ] Run tests and confirm successful summaries pass.

### Task 3: Ledger digest and event-kind summary

**Files:**
- Modify: `crates/argorix_vm/src/security_report.rs`
- Modify: `Cargo.toml`

- [ ] Add deterministic-same-ledger and changed-ledger digest tests, event-kind counts, first/last event tests.
- [ ] Run tests and confirm digest behavior is absent.
- [ ] Add workspace `sha2`, compact ordered-event JSON hashing, `sha256:` formatting, and `BTreeMap` event counts.
- [ ] Run digest tests and confirm passes.

### Task 4: Failed reports and evidence-driven verdicts

**Files:**
- Modify: `crates/argorix_vm/src/security_report.rs`
- Modify: `crates/argorix_vm/src/vm.rs`
- Modify: `crates/argorix_vm/src/reactive.rs`

- [ ] Add tests for informational, pass, policy-failure, runtime-failure, and external-provider-blocked verdicts.
- [ ] Add tests for failed policy reconstruction, blocked attempts, activated failures, and preserved ledger.
- [ ] Run targeted tests and confirm incorrect/missing verdicts.
- [ ] Implement summary fallback from state/events/error and precedence rules with concrete reasons.
- [ ] Run targeted tests and confirm passes.

### Task 5: Versions and v0.13 fixtures

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/argorix_ir/src/ir.rs`
- Modify: `crates/argorix_bytecode/src/bytecode.rs`
- Modify: `crates/argorix_bytecode/src/lower.rs`
- Modify: `crates/argorix_vm/src/vm.rs`
- Modify: `crates/argorixc/src/main.rs`
- Create: `examples/provider_allowlists_v013.argx`
- Create: `examples/provider_allowlists_v013.argbc.json`

- [ ] Add tests for default IR/Bytecode/VM 0.13 and Bytecode 0.12 compatibility.
- [ ] Run tests and confirm old emitted versions.
- [ ] Advance workspace and emitted versions, accept Bytecode 0.13, and preserve version-specific v0.11/v0.12 allowlist rules.
- [ ] Generate v0.13 Bytecode from source and structurally compare it with compiler output.
- [ ] Run version and compatibility tests.

### Task 6: CLI security-report export

**Files:**
- Modify: `crates/argorix-vm/src/main.rs`
- Modify: `crates/argorix-vm/Cargo.toml`
- Create: `reports/.gitignore`

- [ ] Add CLI parser and real-binary integration tests for successful report writing, parent creation, clean JSON stdout, failed report writing, and nonzero failed exit.
- [ ] Run tests and confirm `--security-report` is absent.
- [ ] Add the path flag, always use `run_reactive_outcome()`, write report before result propagation, and suppress report messages in JSON mode.
- [ ] Add `reports/.gitignore` with `*.security.json`.
- [ ] Run CLI and integration tests.

### Task 7: Documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-06-19-security-report-export-v013-design.md`

- [ ] Document report contents, CLI usage, failed-report behavior, digest limits, non-signature status, no real-world safety proof, provider-boundary evidence, and existing allowlist rule.
- [ ] Correct the example intrinsic counts to 3/3/6.
- [ ] Update architecture, commands, compatibility, workspace, examples, and roadmap to v0.13.

### Task 8: Final verification

- [ ] Run `cargo fmt`.
- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo test --workspace`.
- [ ] Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] Run all requested valid compiler and VM report commands.
- [ ] Confirm report JSON fields, digest prefix, allowlists, verdict, and clean JSON stdout.
- [ ] Run failed execution through the real CLI; confirm report exists and exit is nonzero.
- [ ] Confirm all six v0.12 invalid fixtures remain nonzero.
- [ ] Structurally compare the committed v0.13 Bytecode fixture with fresh compiler output.
- [ ] Run `git diff --check` and inspect final status.
