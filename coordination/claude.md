# Claude lane

## Current claim

- Task: MAT-029 — language governance, support, and maintenance.
- State: IN PROGRESS.
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

Pending implementation and evidence.
