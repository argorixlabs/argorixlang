# Drill: incompatible language change (bytecode version bump)

- Date: 2026-09-19
- Task: MAT-029, AC-3 (every language change needs spec, compatibility, and
  tests)
- Run by: Claude (AI agent, Claude lane in `WORKBOARD.md`) on Windows 11
  x86-64
- Toolchain: `rustc 1.96.0`, `cargo 1.96.0`
- Starting point: `origin/main` at `3e620ab9`
- Rules under test: [GOVERNANCE.md — Changing the language](../../GOVERNANCE.md#changing-the-language)
  and [spec/compatibility.md](../../spec/compatibility.md) rules 2, 4, and 5

## Scenario

A contributor changes the version an artifact format carries, without a change
proposal, spec clause, or test update. The drill checks whether the repository
catches this on its own, and what a legitimate version of the change would
cost. Two variants were run in a throwaway worktree; nothing was committed.

## Variant A — internal IR version (`ir_version` 1.0 → 1.1)

One-line change at `crates/argorix_ir/src/ir.rs:986`.

- `cargo test --workspace --locked --no-fail-fast`: **385 passed, 26 failed**.
- All 26 failures are string-equality assertions (`left: "1.1"`,
  `right: "1.0"`) in `tests/compiler_tests.rs`, `crates/argorix_ir`, and
  `crates/argorix_module/tests/resolver.rs`.
- `argorixc emit-bytecode` still emits `bytecode_version: "1.0"`: the IR
  version does not reach bytecode. An unmodified VM runs the output.

Finding: `spec/compatibility.md` classifies IR as an internal identity that
"can change with the compiler", yet 26 tests pin it as if it were a public
format. The change is caught, but the tests treat an internal axis as stable,
which will add churn during ESP-007 onward. Recorded for the language owner; no
change made here.

## Variant B — public bytecode version (`bytecode_version` 1.0 → 1.1)

One-line change at `crates/argorix_bytecode/src/lower.rs:185`.

- `cargo test --workspace --locked --no-fail-fast`: **355 passed, 56 failed**
  (34 in `tests/compiler_tests.rs`, 15 in `tests/official_suite.rs`, 2 in
  `tests/pipeline.rs`, 1 in `tests/package_cli.rs`, 4 in unit tests).
- Causes: 24 version-string assertions, and verification failures
  `UnsupportedVersion("1.1")` / `unsupported bytecode version '1.1'` from
  `verify_bytecode`, including the conformance runner's `verify_bytecode`
  stage.
- `argorixc verify-bytecode examples/runtime_mvp_v100.argx`: exit 1.
- `argorixc emit-bytecode examples/runtime_mvp_v100.argx`: exit 1, **no output
  written**: the emitter verifies its own output and refuses to emit bytecode
  the verifier does not accept.

### Old reader, new artifact (compatibility rule 4)

The `v1.0.1` VM, unmodified, was given `examples/runtime_mvp_v100.argbc.json`
with only `bytecode_version` changed to `"1.1"`:

| Input | Result |
| --- | --- |
| Control: original `1.0` artifact | `Status: completed`, exit 0 |
| Same artifact marked `1.1` | `bytecode verification failed: unsupported bytecode version '1.1'; ...`, exit 1, nothing executed |

The old VM rejects the unknown version before execution, as rule 4 requires.

## Cost of doing it legitimately

A real bytecode version bump would have to change, besides the emitter:

- 24 version allow-list sites in non-test code of
  `crates/argorix_bytecode/src/bytecode.rs` (including
  `TYPED_PAYLOAD_VERSIONS`);
- 21 bytecode-version assertions on `"1.0"` in `tests/compiler_tests.rs`,
  plus assertions in the crates' own tests (`"1.0"` literals appear in 13
  files under `crates/`, some of them other formats' versions);
- the golden `.argbc.json` fixtures compared against fresh emission;
- `spec/compatibility.md` (accepted-version list), the spec clause, a
  migration note, and positive/negative fixtures — the parts no test forces.

## Findings

1. **The repository fails closed on unknown bytecode versions** at three
   points: the emitter, `verify-bytecode`, and an older VM. This is tested
   behavior, not policy text.
2. **Tests catch the code change but not the missing documentation.** Nothing
   fails if a contributor updates the allow-lists and tests but skips the spec
   clause, compatibility classification, and migration guide. That part is
   enforced only by the PR checklist added in this task.
3. **Version allow-lists are spread over 24 match arms.** Forgetting one arm
   gives a partial, confusing rejection. Worth consolidating when the bytecode
   crate is next touched (not in MAT-029 scope).
4. **Rejection messages cascade**: an unknown version lists every metadata
   requirement it fails (`provider contracts require 0.11 through 0.15; policy
   metadata requires 0.17; ...`) instead of stopping at "unsupported version".
   Correct but noisy.
5. **IR version is pinned by 26 tests** despite being internal (Variant A).

## Not covered

- Writing the migration guide and spec clause for a real version change (no
  real change was proposed).
- Linux and macOS runs.
