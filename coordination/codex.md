# Codex lane

## Current claim

- Task: ESP-008 — transitional C backend and minimal runtime.
- State: IN PROGRESS.
- Branch: `codex/c-backend-esp008`.
- Base: `13069ff02b3dcd6583e1fd75f1ab5ab2165e8d46`.
- Started: 2026-09-19.
- Exclusive paths:
  - `bootstrap/c/**`
  - `crates/argorix_ir/src/core_c.rs`
  - Core C CLI integration in `crates/argorixc/src/main.rs`
  - `spec/core/c-backend.md`
  - `tasks/espada/ESP-008.md`
- Shared closeout paths, edited only at task closure:
  - `PLAN_ESPADA_INDEPENDIENTE.md`
  - `PLAN_MAESTRO_ARGORIXLANG.md`
  - `tasks/madurez/BACKLOG.json`
  - `WORKBOARD.md`

## Intended result

Emit deterministic portable C from `VerifiedCoreIr`, provide the smallest
non-Rust runtime/ABI needed by the approved executable fixtures, compile with a
declared C toolchain without shell commands derived from source text, and prove
the resulting programs do not link a Rust runtime.

## Handoff

- 2026-09-19: defined the transitional C contract and C1 runtime ABI.
- Added checked integer helpers, deterministic trap output, and an explicit
  step budget under `bootstrap/c/`.
- Local WSL compiler: `cc (Debian 15.2.0-14) 15.2.0` with C11, pedantic
  warnings as errors.
- Runtime self-test observed `ARGORIX_RESULT:42`, `INTEGER_OVERFLOW`,
  `DIVISION_BY_ZERO`, and `STEP_LIMIT` traps. Windows `wsl.exe` maps the Linux
  trap exit status to its own process status, so native exit-code evidence will
  be collected inside the conformance runner.
- Initial next step was `CoreIrBackend` C emission with sequenced temporaries
  and executable Core fixtures; the following entry records that milestone.
- 2026-09-19: added the verified scalar C emitter and `argorixc core-emit-c`.
  Recursive calls, checked arithmetic, assignments, `while`, value `if`, and
  boolean short-circuiting lower through deterministic temporaries.
- Added three executable Core fixtures: normal result 42, unsigned overflow,
  and division by zero. Rust-side deterministic emission tests pass; C-enabled
  CI is responsible for compile/execute evidence on a clean native host.
- Remaining before closure: aggregates/enums/arrays/slices, bounds/handle
  checks, resource-profile plumbing, conformance report, dependency inspection,
  and full workspace regression.
