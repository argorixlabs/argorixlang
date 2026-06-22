# Argorix Lang v0.20 Sandboxed Provider Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add declarative, offline-verifiable provider containment metadata without enabling external provider execution.

**Architecture:** Extend the existing passport-style metadata pipeline with source-faithful harness AST values, semantic validation, deterministic package merging, IR/Bytecode 0.20 metadata and VM evidence propagation. Keep provider execution unchanged and fail-closed: harnesses are never instructions and `simulated` remains the only executable provider.

**Tech Stack:** Rust workspace, custom lexer/parser/semantic checker, serde JSON IR/Bytecode, reactive VM, conformance runner.

---

## File Structure

- `crates/argorix_parser/src/{lexer,ast,parser}.rs`: integer tokens, harness AST and structural parser.
- `crates/argorix_semantics/src/{symbols,checker}.rs`: harness names, required fields, enums, references and limits.
- `crates/argorix_module/src/merge.rs`: deterministic package harness merge.
- `crates/argorix_ir/src/ir.rs`: IR 0.20 harness metadata.
- `crates/argorix_bytecode/src/{bytecode,lower,lib}.rs`: Bytecode 0.20 metadata and verifier.
- `crates/argorix_vm/src/{trace,runtime,reactive,policy,vm,security_report,evidence,lib}.rs`: evidence-only runtime propagation.
- `crates/argorixc/src/main.rs`, `crates/argorix-vm/src/main.rs`: v0.20 CLI identity and outputs.
- `examples/`, `conformance/`: valid, invalid, package and official-suite fixtures.
- `README.md`, `Cargo.toml`: release documentation and workspace version.

### Task 1: Lexer, AST and parser

- [ ] Add lexer tests in `crates/argorix_parser/src/lexer.rs` proving `10` and `1000` become `IntegerLiteral(u64)` and malformed numeric syntax is rejected.
- [ ] Run `cargo test -p argorix_parser lexer::tests -- --nocapture` and confirm RED because integer tokens do not exist.
- [ ] Add `TokenKind::IntegerLiteral(u64)` and unsigned decimal scanning without floats or signs.
- [ ] Add parser tests in `crates/argorix_parser/src/parser.rs` for required fields, optional limits/contracts, empty attestations, unknown enum preservation and malformed structure.
- [ ] Run `cargo test -p argorix_parser parser::tests -- --nocapture` and confirm RED because `harness` is not a top-level declaration.
- [ ] Add `Program.harnesses`, `ProviderHarnessDecl`, the four harness enums and `source_name()` helpers in `crates/argorix_parser/src/ast.rs`.
- [ ] Parse harness fields with duplicate-field rejection, required-field sentinels, `Option<u64>` limits and `Vec<Spanned<String>>` attestations.
- [ ] Run `cargo test -p argorix_parser` and confirm GREEN.
- [ ] Commit parser slice.

### Task 2: Semantic checker

- [ ] Add tests in `crates/argorix_semantics/src/checker.rs` for duplicate names, all missing required fields, unknown provider, invalid enum values, zero limits, missing contracts and empty attestation entries.
- [ ] Add positive tests for empty attestations and valid `dry_run`/`simulated`, `none`/`read_only` combinations.
- [ ] Run `cargo test -p argorix_semantics harness -- --nocapture` and confirm RED.
- [ ] Extend symbol collection with provider/type lookup used by harness validation.
- [ ] Add a focused `check_provider_harnesses` pass that rejects sentinels and `Unknown` values before lowering.
- [ ] Preserve existing external-provider contract checks unchanged.
- [ ] Run `cargo test -p argorix_semantics` and confirm GREEN.
- [ ] Commit semantic slice.

### Task 3: Module/package merge

- [ ] Add module tests in `crates/argorix_module/src/merge.rs` for imported harness inclusion, unimported exclusion through resolver coverage, deterministic ordering and duplicate cross-module rejection.
- [ ] Run `cargo test -p argorix_module harness -- --nocapture` and confirm RED.
- [ ] Initialize and extend `Program.harnesses` in deterministic merge order.
- [ ] Run `cargo test -p argorix_module` and confirm GREEN.
- [ ] Commit module slice.

### Task 4: IR 0.20

- [ ] Add IR tests in `crates/argorix_ir/src/ir.rs` asserting ordered `provider_harnesses`, null optional values, empty attestations and `ir_version == "0.20"`.
- [ ] Run `cargo test -p argorix_ir harness -- --nocapture` and confirm RED.
- [ ] Add `IrProviderHarness` and lower only semantic-valid enum source names and values.
- [ ] Advance IR version to 0.20 while preserving all v0.19 metadata.
- [ ] Run `cargo test -p argorix_ir` and confirm GREEN.
- [ ] Commit IR slice.

### Task 5: Bytecode 0.20 and verifier

- [ ] Add lowering tests in `crates/argorix_bytecode/src/lower.rs` for harness metadata and absence of harness instructions.
- [ ] Add verifier tests in `crates/argorix_bytecode/src/bytecode.rs` for 0.19/0.20 acceptance and each invalid harness structure.
- [ ] Run `cargo test -p argorix_bytecode harness -- --nocapture` and confirm RED.
- [ ] Add `BytecodeProviderHarness`, `BytecodeProgram.provider_harnesses` with serde defaults and exports.
- [ ] Advance lowering to Bytecode 0.20 without adding an `Instruction` variant.
- [ ] Add `HarnessesRequireV020`, duplicate/invalid harness diagnostics and reference validation against providers/types.
- [ ] Update historical version guards so existing metadata remains accepted in 0.20.
- [ ] Run `cargo test -p argorix_bytecode` and confirm GREEN.
- [ ] Commit Bytecode slice.

### Task 6: VM trace and ledger

- [ ] Add VM tests asserting harness metadata is copied to `ReactiveExecutionTrace` and declaration/validation/sandbox events are present.
- [ ] Add a regression test proving external-provider model/tool execution remains blocked even when a harness exists.
- [ ] Run targeted VM tests and confirm RED.
- [ ] Add harness metadata to runtime state/trace construction and event variants `ProviderHarnessDeclared`, `ProviderHarnessValidated`, `ProviderHarnessSandboxed`, `ProviderHarnessRejected`.
- [ ] Record only metadata events during initialization; do not route harnesses through `ProviderRegistry`.
- [ ] Advance VM trace/version strings to 0.20.
- [ ] Run `cargo test -p argorix_vm` and confirm GREEN.
- [ ] Commit VM trace slice.

### Task 7: Policy Language v2

- [ ] Add parser/semantic policy-name tests for all six rules.
- [ ] Add VM policy tests covering declared/missing, sandbox dimensions and external-provider coverage.
- [ ] Run targeted parser, semantics and VM policy tests and confirm RED.
- [ ] Add the six `PolicyRule` variants, source names and Bytecode verifier allowlist entries.
- [ ] Extend `PolicyEvidenceContext` with harness booleans derived only from verified Bytecode metadata.
- [ ] Evaluate universal rules with vacuous truth and use `provider_harness_declared` for existence.
- [ ] Run targeted policy tests and confirm GREEN.
- [ ] Commit policy slice.

### Task 8: SecurityReport and EvidenceBundle

- [ ] Add SecurityReport tests for counts, sorted unique lists, optional contracts and non-escalating verdict behavior.
- [ ] Add EvidenceBundle tests for v0.20 digest verification, mutation detection and v0.19 compatibility.
- [ ] Run `cargo test -p argorix_vm security_report evidence -- --nocapture` in separate targeted invocations and confirm RED.
- [ ] Add `ProviderHarnessSummary`, populate it from Bytecode metadata and advance report version.
- [ ] Advance bundle version to 0.20 and expand the accepted offline version list through 0.20.
- [ ] Preserve the existing digest algorithm and artifact graph.
- [ ] Run all `argorix_vm` tests and confirm GREEN.
- [ ] Commit report/evidence slice.

### Task 9: CLI, fixtures and package example

- [ ] Add CLI assertions for compiler/VM v0.20 and harness-containing output artifacts.
- [ ] Create `examples/provider_harness_v020.argx`, the package tree and invalid harness fixtures.
- [ ] Run compiler checks against the valid fixture and each invalid fixture; confirm expected RED before completing compiler integration.
- [ ] Advance workspace/crate and CLI version strings to 0.20.
- [ ] Generate `examples/provider_harness_v020.argbc.json` through `argorixc emit-bytecode`.
- [ ] Run single-file/package compile, Bytecode verify, VM dry-run, report/trace/bundle generation and evidence verification.
- [ ] Commit CLI/fixture slice.

### Task 10: Conformance Suite 0.20

- [ ] Add conformance validation tests accepting suite 0.20 and the `provider_harness` category.
- [ ] Add source/module/bytecode fixtures and `conformance/suite.v020.json` positive and negative cases.
- [ ] Run official suite tests and confirm RED before updating validation/version gates.
- [ ] Extend conformance validation and runner expectations for v0.20 artifacts.
- [ ] Run Conformance Suite 0.20 in text and JSON modes and confirm GREEN.
- [ ] Commit conformance slice.

### Task 11: README and security-boundary audit

- [ ] Update README with syntax, field semantics, provider contract/harness distinction, policy/report/evidence behavior and explicit non-execution limits.
- [ ] Search production code for newly introduced network, socket, HTTP, environment-secret and external-execution APIs:

```powershell
rg -n "reqwest|hyper|TcpStream|UdpSocket|std::net|std::env|API_KEY|OPENAI|ANTHROPIC|Provider::execute|run-provider|test-openai" crates
```

- [ ] Confirm any matches are pre-existing labels/tests or reject the change if new operational capability was added.
- [ ] Commit documentation slice.

### Task 12: Final verification

- [ ] Run `cargo fmt`.
- [ ] Run `cargo test --workspace`.
- [ ] Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] Run the valid single-file and package CLI commands from the v0.20 specification.
- [ ] Run Conformance Suite 0.20 in text and JSON output modes.
- [ ] Run invalid fixture checks and confirm every command exits non-zero.
- [ ] Re-run Bytecode 0.19 and EvidenceBundle 0.19 compatibility tests.
- [ ] Inspect `git diff --check` and `git status --short`.
- [ ] Report exact command evidence and any remaining limitations.
