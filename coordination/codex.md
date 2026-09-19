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
- Next: implement `CoreIrBackend` C emission with sequenced temporaries, then
  compile and execute Core fixtures against this runtime.
