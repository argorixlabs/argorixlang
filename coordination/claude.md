# Claude lane

## Current claim

- Task: ESP-008.R follow-up — differential testing against a spec oracle
  (continues the gap corpus of PR #28, same lane paths).
- State: DONE (pending review).
- Branch: `claude/core-c-differential`, stacked on `claude/core-c-gaps` (PR #28).
- Base: `91eff90`.
- Started: 2026-09-19.
- Exclusive paths: `conformance/core_c/**`, `.github/workflows/core-c.yml`.
- Not touched: any Codex ESP-008/ESP-009 path.

## Intended result

Random Core programs whose expected result comes from an evaluator written
against `spec/core/evaluation.md`, never from `argorixc` or the C backend, run
through the existing emit/compile/execute pipeline. The generator avoids every
shape recorded in `gaps/`, so a failure means a real divergence.

## Handoff

- Result: **900 generated programs (seeds 42, 7, 13; 296 expected traps) all
  match the oracle** with GCC 15.2 in WSL, emitted by `argorixc` built from
  `main@4b0705a`. No divergence in arithmetic, traps, evaluation order,
  short-circuiting, loops, or calls.
- New defect found by the generator and recorded as gap `g16`: an arithmetic
  expression containing an `if`, used as a comparison operand, is rejected with
  `CBackendUnsupported: checked arithmetic requires an integer`. Minimal repro:
  `if ((if c { 5u32 } else { 1u32 }) + 1u32) > 3u32 { 42u32 } else { 0u32 }`,
  which `core-check` accepts. Same root cause as `g09`. Reported on issue #27.
- 57 unit tests pass (19 new for the oracle: trap rules, truncating division,
  left-to-right order, short-circuiting, loop fuel, corpus determinism, and a
  check that generated programs avoid the known-gap shapes).
- CI: new `differential` job runs three seeds and uploads the generated
  programs with the report, so a failure is reproducible from the artifact.
- Limit: the generator covers scalars only. Arrays, structs, enums, buffers,
  arenas, bytes, and UTF-8 are not generated yet; they are the obvious next
  extension once the aggregate gaps in issue #27 are fixed.

---

# Previous claim: ESP-008.R (merged in PR #25)

## Claim

- Task: ESP-008.R — native C execution runner and dependency evidence
  (subtask of Codex's ESP-008; split accepted in `coordination/codex.md`,
  "Accepted file-level split", commit `2756e76` on `codex/c-backend-esp008`).
- State: DONE, merged in PR #25; the runner now produces ESP-008's execution evidence on `main`.
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
