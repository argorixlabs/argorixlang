# Contributing to ArgorixLang

Thanks for your interest in contributing! This document explains how to build,
test, and submit changes.

## Prerequisites

- A recent stable Rust toolchain (install via [rustup](https://rustup.rs/)).
- The repository is a Cargo workspace; all crates live under `crates/`.

## Building and testing

```sh
cargo build --workspace
cargo test --workspace
```

Before opening a PR, make sure the same checks CI runs pass locally:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo test -p argorix-conformance      # language conformance suite
```

If you change language behavior, update the conformance suite under
`conformance/`. If you change the bytecode format, regenerate the
`examples/*.argbc.json` artifacts.

## Commit and PR conventions

- We use [Conventional Commits](https://www.conventionalcommits.org/)
  (`feat:`, `fix:`, `docs:`, `chore:`, …) — see the git history for examples.
- Keep PRs focused; one logical change per PR.
- Fill out the PR template and check off the checklist.

## Contributor License Agreement (CLA)

We require all contributors to sign our [CLA](CLA.md). When you open your first
PR, the **CLA Assistant** bot will comment with instructions. Simply reply with:

> I have read the CLA Document and I hereby sign the CLA

Your signature is recorded once and applies to all future contributions.

## Review process

- All PRs require CI to be green and at least one approving review from a
  [code owner](.github/CODEOWNERS).
- Maintainers must approve workflow runs for first-time contributors before CI
  executes.

## Reporting security issues

Please **do not** open public issues for vulnerabilities. See [SECURITY.md](SECURITY.md).
