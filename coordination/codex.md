# Codex lane

## Current claim

- Task: ESP-009 — minimal standard library.
- State: IN PROGRESS.
- Branch: `codex/stdlib-esp009`.
- Base: `4b0705a7fc162ceb57eaf604890eb16b98fb4a0c`.
- Started: 2026-09-19.
- Exclusive paths:
  - `stdlib/**`
  - `spec/core/stdlib.md`
  - `tests/selfhost/stdlib/**`
  - `tasks/espada/ESP-009.md`
  - ESP-009 Buffer/Arena/host-ABI additions in `bootstrap/c/**` and the Core
    parser/semantics/IR/C-backend modules
- Shared closeout paths, edited only at task closure:
  - `PLAN_ESPADA_INDEPENDIENTE.md`
  - `PLAN_MAESTRO_ARGORIXLANG.md`
  - `tasks/madurez/BACKLOG.json`
  - `WORKBOARD.md`

## Accepted file-level split

Accepted on 2026-09-19: **ESP-008.R — native C execution runner and dependency evidence**.

- Owner: Claude.
- Claude-exclusive new paths:
  - `conformance/core_c/**`
  - `.github/workflows/core-c.yml`
- Read-only inputs for Claude:
  - `tests/selfhost/runtime/cases.json`
  - `bootstrap/c/toolchain.json`
- Claude scope: invoke `argorixc core-emit-c`, compile through an allow-listed
  compiler using an argument array without a shell, execute every declared
  case, compare stdout/stderr/exit status, inspect Linux dependencies for Rust
  artifacts or tool invocations, emit a JSON report, and run it in Linux GCC CI.
- Claude must not edit Codex-owned ESP-008 implementation/spec/task paths or
  add runtime cases. Codex will not edit Claude's two exclusive paths.
- Integration order: Claude opens an independent PR from current `main`.
  Codex reviews/lands or rebases around that PR before ESP-008 closeout; neither
  owner copies or overwrites the other's branch.

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
- 2026-09-19: completed fixed-array/bounds, struct, tagged-enum/match,
  bytes/UTF-8, and explicit step-limit execution cases. The manifest now
  contains ten Core fixtures with exact stdout, stderr, and exit-code oracles.
- Claude's ESP-008.R landed on `main` through PR #25 and was merged into this
  branch. Its gcc, clang, and Rust-free Debian execution jobs passed all ten
  cases, including ELF dependency/process-spawn inspection.
- Added the canonical 48-byte C1 handle representation and fail-closed runtime
  validation for arena identity/lifetime, slot, generation, type, range, and
  write permission. Native runtime self-tests cover a valid handle, stale
  generation (`USE_AFTER_FREE`), and released arena (`ARENA_RELEASED`).
- The C emitter still does not construct `Arena<T>` or `Buffer<T>` values; those
  collection APIs belong to ESP-009. ESP-008 proves the minimal C1 runtime
  checks and representative executable Core profile, not the full standard
  library, self-hosting, or the future native backend.
- 2026-09-19 closeout: all GitHub checks passed, including gcc, clang,
  Rust-free Debian, Linux beta/stable, macOS, Windows, DCO, conformance, and
  dependency scanning. Local formatting/Clippy passed and 418 workspace tests
  passed with zero failures. Versioned evidence is in
  `bootstrap/ESP-008-validation.json`; PR #23 is ready for review/merge.
