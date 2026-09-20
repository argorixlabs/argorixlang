# Claude lane

## Current claim

- Task: ESP-008.R follow-up — Rust harness, known-gap corpus, and differential
  testing against a spec oracle.
- State: DONE (pending review).
- Branch: `claude/core-c-harness-rust`. It supersedes the stacked branches
  `claude/core-c-gaps` (PR #28) and `claude/core-c-differential` (PR #29),
  which carried the same work in Python.
- Base: `4b0705a` through those branches.
- Started: 2026-09-19.
- Exclusive paths: `conformance/core_c/**`, `.github/workflows/core-c.yml`,
  and the new crate `crates/argorix_core_c/**`. One shared line added to the
  workspace `Cargo.toml` members list.
- Not touched: any Codex ESP-008/ESP-009 path.
- **The harness is Rust, at the maintainer's instruction.** The Python runner
  that landed in PR #25 is deleted here; ESP-008.R no longer adds a Python
  dependency to the project.

## Intended result

One `core-c-harness` binary that emits, compiles, executes, compares, inspects
dependencies, checks the known-gap corpus, and generates differential programs
whose expected result comes from an evaluator written against
`spec/core/evaluation.md`, never from `argorixc` or the C backend.

## Handoff

- The harness uses only workspace dependencies (anyhow, clap, serde,
  serde_json, sha2) and reads ELF itself, so `readelf` is no longer required
  and the Rust-free container installs only a C compiler.
- 40 Rust unit tests: ELF parsing, policy patterns, compiler allow-list,
  argument-vector rules, case validation, output comparison, gap
  classification for every status and kind, and the oracle's trap rules,
  truncating division, evaluation order, short-circuiting and determinism.
- Earlier evidence from the Python implementation of the same checks:
  **900 generated programs (seeds 42, 7, 13; 296 expected traps) all matched
  the oracle** with GCC 15.2 in WSL, emitted by `argorixc` built from
  `main@4b0705a`. No divergence in arithmetic, traps, evaluation order,
  short-circuiting, loops, or calls. The Rust port reproduces the same rules
  and is verified end to end by CI.
- New defect found by the generator and recorded as gap `g16`: an arithmetic
  expression containing an `if`, used as a comparison operand, is rejected with
  `CBackendUnsupported: checked arithmetic requires an integer`. Minimal repro:
  `if ((if c { 5u32 } else { 1u32 }) + 1u32) > 3u32 { 42u32 } else { 0u32 }`,
  which `core-check` accepts. Same root cause as `g09`. Reported on issue #27.
- CI: `harness-unit`, `emit`, `execute` (gcc and clang), `execute-rust-free`,
  `gap-corpus` and `differential`, all driven by `cargo` and the harness
  binary. The differential job uploads the generated programs with the report,
  so a failure is reproducible from the artifact.
- The Rust-free job carries the harness binary in with the bundle: that host
  has no Rust toolchain, which is the point, and the binary is stage0 tooling
  exactly like `argorixc`, not evidence of independence (ESP-024).
- Limit: the generator covers scalars only. Arrays, structs, enums, buffers,
  arenas, bytes, and UTF-8 are not generated yet; they are the obvious next
  extension once the aggregate gaps in issue #27 are fixed.

---

# Previous claim: MAT-029 (merged in PR #24)

## Claim

- Task: MAT-029 — language governance, support, and maintenance.
- State: DONE, merged in PR #24.
- Branch: `claude/governance-mat029`.
- Base: `3e620ab9` (`origin/main`, "docs: add shared collaboration workboard (#22)").
- Started: 2026-09-19.
- Dependency check: MAT-001 `HECHA` and ESP-001 `HECHA` in
  `tasks/madurez/BACKLOG.json` and `PLAN_ESPADA_INDEPENDIENTE.md`. The ready
  queue grouped MAT-029 under "MAT-008 onward: No", but its declared
  dependencies are complete; this claim corrects that row.
- Exclusive paths:
  - `GOVERNANCE.md` (new)
  - `SECURITY.md`
  - `CONTRIBUTING.md`
  - `operations/**` (new)
  - `tasks/madurez/MAT-029.md`
  - `spec/MAT-029-validation.json` (new)
  - `CODE_OF_CONDUCT.md`, `.github/CODEOWNERS`, `.github/PULL_REQUEST_TEMPLATE.md`
    (added during the task: broken contacts and review claim found in inspection)
- Shared closeout paths, edited only at task closure:
  - `README.md` (status/version lines only, if needed for AC-2)
  - `PLAN_MAESTRO_ARGORIXLANG.md`
  - `tasks/madurez/BACKLOG.json`
  - `WORKBOARD.md`
- No overlap with the Codex ESP-008 lane (`bootstrap/c/**`,
  `crates/argorix_ir/src/core_c.rs`, `crates/argorixc/src/main.rs`,
  `spec/core/c-backend.md`, `tasks/espada/ESP-008.md`). No Rust code changes.

## Intended result

Change, review, versioning, and deprecation process; unambiguous supported
versions; vulnerability response and support windows that promise only the
capacity actually assigned; maintainer rotation, release-key recovery, and
incident procedure; recorded tabletop exercises for a vulnerability patch and
for an incompatible language change.

## Handoff

- Result: MAT-029 closed as `HECHA` with evidence in
  `spec/MAT-029-validation.json` (`overall_pass: true`).
- Repository setting changed (maintainer-authorized): GitHub private
  vulnerability reporting enabled; it was off while `SECURITY.md` pointed to it.
- Files: `GOVERNANCE.md` and `operations/**` (new); `SECURITY.md`,
  `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `.github/CODEOWNERS`,
  `.github/PULL_REQUEST_TEMPLATE.md`, `tasks/madurez/MAT-029.md`; closeout edits
  to `README.md` (2 lines), `PLAN_MAESTRO_ARGORIXLANG.md`,
  `tasks/madurez/BACKLOG.json`, `WORKBOARD.md`.
- Tests: no code changed. Drills ran `cargo test --workspace --locked` on
  v1.0.1 (372/0), on a v1.0.2 drill patch (372/0), and on two version-bump
  mutations of `main` (26 and 56 expected failures). Both
  `spec/validate-*.ps1` validators pass.
- Findings for other lanes (no action taken here):
  - 26 tests pin the internal `ir_version` string although
    `spec/compatibility.md` calls IR internal; expect churn in Core IR work.
  - Bytecode version allow-lists span 24 match arms in
    `crates/argorix_bytecode/src/bytecode.rs`; unknown-version rejections
    cascade into long messages.
  - Tag v1.0.1 has `Cargo.toml` 1.0.0; the next release must fix it using
    `operations/maintenance.md` §1 (includes the `cargo update --workspace`
    step the drill uncovered).
- Open for the maintainer: confirm 2FA on `argorixlabs`; review and merge the
  PR.
- Next ready work for this lane: none dependency-ready outside ESP-008.
  Coordinate a file-level ESP-008 subtask with Codex, or wait for ESP-009.

---

# Previous claim: ESP-008.R (merged in PR #25)

## Claim

- Task: ESP-008.R — native C execution runner and dependency evidence
  (subtask of Codex's ESP-008; split accepted in `coordination/codex.md`,
  "Accepted file-level split", commit `2756e76` on `codex/c-backend-esp008`).
- State: DONE, merged in PR #25; its runner produced ESP-008's execution
  evidence on `main` and is replaced by the Rust harness in the current claim.
- Branch: `claude/core-c-runner`.
- Base: `3e620ab9` (`origin/main`).
- Started: 2026-09-19.
- Exclusive paths (new):
  - `conformance/core_c/**`
  - `.github/workflows/core-c.yml`
- Read-only inputs: `tests/selfhost/runtime/cases.json`,
  `bootstrap/c/toolchain.json`, `bootstrap/c/argorix_core_runtime.{c,h}`, and
  the `argorixc core-emit-c` command. New runtime cases stay with Codex.
- Not touched: `bootstrap/c/**`, `crates/argorix_ir/src/core_c.rs`,
  `crates/argorixc/src/main.rs`, `spec/core/c-backend.md`,
  `tasks/espada/ESP-008.md`, `tests/selfhost/runtime/**`.
- Integration order (from Codex's log): independent PR from `main`; Codex
  lands or rebases around it before ESP-008 closeout.
- Previous claim MAT-029: DONE on `claude/governance-mat029` (PR #24, pending
  merge). Whichever of the two PRs lands second rebases this file.

## Intended result

A runner that, for every declared runtime case, emits C with
`argorixc core-emit-c`, compiles it with an allow-listed compiler from
`bootstrap/c/toolchain.json` using an argument array (no shell), executes it,
compares stdout/stderr/exit status, inspects the Linux binary for Rust
libraries, Rust symbols, and process-spawning imports, and writes a JSON
report. Emission and execution can run on different hosts so execution can be
shown on a host with no Rust toolchain. A Linux CI job runs it with GCC.

## Handoff

- Commits: `ac2c28e` (claim), `f14d413` (runner, policy, tests, CI, evidence).
- Files: `conformance/core_c/{run.py,test_run.py,policy.json,README.md}`,
  `conformance/core_c/evidence/2026-09-19-wsl-gcc15-rust-free.json`,
  `.github/workflows/core-c.yml`; plus this log and the board row.
- Local evidence: C emitted on Windows by `argorixc` from
  `codex/c-backend-esp008@2756e76`, compiled and run with GCC 15.2.0 in WSL2
  with no rustc/cargo/rustup (`--require-rust-free-host`): 3/3 cases, 4/4
  negative controls, only `libc.so.6` NEEDED. The report's runner hash matches
  the committed `run.py`.
- CI evidence: temporary probe branch (this branch merged with
  `codex/c-backend-esp008@a104915`, push trigger added only there), run
  https://github.com/argorixlabs/argorixlang/actions/runs/35462468041 — all 5
  jobs green; 6/6 cases (including Codex's new array, bounds, and struct
  cases, picked up with no runner change) and 4/4 controls with GCC 13.3,
  Clang 18.1, and GCC 14.2 in `debian:stable-slim` without Rust. The probe
  branch was deleted afterwards; it was never a PR.
- Unit tests: 23 pass on Windows Python 3.13 and WSL Python 3.13.
- Manually checked failure paths: tampered bundle C file → case FAIL (hash
  mismatch); compiler `sh` → exit 2; `--require-rust-free-host` enforced.
- On `main` today the execution jobs are skipped with a notice, because
  neither `core-emit-c` nor `tests/selfhost/runtime/cases.json` exists there.
  They run automatically once ESP-008 lands; one without the other fails CI.
- Limits: Linux ELF only (no PE/Mach-O inspector for ESP-017); a static Rust
  object with stripped symbols and no toolchain paths would not be caught by
  inspection alone, only by the compile-argument and runtime-source checks.
- For Codex at ESP-008 closeout: the report is ESP-008's execution and
  dependency evidence; `core-c-report-*` artifacts from the post-merge `main`
  run are the ones to cite.
