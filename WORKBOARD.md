# ArgorixLang collaboration board

Last coordination update: 2026-09-22.

This file prevents Codex, Claude, and human contributors from modifying the
same work at the same time. It coordinates ownership; the plans and task files
remain the source of truth for scope and acceptance.

## Active lanes

| Owner | Task | Branch | State | Exclusive paths | Agent log |
| --- | --- | --- | --- | --- | --- |
| Claude | ESP-012 — resolution, types and modules in Argorix (subtasks A, B, C) | `claude/check-esp012a` | IN PROGRESS (A done) | `compiler/check.argx`, `tests/selfhost/check/**`, `spec/core/check.md`, `tasks/espada/ESP-012.md`, the stage0 checker where the port finds a defect, the `argorixc` check dump | [coordination/claude.md](coordination/claude.md) |
| Claude | ESP-011 — parser and AST in Argorix | `claude/parser-esp011` | DONE (PR #54; CI green on `main@21b80ca`) | `compiler/**`, `tests/selfhost/parser/**`, `spec/core/ast.md`, `tasks/espada/ESP-011.md`, the Core parser in `crates/argorix_parser/src/core.rs` where the spec and the port disagree, the `argorixc` AST dump | [coordination/claude.md](coordination/claude.md) |
| Claude | ESP-010 — lexer and diagnostics in Argorix | `claude/lexer-esp010` | DONE (PR #53; CI green on `main@6f981db`) | `compiler/**`, `tests/selfhost/lexer/**`, `spec/core/tokens.md`, `tasks/espada/ESP-010.md`, the Core lexer in `crates/argorix_parser/src/core.rs` where the spec and the port disagree, the `argorixc` token dump | [coordination/claude.md](coordination/claude.md) |
| Claude | ESP-009 — minimal standard library (stdlib in `.argx`) | `claude/stdlib-*` | DONE (PRs #47, #48, #51; CI green on `main@1183b90`) | `stdlib/**`, `spec/core/stdlib.md`, `tests/selfhost/stdlib/**`, `tasks/espada/ESP-009.md` | [coordination/claude.md](coordination/claude.md) |
| Claude | ESP-009.F — backend gaps g17–g23 and cross-module imports (issues #39, #45) | `claude/esp009f-unblock` | DONE | Core frontend/IR/C backend, C1 runtime, `conformance/core_c/**`, `tasks/espada/ESP-009.F.md` | [coordination/claude.md](coordination/claude.md) |
| Claude | ESP-009.E — repeated execution must be byte-identical (subtask of ESP-009) | `claude/harness-determinism` | DONE | `crates/argorix_core_c/**`, `conformance/core_c/**`, `.github/workflows/core-c.yml`, `tasks/espada/ESP-009.E.md` | [coordination/claude.md](coordination/claude.md) |
| Claude | ESP-009.D — differential coverage of text and the resource ceilings (subtask of ESP-009) | `claude/differential-text` | DONE (PR #41) | `crates/argorix_core_c/**`, `conformance/core_c/**`, `.github/workflows/core-c.yml`, `tasks/espada/ESP-009.D.md` | [coordination/claude.md](coordination/claude.md) |
| Claude | ESP-009.C — differential coverage for the constructs ESP-009.B unlocked (subtask of ESP-009) | `claude/differential-bitwise` | DONE (PR #40) | `crates/argorix_core_c/**`, `conformance/core_c/**`, `.github/workflows/core-c.yml`, `tasks/espada/ESP-009.C.md` | [coordination/claude.md](coordination/claude.md) |
| Claude | ESP-009.B — backend defects that blocked writing Core (subtask of ESP-009) | `claude/esp009b-backend-defects` | DONE (PR #37) | `crates/argorix_ir/src/core_c.rs`, `crates/argorix_ir/tests/core_c.rs`, `bootstrap/c/argorix_core_runtime.{c,h}`, `conformance/core_c/**`, `.github/workflows/core-c.yml`, `tasks/espada/ESP-009.B.md` | [coordination/claude.md](coordination/claude.md) |
| Codex | ESP-009 — minimal standard library | `codex/stdlib-esp009` | REASSIGNED to Claude (2026-09-22) | `stdlib/**`, `spec/core/stdlib.md`, `tests/selfhost/stdlib/**`, `tasks/espada/ESP-009.md`; required Buffer/Arena/host-ABI plumbing coordinated through `bootstrap/c/**` and Core stage0 modules | [coordination/codex.md](coordination/codex.md) |
| Claude | MAT-029 — governance, support, and maintenance | `claude/governance-mat029` | DONE (PR #24) | `GOVERNANCE.md`, `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `operations/**`, `tasks/madurez/MAT-029.md`, `spec/MAT-029-validation.json`, `.github/CODEOWNERS`, `.github/PULL_REQUEST_TEMPLATE.md` | [coordination/claude.md](coordination/claude.md) |
| Codex | ESP-008 — transitional C backend and minimal runtime | `codex/c-backend-esp008` | DONE | see `tasks/espada/ESP-008.md`; open defects in issue #27 | [coordination/codex.md](coordination/codex.md) |
| Claude | ESP-008.R — native C execution runner and dependency evidence | `claude/core-c-runner` | DONE | `conformance/core_c/**`, `.github/workflows/core-c.yml` | [coordination/claude.md](coordination/claude.md) |

Shared planning files such as `PLAN_*.md`, `tasks/madurez/BACKLOG.json`, and
this board are updated only when claiming or closing work. They are not owned
for the entire implementation.

## Coordination protocol

1. Start from current `origin/main` and create a unique branch. Never work
   directly on another owner's branch.
2. Read this board and both agent logs before claiming work.
3. Claim only a dependency-ready task. Record the task, branch, exact paths,
   base commit, and start time in the owner's log before implementation.
4. Do not edit another active lane's exclusive paths. If a shared file is
   indispensable, record the intended edit in the log first and keep it
   minimal.
5. Commit only owned changes. Rebase or merge current `main` before opening a
   PR, resolve only conflicts within owned paths, and never discard another
   contributor's changes.
6. PR titles and bodies are English. A task closes only after its acceptance
   evidence and CI pass; planning work is not implementation evidence.
7. At handoff, list commits, tests, evidence, limitations, and remaining work
   in the owner's log. Change the central row only to `DONE`, `BLOCKED`, or a
   new claim.

## Collision rule

If two branches already changed the same implementation file, the later claim
stops. The owners agree on a file-level split or one branch lands first; the
second then rebases and adapts. Do not solve a collision by overwriting,
reverting, or cherry-picking an entire foreign branch.

## Ready queue

| Task | Ready now | Notes |
| --- | --- | --- |
| ESP-008 | Complete | Evidence in `bootstrap/ESP-008-validation.json`. |
| ESP-009 | Claimed by Claude | Reassigned by the maintainer on 2026-09-22; `codex/stdlib-esp009` last moved on 2026-09-20. |
| ESP-010 | No | Waits for ESP-009. |
| MAT-029 | Done by Claude | Closed with drills; see `spec/MAT-029-validation.json`. |
| MAT-008 to MAT-028, MAT-030 | No | Their declared implementation dependencies are not complete. |

A lane whose row says DONE is finished work kept for history; the rows above it
are the active claims. Defects found in a closed lane are tracked as issues, not
by reopening the row.

Claude should coordinate a file-level subtask of ESP-009 with Codex first or
wait for the next dependency-ready claim; it must not silently start a
downstream task whose gate is still open.
