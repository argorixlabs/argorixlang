# Drill: security patch release from v1.0.1

- Date: 2026-09-19
- Task: MAT-029, AC-4
- Run by: Claude (AI agent, Claude lane in `WORKBOARD.md`) on the maintainer's
  Windows 11 x86-64 workstation
- Toolchain: `rustc 1.96.0 (ac68faa20 2026-05-25)`, `cargo 1.96.0`
- Starting point: tag `v1.0.1` → commit `12e1fad7f12030fed8a5f7db3dac4ff476c81219`
- Procedure under test: [maintenance.md §1](../maintenance.md#1-patch-release-vxyz--vxyz1)

## Scenario

A vulnerability is accepted against the latest release, v1.0.1, and must ship
as v1.0.2 without the unreleased work on `main`. No real vulnerability was
used. The patch content was a real defect instead: v1.0.1 was tagged with
`Cargo.toml` at `1.0.0`, so its binaries report the wrong version.

## Steps and observed results

| Step | Command | Result |
| --- | --- | --- |
| Isolate from `main` | `git worktree add --detach <scratch> v1.0.1`, then `git switch -c drill/patch-v1.0.2` | Clean checkout of the tag |
| Tag still builds | `cargo test --workspace --locked` | **372 passed, 0 failed**, 43 s |
| Bump version | `Cargo.toml` 1.0.0 → 1.0.2 (2 lines), `CITATION.cff` 1.0.1 → 1.0.2 | 3 lines changed |
| Test with `--locked` | `cargo test --workspace --locked` | **Failed**: `cannot update the lock file ... because --locked was passed` (exit 101) |
| Update lockfile | `cargo update --workspace --offline` | 11 workspace crates 1.0.0 → 1.0.2; `Cargo.lock` diff 11/11 lines, no third-party change |
| Retest | `cargo test --workspace --locked` | **372 passed, 0 failed** |
| Version check | `argorixc --version`, `argorix-vm --version` | `argorixc 1.0.2`, `argorix-vm 1.0.2` |
| Commit and tag | `git commit --signoff`; `git tag -a drill-v1.0.2` | Commit `b692283d310871e418fdfa9bffaf463ba65e5b6d` |
| Source archive | `git archive --format=tar.gz --prefix=argorixlang-1.0.2/ drill-v1.0.2` | SHA-256 `1a89603648aa6221d580132f5cd333a29a2217f7c337c60cf548bca26f0c67ef` |
| Reproducibility | Same command a second time | Identical SHA-256 |
| Publish | `git push`, `gh release create`, publish advisory | **Not executed** — publishing is a maintainer decision |

After the drill the scratch worktree, the `drill/patch-v1.0.2` branch, and the
`drill-v1.0.2` tag were deleted locally. Nothing was pushed.

## Findings

1. **The lockfile step was missing from any written procedure.** A version bump
   breaks every `--locked` build until `cargo update --workspace --offline` is
   run. It is now step 4 of the patch procedure.
2. **v1.0.1 reports version 1.0.0.** Confirmed by the drill; the next real
   release must include the fix exercised here.
3. **The release tag still builds and passes with a current toolchain**
   (Rust 1.96.0, four months after the tag). A patch release is technically
   possible today.
4. **Publishing depends on one person.** Steps 8–9 cannot be delegated; see the
   single-maintainer gap in [maintenance.md §7](../maintenance.md#7-open-gaps).

## Not covered

- Developing a fix in a GitHub advisory private fork (no real advisory exists).
- Publishing the release and the advisory, and the forward-port PR to `main`.
- Building or testing on Linux or macOS; only Windows was used.
