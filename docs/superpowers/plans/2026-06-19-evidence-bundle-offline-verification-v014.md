# Argorix Lang v0.14 Evidence Bundle and Offline Verification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Export portable semantic evidence bundles and verify their Bytecode, trace, report, and ledger consistency offline.

**Architecture:** Add a focused public `argorix_vm::evidence` module that owns canonical hashing, relative path conversion, bundle construction, and verification. Keep the CLI responsible only for file orchestration and output formatting, while preserving `ExecutionOutcome` behavior for failed runs.

**Tech Stack:** Rust 2021, Cargo workspace, serde, serde_json, sha2, clap, anyhow.

---

### Task 1: Evidence types and semantic digests

**Files:**
- Create: `crates/argorix_vm/src/evidence.rs`
- Create: `crates/argorix_vm/tests/evidence.rs`
- Modify: `crates/argorix_vm/src/lib.rs`
- Modify: `crates/argorix_vm/src/security_report.rs`

- [ ] Write tests for public types, deterministic Bytecode/trace/report digests, whitespace independence after deserialization, semantic changes, and digest syntax.
- [ ] Run `cargo test -p argorix_vm --test evidence` and confirm missing evidence APIs.
- [ ] Implement serializable types and shared `canonical_digest`.
- [ ] Route ledger hashing through `canonical_digest`.
- [ ] Run the targeted tests and existing security-report tests.

### Task 2: Portable bundle construction

**Files:**
- Modify: `crates/argorix_vm/src/evidence.rs`
- Modify: `crates/argorix_vm/tests/evidence.rs`

- [ ] Write tests for success metadata, nullable failed trace, normalized `/` paths, bundle-relative sibling paths, and rejected external absolute paths.
- [ ] Run targeted tests and confirm construction behavior is absent.
- [ ] Implement `EvidenceBundle::from_outcome` and portable path conversion.
- [ ] Run targeted tests.

### Task 3: Offline verifier

**Files:**
- Modify: `crates/argorix_vm/src/evidence.rs`
- Modify: `crates/argorix_vm/tests/evidence.rs`

- [ ] Write intact, whitespace-only, missing artifact, tampered Bytecode/report, ledger mismatch, version mismatch, malformed digest, and trace pair consistency tests.
- [ ] Run targeted tests and confirm verifier APIs are absent.
- [ ] Implement `verify_evidence` with accumulated checks and diagnostics.
- [ ] Run targeted tests.

### Task 4: CLI export and verification

**Files:**
- Modify: `crates/argorix-vm/src/main.rs`
- Create: `crates/argorix-vm/tests/evidence_cli.rs`
- Modify: `crates/argorix-vm/tests/security_report_cli.rs`

- [ ] Write parser and real-binary tests for `--trace-out`, `--evidence-bundle`, `verify-evidence`, JSON cleanliness, parent creation, portable paths, failed-run bundles, and nonzero failure exits.
- [ ] Run CLI tests and confirm flags/subcommand are absent.
- [ ] Implement file writers and delegate bundle construction/verification to `argorix_vm::evidence`.
- [ ] Run CLI tests.

### Task 5: Versioning and fixtures

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/argorix_ir/src/ir.rs`
- Modify: `crates/argorix_bytecode/src/lower.rs`
- Modify: `crates/argorix_bytecode/src/bytecode.rs`
- Modify: `crates/argorix_vm/src/vm.rs`
- Modify: `crates/argorix_vm/src/security_report.rs`
- Modify: `crates/argorixc/src/main.rs`
- Modify: version assertions across tests
- Create: `examples/provider_allowlists_v014.argx`
- Create: `examples/provider_allowlists_v014.argbc.json`

- [ ] Change version assertions first and run targeted tests to observe v0.13 failures.
- [ ] Advance emitted/workspace versions to v0.14 and accept Bytecode 0.14 while retaining 0.13.
- [ ] Copy the approved source fixture and generate Bytecode using `argorixc`.
- [ ] Structurally compare generated and committed Bytecode JSON.
- [ ] Run workspace tests.

### Task 6: Reports ignore rules and README

**Files:**
- Modify: `.gitignore`
- Modify: `reports/.gitignore`
- Modify: `README.md`

- [ ] Remove all merge-conflict markers while preserving the intended v0.13 content.
- [ ] Document v0.14 architecture, commands, digest semantics and limits, portable paths, failure behavior, compatibility, and provider restrictions.
- [ ] Add ignore patterns for security reports, bundles, and traces.
- [ ] Run `rg -n "^(<<<<<<<|=======|>>>>>>>)" README.md` and require no matches.

### Task 7: Acceptance and final verification

- [ ] Run compiler check, IR emission, and Bytecode emission for the v0.14 fixture.
- [ ] Run the VM command producing report, trace, and bundle.
- [ ] Parse all three generated JSON artifacts.
- [ ] Run text and JSON offline verification and require clean JSON stdout.
- [ ] Tamper with a copied artifact and require nonzero verification.
- [ ] Confirm failed execution writes report/bundle and exits nonzero.
- [ ] Confirm Bytecode 0.13 still verifies and executes.
- [ ] Confirm external providers remain non-executable and `simulated` remains the only executable provider.
- [ ] Run `cargo fmt`.
- [ ] Run `cargo test --workspace`.
- [ ] Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] Run `git diff --check` and inspect final status.
