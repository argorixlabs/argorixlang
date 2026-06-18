# Argorix Lang v0.11 External Adapter Contracts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add declarative, validated external adapter contracts while keeping `simulated` as the only executable provider and blocking all external execution fail-closed.

**Architecture:** Extend the compiler pipeline with module-level provider contracts and stable empty allowlist fields in IR/Bytecode 0.11. Keep executable providers and declarative contracts in separate registry maps. The VM builds an execution-local registry, validates contracts before scheduling, emits audit events, and preserves runtime ledgers on rejected contracts or blocked external execution.

**Tech Stack:** Rust 2021, Cargo workspace, serde, serde_json, thiserror, clap.

---

### Task 1: Adapter contract type and separated registry

**Files:**
- Create: `crates/argorix_provider/src/contract.rs`
- Modify: `crates/argorix_provider/src/errors.rs`
- Modify: `crates/argorix_provider/src/registry.rs`
- Modify: `crates/argorix_provider/src/lib.rs`

- [ ] Add provider-crate tests that construct an external `AdapterContract`, register it, retrieve it, validate it, and confirm `is_enabled` is false.
- [ ] Add tests rejecting duplicate contracts, the reserved executable name `simulated`, enabled external contracts, missing dry-run/feature-flag/approval requirements, and non-empty reserved allowlists.
- [ ] Add a test proving `registry.get("OpenAI")` cannot return a declarative contract.
- [ ] Run `cargo test -p argorix_provider`; expect the new tests to fail because the API is absent.
- [ ] Implement serializable `AdapterContract`, contract-specific errors, the separate `contracts` map, and contract query/validation methods.
- [ ] Run `cargo test -p argorix_provider`; expect all provider tests to pass.

### Task 2: Parser AST and semantic contract rules

**Files:**
- Modify: `crates/argorix_parser/src/ast.rs`
- Modify: `crates/argorix_parser/src/parser.rs`
- Modify: `crates/argorix_semantics/src/checker.rs`
- Modify: `tests/compiler_tests.rs`

- [ ] Add parser tests for `provider OpenAI`, `kind external`, both booleans, and both `requires` clauses.
- [ ] Add parser rejection coverage for unsupported public `allowed_targets` and `allowed_capabilities` syntax.
- [ ] Add semantic tests for one valid disabled external contract and rejection of enabled, non-dry-run, missing feature flag, missing approval, duplicate, and reserved `simulated` declarations.
- [ ] Add semantic tests proving tools and models cannot use a declared external contract.
- [ ] Run the targeted parser/compiler tests; expect failures because provider declarations are unsupported.
- [ ] Add `ProviderDecl` and `ProviderKindDecl` to the AST and parse the fixed-order v0.11 declaration grammar.
- [ ] Add semantic contract validation before tool/model checks while retaining simulated-only execution rules.
- [ ] Run the targeted tests; expect them to pass.

### Task 3: IR and Bytecode 0.11 contracts

**Files:**
- Modify: `crates/argorix_ir/src/ir.rs`
- Modify: `crates/argorix_bytecode/src/bytecode.rs`
- Modify: `crates/argorix_bytecode/src/lower.rs`
- Modify: `crates/argorixc/src/main.rs`
- Modify: `tests/compiler_tests.rs`

- [ ] Add tests asserting IR version `0.11`, top-level declarative `providers`, and empty `allowed_targets`/`allowed_capabilities`.
- [ ] Add tests asserting Bytecode version `0.11`, top-level contracts, and `DeclareProviderContract` before tool/model/agent declarations.
- [ ] Add bytecode-verifier tests for invalid contracts, duplicate contracts, reserved-name collisions, mismatched declaration instructions, and v0.10 compatibility.
- [ ] Run targeted IR/bytecode/compiler tests; expect failures on missing structures and old versions.
- [ ] Implement `IrProviderContract`, `BytecodeProviderContract`, stable serde defaults, `DeclareProviderContract`, lowering, and v0.11 verifier rules.
- [ ] Update compiler and bytecode-verifier version text to `0.11`.
- [ ] Run targeted tests; expect them to pass.

### Task 4: VM loading, audit events, and fail-closed execution

**Files:**
- Modify: `crates/argorix_vm/src/errors.rs`
- Modify: `crates/argorix_vm/src/runtime.rs`
- Modify: `crates/argorix_vm/src/reactive.rs`
- Modify: `crates/argorix_vm/src/trace.rs`
- Modify: `crates/argorix_vm/src/vm.rs`
- Modify: `crates/argorix_vm/src/lib.rs`

- [ ] Add VM tests for contract loading, `ProviderContractDeclared`, `ProviderContractValidated`, and JSON `provider_contracts`.
- [ ] Add runtime tests that inject invalid/mutated bytecode and observe `ProviderContractRejected` with a preserved ledger.
- [ ] Add runtime tests that mutate a tool/model provider to a registered external contract and observe `ExternalProviderExecutionBlocked` with a preserved ledger.
- [ ] Add compatibility coverage for reactive execution of the v0.10 fixture without contracts.
- [ ] Run targeted VM tests; expect failures because contracts are not loaded or reported.
- [ ] Add provider contract summaries and `enabled` to executable provider summaries.
- [ ] Build an execution-local registry, record declaration/validation events before scheduling, and pass it to the reactive scheduler.
- [ ] Detect contract-only names during provider lookup, record blocked-execution events, activate failure mode, fail state, and return the existing provider-boundary error.
- [ ] Set reactive VM output to version `0.11`.
- [ ] Run targeted VM tests; expect them to pass.

### Task 5: CLI, examples, generated fixtures, and documentation

**Files:**
- Modify: `crates/argorix-vm/src/main.rs`
- Create: `examples/provider_contracts_v011.argx`
- Create: `examples/provider_contracts_v011.argbc.json`
- Create: `examples/provider_external_enabled.argx`
- Create: `examples/provider_external_missing_feature_flag.argx`
- Create: `examples/provider_external_missing_approval.argx`
- Create: `examples/provider_external_used_by_model.argx`
- Create: `examples/provider_external_used_by_tool.argx`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-06-18-external-adapter-contracts-v011-design.md`

- [ ] Add a CLI parser test for `--provider-contracts`; run it and expect failure.
- [ ] Implement separated textual provider/contract output and the blocked-by-design statement; update VM version text and dry-run error text.
- [ ] Build the valid fixture from `provider_boundary_v010.argx`, add the five invalid fixtures, and generate the committed Bytecode 0.11 JSON through `argorixc`.
- [ ] Update README architecture, commands, JSON shape, instructions, compatibility, workspace, examples, and roadmap.
- [ ] Add the two explicit documentation clarifications: `simulated` is default-only and cannot be a contract; top-level IR/Bytecode `providers` means declarative contracts, not executable instances.
- [ ] Run CLI tests and check documentation/fixture diffs for consistency.

### Task 6: Required command and regression verification

- [ ] Run `cargo run -p argorixc -- check examples/provider_contracts_v011.argx`; expect exit code 0.
- [ ] Run `cargo run -p argorixc -- emit-ir examples/provider_contracts_v011.argx`; verify IR 0.11 contract arrays.
- [ ] Run `cargo run -p argorixc -- emit-bytecode examples/provider_contracts_v011.argx`; verify Bytecode 0.11 and `DeclareProviderContract`.
- [ ] Run both requested reactive VM commands with `--provider-contracts`; verify text and JSON reports.
- [ ] Run each of the five invalid compiler checks independently; expect nonzero exit codes and the intended diagnostic.
- [ ] Run the v0.10 bytecode fixture through verifier/runtime compatibility tests.
- [ ] Run `cargo fmt`.
- [ ] Run `cargo fmt --all -- --check`; expect exit code 0.
- [ ] Run `cargo test --workspace`; expect zero failures.
- [ ] Run `cargo clippy --workspace --all-targets -- -D warnings`; expect exit code 0.
- [ ] Review `git diff --check` and `git status --short`, preserving unrelated/local v0.10 changes.
