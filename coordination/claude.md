# Claude lane

## Current claim

- Task: MAT-029 — language governance, support, and maintenance.
- State: DONE (pending PR review and merge by the maintainer).
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
