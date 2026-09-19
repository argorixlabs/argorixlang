# Argorix Lang governance

This document says who decides what in Argorix Lang, how the language changes,
and how versions are supported. It describes the project as it is operated
today, not as it may be staffed later. Operational procedures (patch releases,
incidents, maintainer rotation, account and key recovery) are in
[`operations/maintenance.md`](operations/maintenance.md).

## Current capacity

Argorix Lang has **one human maintainer**. There is no maintainer team, no
on-call rotation, and no funded support. Every role below is held by that
person unless the table names someone else.

| Role (from `spec/requirements.json`) | Holder |
| --- | --- |
| Responsable de lenguaje, compatibilidad, release, seguridad | Sole maintainer (GitHub `@argorixlabs`) |
| All other roles listed in `spec/requirements.json` | Sole maintainer, or **vacant** where the work has not started |
| Second maintainer with merge and release rights | **Vacant** |
| Independent security reviewer | **Vacant** (see MAT-026) |

AI coding agents (for example Codex and Claude) contribute through the lanes
recorded in [`WORKBOARD.md`](WORKBOARD.md). They are contributors, not
maintainers: they do not hold roles, cannot accept risk, and their pull
requests are merged only by a human maintainer.

Nothing in this repository should be read as a promise of response times,
staffing, or support beyond what this section states. Policies that need more
people than are listed here stay proposals until someone holds the role.

## Decisions

The maintainer decides. To keep decisions reviewable:

1. Decisions that change scope, requirements, or acceptance criteria are
   recorded in the relevant plan or task file (`PLAN_MAESTRO_ARGORIXLANG.md`,
   `PLAN_ESPADA_INDEPENDIENTE.md`, `tasks/**`), including what is being given
   up. Removing a requirement to make a task look closed is not allowed.
2. Everything else is decided in the pull request that makes the change.
3. Disagreements from contributors are raised in the pull request or issue. The
   maintainer answers with the reason for the decision.

When a second maintainer exists, this section will be replaced by a written
consensus rule. Until then, there is no quorum to claim.

## Changing the language

A change proposal is required before any change to the parser, semantic
checks, IR, bytecode, VM behavior, evidence formats, or host ABI that is
visible to programs or artifacts. The proposal is an issue or the description
of the pull request itself, and must state:

- the version axes affected (source, `argorix.toml`, bytecode, trace/report/
  bundle, host ABI, protocol profile), as listed in
  [`spec/compatibility.md`](spec/compatibility.md);
- the MAT-001 requirement(s) it serves (`spec/requirements.json`);
- the specification clause added or changed (`spec/language/**`,
  `spec/core/**`);
- at least one positive and one negative test or conformance fixture;
- whether it is additive or incompatible, and the migration guide if it is
  incompatible;
- security impact, including any change to effects, authorization, identity,
  or evidence.

A pull request that changes language behavior without all three of
**specification, compatibility classification, and tests** is not merged. The
PR template checklist enforces this at review time; it is not yet enforced by
CI.

Compatibility rules — what counts as additive, what needs a new incompatible
version, how readers must reject unknown artifacts — are normative in
[`spec/compatibility.md`](spec/compatibility.md) and are not repeated here.

## Versioning

- Releases are git tags `vMAJOR.MINOR.PATCH` with a GitHub release.
- The version in `Cargo.toml` (`[workspace.package]` and the root package)
  must equal the tag being released. Release **v1.0.1 did not meet this**:
  its `Cargo.toml` still says `1.0.0`. The next release must fix it.
- `CITATION.cff` and `.zenodo.json` are updated in the same release commit.
- `main` is a development branch. A version number in `Cargo.toml` on `main`
  does not mean that version has been released; only tags do.
- Work on `main` after v1.0.1 (the Argorix Core bootstrap, ESP-004 onward) is
  unreleased.

## Supported versions

| Version | Status | What it gets |
| --- | --- | --- |
| v1.0.1 (latest release) | Supported, best effort | Security fixes as v1.0.x patch releases |
| v1.0.0 | Not supported | Upgrade to v1.0.1 (metadata-only difference) |
| v0.x (v0.9.0 – v0.36.0) | Not supported | No fixes; historical and paper baselines only |
| `main` (unreleased) | Development | Fixes land here first; no advisories for unreleased code |

Only the most recent release line receives fixes. When a new minor or major
release is published, the previous line stops being supported on the same day
unless its release notes state otherwise. There is currently no long-term
support line.

## Deprecation

A deprecated form keeps working, with a diagnostic, for **two consecutive
stable releases** after the release that announced the deprecation, as set in
[`spec/compatibility.md`](spec/compatibility.md) rule 3. Deprecations are
listed in the release notes of the release that announces them, with the
replacement and a migration recipe or tool.

A security fix may shorten that period only if the release notes and the
advisory state the impact, the mitigation, and whether the change can be
reverted.

## Changing this document

Changes to this file go through a pull request like any other change and are
listed in the next release notes. A change that adds a promise (a response
time, a support window, a new supported platform) must name who holds it.
