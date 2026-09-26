# Claude lane

## Current claim

- Task: ESP-018 — the agent and policy language, migrated to Argorix, as
  subtasks A (lexer), B (parser and AST), C (checks and modules), D
  (lowering to IR and bytecode) and E (compatibility matrix).
- State: IN PROGRESS. A (the lexer) is in PR #64. B (the parser) is done on
  `claude/agent-parser-esp018b`, to open once #64 merges: every agent-language
  program of the repository, 1,342, and 81 samples parse to stage0's tree.
  C (checks and modules) is done on the same branch: every check of
  `checker.rs` (`compiler/agent_check.argx`), 1,462 programs, and the module
  graph (`compiler/agent_package.argx`), 79 packages, give stage0's dumps.
  D (lowering) is in progress: the IR of one file (`compiler/agent_ir.argx`)
  matches stage0 on all 557 programs that check.
- Started: 2026-09-26.
- Ficha: `tasks/espada/ESP-018.md`; dump `spec/language/tokens.md`.
- Paths: `compiler/agent_lexer.argx`, `compiler/unicode.argx` (generated),
  `tests/selfhost/agent/**`, `spec/language/tokens.md`, the ficha, the token
  dump in `crates/argorix_parser/src/lexer.rs` and its `argorixc` command.
- Parallel: ESP-017 (the Windows target) is in PR #63, awaiting review.

## Handoff (ESP-016)

- The native backend compiles Core and the whole compiler without C; the
  compiler builds itself natively with no C compiler (`bootstrap/native.py`).

---

# Previous claim: ESP-016 (DONE)

- Task: ESP-016 — the first native backend: x86-64 machine code for Linux in
  an ELF64 object, linked with a declared linker and a bounded runtime shim,
  and the compiler built natively without a C compiler.
- State: DONE. Merged in PR #61; CI green on `main@5e881ac`.
- Started: 2026-09-24.
- Ficha: `tasks/espada/ESP-016.md`; specification
  `spec/core/native-x86-64.md`.
- Paths: `compiler/native.argx`, `compiler/x86.argx` and `compiler/elf.argx`
  (new); the shared entry points and state of `compiler/c_emit.argx`; the
  `object` key and limits of `compiler/main.argx`; the native manifest of
  `compiler/pipeline.argx`; `argorix.build`; `bootstrap/native/**`,
  `bootstrap/native.py` and `bootstrap/container/native/**` (new);
  `tests/selfhost/native/**`; `crates/argorixc/tests/native.rs` and the shared
  corpus module; the native jobs of `.github/workflows/core-c.yml`.

## Handoff (ESP-015)

- Stage1 builds stage2 and stage2 builds stage3 with a C compiler and
  Python only; the three are byte-identical and pass the suites
  (`bootstrap/selfhost.py`).
- Next: ESP-016, the native backend.

---

# Previous claim: ESP-015 (DONE)

- Task: ESP-015 — stage2/stage3 bootstrap without Rust: stage1 builds
  stage2 and stage2 builds stage3 with fixed tools and flags, the suite runs
  with them, and edits to the compiler's sources show that no seed is copied.
- State: DONE. Merged in PR #60 as `main@31b82d1`.
- Started: 2026-09-24.
- Ficha: `tasks/espada/ESP-015.md`.
- Paths: `bootstrap/selfhost.py` and `bootstrap/container/Dockerfile` (new),
  the selfhost job of `.github/workflows/core-c.yml`, `spec/core/stage1.md`
  (stage2 and stage3), the ficha.

---

# Previous claim: ESP-014 (DONE)

- Task: ESP-014 — the first complete self-hosted compiler, as subtasks A
  (diagnostics as stage0 words and renders them), B (the driver and its build
  file), C (stage1 built by stage0 and run without Rust, with its provenance)
  and D (the C backend's refusal reasons).
- State: DONE. Merged in PR #59; CI green on `main@9054379`.
- Started: 2026-09-24.
- Ficha: `tasks/espada/ESP-014.md`.
- Paths: `compiler/main.argx` and `compiler/report.argx` (new), messages and
  spans in `compiler/check.argx`, `compiler/link.argx`, `compiler/parser.argx`,
  `compiler/ast.argx`, `compiler/diagnostics.argx` and `compiler/c_emit.argx`,
  `argorix.build`, `bootstrap/stage1.py`, `tests/selfhost/stage1/**`,
  `tests/selfhost/check/samples/messages.src`, `spec/core/stage1.md`,
  `spec/core/check.md`, `spec/core/c-backend.md`, the ficha,
  `crates/argorixc/tests/stage1.rs` and `link_differential.rs`, and the
  stage1 jobs and fixtures of `.github/workflows/core-c.yml`.

## Handoff (ESP-014)

- Stage1 (`compiler/main.argx`) compiles its own sources to stage0's C and
  writes stage0's diagnostics, byte for byte, including every reachable
  refusal of the C backend. CI builds and runs it on a host with no Rust.
- A build is described by `argorix.build` (`spec/core/stage1.md`), since
  the compiler-host profile has no command line.
- Next: ESP-015, stage2 and stage3 with binary identity and no Rust.

---

# Previous claim: ESP-013 (DONE)

- Task: ESP-013 — lowering and emission in Argorix, as subtasks A (IR and
  canonical JSON), B (verifier), C (C backend) and D (pipeline and manifest).
- State: DONE on 2026-09-24; A–D merged in PR #58. CI is green on
  `main@89d67d0`.
- Started: 2026-09-24.
- Ficha: `tasks/espada/ESP-013.md`.
- Paths: `compiler/ir.argx`, `compiler/ir_verify.argx`, `compiler/c_emit.argx`,
  `compiler/pipeline.argx` and `compiler/sha256.argx` (new), `tests/selfhost/pipeline/**`,
  `compiler/link.argx`, `tests/selfhost/ir/**`, `tests/selfhost/c/**`,
  `spec/core/c-backend.md`, `crates/argorix_ir/src/core_c.rs` (package C dump),
  `spec/core/ir.md`, the ficha, `crates/argorix_ir/src/core.rs` (package IR
  dump), `crates/argorix_semantics/src/core_check_dump.rs` (package loading),
  the `argorixc` package dumps, `.github/workflows/core-c.yml` (IR fixtures).

## Handoff (ESP-013)

- The whole path from Core source to C runs in Argorix, byte-identical to
  stage0 on every package of the differential, with a fixed point of the C
  backend on itself.
- Found: stage0 cannot read back the IR of a program nested past serde_json's
  recursion limit (`deep_unary_ok.src`); recorded in `spec/core/ir.md`.
- The pre-existing `generate.rs` compile error was specific to rustc 1.94.1;
  rustc 1.98 (CI's stable) builds the whole workspace.
- Next: ESP-014, the driver and stage1.

---

# Previous claim: ESP-012 (DONE)

- Task: ESP-012 — resolution, types and modules in Argorix, as subtasks A
  (types and names), B (ownership and views) and C (module linker).
- State: DONE on 2026-09-24; A, B and C merged in PRs #55, #56 and #57. CI is
  green on `main@def7a46`.
- Started: 2026-09-23.
- Ficha: `tasks/espada/ESP-012.md`.
- Paths: `compiler/check.argx`, `compiler/link.argx` (new),
  `tests/selfhost/check/**`, `spec/core/check.md`, `spec/core/modules.md`
  (the import rules as implemented), the ficha, and the stage0 checker and
  linker where the port needs them (`check_core_package`, the package dump).

## Handoff (ESP-012.C)

- The Argorix linker (`compiler/link.argx`) and a module-aware checker. The
  differential agrees with stage0 on 170 packages, including the compiler
  linked with itself.
- The stage0 driver now links and checks through
  `argorix_semantics::check_core_package`, and `argorixc
  core-check-package-dump` prints a package's diagnostics.
- Open: stage0's `a__b__name` renaming can make a root item collide with a
  dependency's item. The Argorix checker does not reproduce this; it is
  recorded in `spec/core/check.md`.
- Found, not fixed: `crates/argorix_core_c/src/generate.rs` (`Rng::pick` at
  lines 1923 and 3164) does not compile with rustc 1.94.1. CI's stable
  toolchain builds it. It is outside this lane.
- Next: ESP-013 (lowering and emission in Argorix) once ESP-012 is merged with
  CI green.

---

# Previous claim: ESP-011 (DONE)

- Task: ESP-011 — parser and AST in Argorix.
- State: DONE on 2026-09-23, merged in PR #54; CI green on `main@21b80ca`.
- Branch: `claude/parser-esp011`, base `6f981db` (`origin/main`).
- Started: 2026-09-23.
- Ficha: `tasks/espada/ESP-011.md`.
- Paths: `compiler/**`, `tests/selfhost/parser/**`, `spec/core/ast.md`, the
  ficha, the stage0 Core parser where the port and the spec disagree, and the
  `argorixc` AST dump.

---

# Previous claim: ESP-010 (DONE)

- Task: ESP-010 — lexer and diagnostics in Argorix.
- State: DONE on 2026-09-23, merged in PR #53; CI green on `main@6f981db`.
- Branch: `claude/lexer-esp010`, base `1183b90` (`origin/main`).
- Started: 2026-09-23.
- Ficha: `tasks/espada/ESP-010.md`.
- Paths:
  - `compiler/**`, `tests/selfhost/lexer/**`, `spec/core/tokens.md` and
    the ficha;
  - the stage0 Core lexer where it disagreed with the port (a string ending
    in a backslash at the end of the input);
  - the `argorixc` token dump and `--modules`;
  - the C backend for string literals and `wrapping_add`.

---

# Previous claim: ESP-009 (DONE)

- Task: ESP-009 — minimal standard library, the `.argx` modules of
  `spec/core/stdlib.md`.
- State: DONE on 2026-09-23. Landed in PRs #47, #48 and #51 (#49 and #50 were
  merged into their stacked bases and re-landed as #51). CI is green on
  `main@1183b90`. Follow-up: #52.
- Branch: `claude/stdlib-modules`, base `e6b6056` (`origin/main`, after
  PR #47 on `claude/stdlib-core`).
- Started: 2026-09-22.
- Ownership: reassigned from the Codex lane by the maintainer on 2026-09-22
  (“toma todo tu”). `codex/stdlib-esp009` last moved on 2026-09-20 and
  carries no `stdlib/*.argx`; its merged work (the S1 API and the
  Buffer/Arena verticals, PR #26) is kept as is.
- Paths: `stdlib/**`, `spec/core/stdlib.md`, `tests/selfhost/stdlib/**`,
  `tasks/espada/ESP-009.md`, and the Core frontend, IR, C backend and C1
  runtime where a stdlib module needs a primitive.
- Progress (2026-09-22):
  - PR #47 (ownership) is merged.
  - PR #48: the stdlib modules.
  - PR #49: arena scopes, the tokenizer example and measurements.
  - PR #50: the compiler-host boundary.
  - The PRs are stacked in that order. Every acceptance criterion in
    `tasks/espada/ESP-009.md` is met on the stack except CI on `main`.
- Open after merge: Windows has no compiler-host shim yet. JSON duplicate
  detection and `ordered_map` insertion are quadratic
  (`bootstrap/ESP-009-measurements.json`).

---

# Previous claim: ESP-009.F (PR #46)

## Claim

- Task: ESP-009.F — backend gaps g17–g23 (issue #39) and cross-module
  symbol resolution (issue #45). Subtask of ESP-009 (§10 of the master plan).
- State: DONE (pending review).
- Branch: `claude/esp009f-unblock`, base `59f70b0`.
- Started: 2026-09-22.
- Ficha: `tasks/espada/ESP-009.F.md`.
- Paths: the Core frontend, IR and C backend crates, the C1 runtime,
  `conformance/core_c/**`, `.github/workflows/core-c.yml`, the ficha.
- Overlap: `core_c.rs` and `bootstrap/c/**` are also listed in the ESP-009
  row, as they were for ESP-009.B. No `stdlib/**`, `spec/core/stdlib.md`,
  `tests/selfhost/**` or `tasks/espada/ESP-009.md` file is touched.
- Out of scope: issue #38, which needs a language decision.

## Handoff

- Issue #39: `loop` as a statement and as a value with `break value`,
  `match` on bools and integers with literal, binding and wildcard patterns
  and guards, module constants, and `x = x;`. g17-g23 moved to
  `regression/` with eight adversarial cases; the gap corpus is empty.
- Issue #45: a new linker (`argorix_semantics::core_link`) merges the module
  graph; `b.f(..)` and `b.C` reach public items, public types are named by
  bare name, and private, unknown, missing, duplicate and cyclic imports are
  resolution errors rendered against their own file. Two multi-module
  programs run through the C backend.
- The differential generator now produces the constructs of #39 and found a
  defect in the new match lowering, fixed and kept as `f08`.
- Evidence: regression 33/33 on GCC, clang and the sanitizers; 500
  generated programs; 526 workspace tests.
- Open: the bare-name visibility of imported public types is a language
  decision to write into `spec/core/modules.md`; issue #38 is unchanged.

---

# Previous claim: ESP-009.E (PR #42)

## Claim

- Task: ESP-009.E — repeated execution must be byte-identical, which
  `spec/core/stdlib.md` requires and nothing checked. Subtask of ESP-009
  under the master plan's subdivision rule (§10).
- State: DONE (pending review).
- Branch: `claude/harness-determinism`, stacked on `claude/differential-text`
  (PR #41).
- Started: 2026-09-22.
- Ficha: `tasks/espada/ESP-009.E.md`.
- Exclusive paths: `crates/argorix_core_c/**`, `conformance/core_c/**`,
  `.github/workflows/core-c.yml`, `tasks/espada/ESP-009.E.md`.

## Handoff

- `--repeat N` runs each executable N times and fails the case if any run
  differs from the first in stdout, stderr or exit status. CI uses 3 on both
  compilers, which is the sentence from the spec word for word.
- A fifth negative control, `repeat_detects_nondeterminism`, proves the check
  can fail: a sensor that prints its own process id. It is the only sensor
  that is executed rather than only compiled.
- Evidence: runtime cases 15/15 three times each with GCC 12.2 and clang
  14.0.6, a generated corpus 60/60 twice each, and the control detected every
  time.
- Limits: three runs catch an unstable output, they do not prove stability;
  nothing here compares hosts or compiler versions, only runs of the same
  binary.

---


# Previous claim: ESP-009.D (PR #41)

## Claim

- Task: ESP-009.D — differential coverage of the parts of ESP-009 that
  already execute: bytes/UTF-8 and the resource ceilings of `Buffer` and
  `Arena`. Subtask of ESP-009 under the master plan's subdivision rule
  (§10).
- State: DONE (pending review).
- Branch: `claude/differential-text`, stacked on `claude/differential-bitwise`
  (PR #40). It retargets to `main` once that lands.
- Base: `bcac7e3`.
- Started: 2026-09-22.
- Ficha: `tasks/espada/ESP-009.D.md`.
- Exclusive paths: `crates/argorix_core_c/**`, `conformance/core_c/**`,
  `.github/workflows/core-c.yml`, `tasks/espada/ESP-009.D.md`.
- Not touched: `stdlib/**`, `spec/**`, `tests/selfhost/**`,
  `crates/argorix_ir/**`, `bootstrap/c/**` — every Codex ESP-009 path.

## Intended result

The generator produces byte arrays decoded as UTF-8, aimed at the boundaries
a validator gets wrong, and programs that reach the declared ceilings of
`Buffer` and `Arena`. The oracle validates UTF-8 from Table 3-7 of the Unicode
Standard and models the ceilings from `spec/core/stdlib.md`, never from the
runtime.

## Handoff

- **Bytes and UTF-8 are generated**, from random code points across all four
  widths and from a table of the ends a validator gets wrong, with a planted
  mistake on top part of the time. 38 invalid sequences reached the decode
  and every one trapped `UTF8_INVALID` where the oracle said it would. The
  oracle's validator comes from Table 3-7 of the Unicode Standard and has its
  own tests.
- **The resource ceilings are generated**: a `Buffer<u64>` filled past the
  profile's byte ceiling and an arena allocated past its 1024 slots, 41
  `RESOURCE_LIMIT` traps in all. A unit test pins the exact push the buffer
  refuses and the exact slot the arena refuses.
- **960 programs, zero divergences**: 720 with GCC 12.2 (seeds 301–306 ×
  120), 160 with clang 14.0.6, 80 under AddressSanitizer and UBSan. Runtime
  cases 15/15, regression 16/16, gap corpus unchanged, workspace 515 tests.
- **A defect of my own, found and fixed here:** a loop body was not a scope in
  the oracle, so a `let` inside it piled up a binding per iteration and
  lookups past it went linear; with the loop budget raised to 300,000 for the
  buffer fill, generating seed 312 stopped finishing. It now marks and
  restores per iteration, which also fixes a shadowing semantics no generated
  program had reached.
- Limits: the arena's byte ceiling is modelled but unreachable, since its slot
  limit binds first; the text vertical only fits a `u64` program; the oracle
  charges loops and depth but not calls, which is safe while the generated
  call graph stays small.
- For Codex: nothing here touches an ESP-009 path. When the stdlib grows maps,
  paths, the file boundary and the JSON subset, this harness is where their
  adversarial cases belong — duplicate keys, canonical order, traversal and
  oversized input.

---

# Previous claim: ESP-009.C (PR #40)

## Claim

- Task: ESP-009.C — differential coverage for the constructs ESP-009.B
  unlocked: shifts, signed negation, `if` as a statement, scoped blocks and
  nested aggregates. Subtask of ESP-009 under the master plan's subdivision
  rule (§10): it cuts none of the parent's criteria, it raises the evidence
  behind them.
- State: DONE (pending review).
- Branch: `claude/differential-bitwise`.
- Base: `9c77061` (`origin/main`, "Close the backend defects that blocked
  writing Core (ESP-009.B) (#37)").
- Started: 2026-09-22.
- Ficha: `tasks/espada/ESP-009.C.md`.
- Exclusive paths: `crates/argorix_core_c/**`, `conformance/core_c/**`,
  `.github/workflows/core-c.yml`, `tasks/espada/ESP-009.C.md`.
- Not touched: `stdlib/**`, `spec/core/stdlib.md`, `tests/selfhost/**`,
  `tasks/espada/ESP-009.md`, `bootstrap/c/**`, `crates/argorix_ir/**` — every
  Codex ESP-009 path and every path ESP-009.B already closed. If the generator
  finds a backend defect, it is recorded in `gaps/gaps.json` and reported as an
  issue, not fixed here.

## Intended result

The differential generator emits the constructs that were gaps until #37, so
they are checked against the spec oracle on every run instead of resting on
the 16 fixed regression fixtures. Concretely: `<<` and `>>` with in-range and
out-of-range amounts (`SHIFT_OUT_OF_RANGE`), signed negation including `MIN`
(`INTEGER_OVERFLOW`), `if` used as a statement, scoped blocks, and nested
arrays and structs. The oracle derives each expectation from
`spec/core/evaluation.md` and `spec/core/types.md`, never from the backend.

## Handoff

- **Every shape the generator used to avoid is now generated**, and its tests
  changed from prohibitions to coverage: shifts, signed negation, `if` as a
  statement, `else if` chains, scoped blocks that shadow, nested arrays,
  structs holding structs, arrays of structs, `if` inside comparison operands
  and aggregate literals, `return`, `continue`, `break`, one nested loop and a
  self-recursive function.
- The oracle gained what those need: real scoping (a `let` shadows instead of
  overwriting; a block or branch drops exactly what it declared), the control
  flow of `return`/`continue`/`break`, the shift and negation rules of
  `spec/core/evaluation.md`, two-level aggregates, and a call-depth budget of
  its own, because it evaluates on the host stack.
- **1200 generated programs, zero divergences**: 960 with GCC 12.2 (seeds
  101–108 × 120), 160 with clang 14.0.6 (seeds 6 and 201 × 80), 80 under
  AddressSanitizer and UBSan. 357 of them return a value and 843 trap, across
  `INTEGER_OVERFLOW`, `INDEX_OUT_OF_BOUNDS`, `SHIFT_OUT_OF_RANGE`,
  `DIVISION_BY_ZERO` and `ARENA_RELEASED`.
- No regression: runtime cases 15/15 with both compilers and 4/4 negative
  controls each, regression corpus 16/16, workspace 507 tests, formatting and
  Clippy clean.
- The same seed produces the same corpus on Windows and Linux: seed 7 with 20
  programs hashes to `3000d6a2…` on both.
- **Two findings, reported rather than worked around.** Issue #38: an `if`
  used as a statement followed by an expression starting with `(` is parsed as
  a call of the `if`, so `if c { .. } (x)` is rejected while `if c { .. } x`
  is accepted; the grammar is ambiguous about `;`-less block statements and
  the maintainers have to pick a reading. Issue #39: seven programs
  `core-check` accepts and the backend refuses, recorded as g17–g23.
- g23 (`x = x;`, a C self-assignment clang rejects under `-Werror` and GCC
  does not diagnose) made the gap corpus compiler-aware: a gap can name its
  compilers and is reported `SKIPPED` elsewhere, instead of being announced
  fixed on every GCC run.
- CI: the differential job runs four GCC seeds of 80, one clang seed and one
  sanitized seed; the gap job runs both compilers.
- For Codex: nothing here touches `stdlib/**`, `spec/**`,
  `tests/selfhost/**`, `crates/argorix_ir/**` or `bootstrap/c/**`. If the
  stdlib wants `loop`, `match` on a bool or an integer, guards or module
  constants, issue #39 has a minimal repro for each.
- Limits: the oracle stops at 200 call frames and skips deeper programs; the
  shift reading (bits leaving the width are dropped, not an overflow) is a
  documented decision, since the spec states only the amount rule; `bytes`,
  `string` and `Slice` are not generated.

---

# Previous claim: ESP-009.C predecessor — ESP-009.B (merged in PR #37)

## Claim

- Task: ESP-009.B — defects of the transitional C backend that blocked writing
  Core. Subtask of ESP-009, under the master plan's subdivision rule (§10):
  it cuts none of the parent's criteria, it enables them.
- State: DONE, merged in PR #37.
- Branch: `claude/esp009b-backend-defects`.
- Base: `06b489b` (`origin/main`).
- Started: 2026-09-20.
- Ficha: `tasks/espada/ESP-009.B.md`, filled in with the mandatory template.
- Exclusive paths: `crates/argorix_ir/src/core_c.rs`,
  `crates/argorix_ir/tests/core_c.rs`, `bootstrap/c/argorix_core_runtime.{c,h}`,
  `conformance/core_c/**`, `.github/workflows/core-c.yml`,
  `tasks/espada/ESP-009.B.md`.
- **Overlap with the Codex ESP-009 lane:** `core_c.rs` and `bootstrap/c/**` are
  inside their claim. These are the defects that block ESP-009's own criterion
  that the algorithms live in `.argx`, and the maintainer asked for the plan to
  advance. No stdlib, spec, runtime-case or ESP-009 ficha file was touched, and
  one Codex test was updated only because field names are now prefixed.

## Intended result

Close every defect of issue #27, so a real Core program can be written: `if` as
a statement, scoped blocks, nested types, shifts, signed negation, and a typed
trap instead of a native stack overflow.

## Handoff

- **16 of 16 gaps fixed**, including the two silent wrong results (233 → 1001
  and 4 → 3) and the SIGSEGV, now `ARGORIX_TRAP:CALL_DEPTH_LIMIT`.
- The corpus moved from `gaps/` to `regression/`: **16/16 pass**, and also
  under AddressSanitizer and UBSan. A CI job runs it on every change, and
  `gaps.json` stays in place, empty, for the next defect.
- No regression: runtime cases **15/15**, differential **120/120** and
  **150/150**, workspace **490 tests**, Clippy and formatting clean.
- While closing g01 my own recorded expectation turned out to be miscomputed
  (42 where the program yields 37). The corpus flagged it as `CHANGED` and the
  record was corrected: the backend was right, my note was not.
- For Codex: the backend now emits `if` statements, scoped blocks, nested
  arrays and structs, checked shifts and negation, and charges a call-depth
  budget. `struct` fields are emitted as `argorix_f_<name>`.
- Remaining for other lanes: the depth limit (50000) is tuned to the 8 MiB
  Linux stack and belongs per profile in MAT-011/MAT-024; arenas are still
  released only explicitly; the differential generator does not emit shifts or
  negation yet, so those stay covered by the regression corpus.

---

# Previous claim: ESP-008.R harness and corpora (merged in #30, #32, #35)


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
