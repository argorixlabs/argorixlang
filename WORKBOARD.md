# ArgorixLang collaboration board

Last coordination update: 2026-09-19.

This file prevents Codex, Claude, and human contributors from modifying the
same work at the same time. It coordinates ownership; the plans and task files
remain the source of truth for scope and acceptance.

## Active lanes

| Owner | Task | Branch | State | Exclusive paths | Agent log |
| --- | --- | --- | --- | --- | --- |
| Codex | ESP-009 — minimal standard library | `codex/stdlib-esp009` | IN PROGRESS | `stdlib/**`, `spec/core/stdlib.md`, `tests/selfhost/stdlib/**`, `tasks/espada/ESP-009.md`; required Buffer/Arena/host-ABI plumbing coordinated through `bootstrap/c/**` and Core stage0 modules | [coordination/codex.md](coordination/codex.md) |
| Claude | ESP-008.R — native C execution runner and dependency evidence (split accepted by Codex) | `claude/core-c-runner` | DONE | `conformance/core_c/**`, `.github/workflows/core-c.yml` | [coordination/claude.md](coordination/claude.md) |

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
| ESP-009 | Claimed by Codex | ESP-008 is complete. |
| ESP-010 | No | Waits for ESP-009. |
| MAT-008 onward | No | Their declared implementation dependencies are not complete. |

Claude should coordinate a file-level subtask of ESP-009 with Codex first or
wait for the next dependency-ready claim; it must not silently start a
downstream task whose gate is still open.
