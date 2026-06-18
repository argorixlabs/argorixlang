# Argorix Lang v0.12 Provider Contract Allowlists Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Activate declarative provider target/capability allowlists while preserving simulated-only execution and fail-closed external-provider behavior.

**Architecture:** Parser and AST preserve optional allowlist blocks and spans. Compiler semantics and the Bytecode verifier resolve target/capability compatibility independently, while `ProviderRegistry` validates only structural contract invariants. IR, Bytecode, VM JSON, and CLI preserve list order without interpreting allowlisting as execution authority.

**Tech Stack:** Rust 2021, Cargo workspace, serde, serde_json, thiserror, clap.

---

### Task 1: Parser and AST allowlist blocks

**Files:**
- Modify: `crates/argorix_parser/src/ast.rs`
- Modify: `crates/argorix_parser/src/parser.rs`
- Modify: `tests/compiler_tests.rs`

- [ ] Add tests parsing both blocks in both orders, preserving ordered `Spanned<String>` entries, accepting absent blocks, and rejecting duplicate blocks.
- [ ] Run targeted parser/compiler tests and confirm failure because `ProviderDecl` has no allowlists.
- [ ] Add both vectors to `ProviderDecl`; parse zero, one, or both post-requirement blocks in arbitrary order; reject a repeated block.
- [ ] Run targeted tests and confirm passes.

### Task 2: Semantic allowlist resolution

**Files:**
- Modify: `crates/argorix_semantics/src/checker.rs`
- Modify: `tests/compiler_tests.rs`

- [ ] Add tests for valid model/tool allowlists; unknown, duplicate, ambiguous, and incompatible entries; and continued external model/tool rejection.
- [ ] Run targeted tests and confirm semantic failures are absent or incorrect.
- [ ] Resolve targets directly against `Program.tools` and `Program.models`, capabilities against the global registry, and report duplicate elements at the repeated span.
- [ ] Enforce compatibility only when the capability list is populated; retain empty-list zero-permission semantics and simulated-only execution.
- [ ] Run targeted tests and confirm passes.

### Task 3: IR 0.12 lowering

**Files:**
- Modify: `crates/argorix_ir/src/ir.rs`
- Modify: `tests/compiler_tests.rs`

- [ ] Add tests requiring IR `0.12`, populated ordered arrays, and empty arrays for v0.11-style source.
- [ ] Run targeted tests and confirm old version/empty-only lowering failures.
- [ ] Lower AST allowlists without sorting or deduplication and advance IR version to `0.12`.
- [ ] Run targeted tests and confirm passes.

### Task 4: Bytecode 0.12 lowering and verification

**Files:**
- Modify: `crates/argorix_bytecode/src/bytecode.rs`
- Modify: `crates/argorix_bytecode/src/lower.rs`
- Modify: `crates/argorixc/src/main.rs`
- Modify: `tests/compiler_tests.rs`

- [ ] Add tests for Bytecode `0.12` populated arrays, Bytecode `0.11` empty-array compatibility, duplicate/unknown/ambiguous/incompatible failures, and exact top-level/instruction correspondence.
- [ ] Run targeted tests and confirm failures against the v0.11 verifier.
- [ ] Emit Bytecode `0.12`, accept `0.12`, keep `0.11` empty-only, validate lists against Bytecode declarations, and compare complete ordered contract declarations.
- [ ] Update compiler/version text to `0.12`.
- [ ] Run targeted tests and confirm passes.

### Task 5: ProviderRegistry structural adjustment

**Files:**
- Modify: `crates/argorix_provider/src/registry.rs`
- Modify: `crates/argorix_provider/src/lib.rs`
- Modify: `crates/argorix_provider/src/errors.rs`

- [ ] Add a test accepting populated allowlists while retaining every external-contract and simulated-only executable invariant.
- [ ] Run provider tests and confirm populated contracts fail under v0.11 rules.
- [ ] Remove only the empty-list restrictions and update versioned structural messages.
- [ ] Run provider tests and confirm passes.

### Task 6: VM, JSON, and CLI reporting

**Files:**
- Modify: `crates/argorix_vm/src/vm.rs`
- Modify: `crates/argorix_vm/src/reactive.rs`
- Modify: `crates/argorix-vm/src/main.rs`

- [ ] Add VM tests for ordered populated lists, `vm_version: 0.12`, v0.11 compatibility, and runtime blocking despite a valid allowlist.
- [ ] Add CLI formatting tests/helpers for populated lists and empty-list `none`.
- [ ] Run targeted tests and confirm old version/text behavior.
- [ ] Advance reactive VM output to `0.12`, preserve lists unchanged, print indented lists or `none`, and retain `ExternalProviderExecutionBlocked`.
- [ ] Run targeted tests and confirm passes.

### Task 7: Source and Bytecode fixtures

**Files:**
- Create: `examples/provider_allowlists_v012.argx`
- Create: `examples/provider_allowlists_v012.argbc.json`
- Create: `examples/provider_allowlists_tools_v012.argx`
- Create: `examples/provider_allowlists_tools_v012.argbc.json`
- Create: `examples/provider_allowlist_unknown_target.argx`
- Create: `examples/provider_allowlist_unknown_capability.argx`
- Create: `examples/provider_allowlist_duplicate_target.argx`
- Create: `examples/provider_allowlist_duplicate_capability.argx`
- Create: `examples/provider_allowlist_incompatible_capability.argx`
- Create: `examples/provider_allowlist_external_execution_still_blocked.argx`

- [ ] Add the two valid source fixtures and six single-purpose invalid fixtures.
- [ ] Run valid checks and all invalid checks; confirm expected exit behavior.
- [ ] Generate both Bytecode fixtures from the compiler.
- [ ] Compare parsed generated JSON structurally with committed fixtures.

### Task 8: Versioning and documentation

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-06-18-provider-contract-allowlists-v012-design.md`

- [ ] Advance workspace packages and user-facing version references to `0.12`.
- [ ] Document public syntax, arbitrary block order, duplicate rules, v0.11 compatibility, `none`, zero future permissions, no-wildcard behavior, and `Allowlisted does not mean executable`.
- [ ] Review examples, instruction lists, compatibility matrix, workspace descriptions, and roadmap for version consistency.

### Task 9: Required verification

- [ ] Run all requested valid compiler/VM commands and inspect text/JSON allowlist output.
- [ ] Run six invalid fixture checks and confirm every exit code is nonzero with the intended diagnostic.
- [ ] Structurally compare both committed Bytecode fixtures against fresh compiler output.
- [ ] Run `cargo fmt`.
- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo test --workspace`.
- [ ] Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] Run `git diff --check` and inspect `git status --short`.
