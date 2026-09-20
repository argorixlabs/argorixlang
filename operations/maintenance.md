# Maintenance and incident procedures

Operational companion to [GOVERNANCE.md](../GOVERNANCE.md) and
[SECURITY.md](../SECURITY.md). Every procedure here is written for the capacity
that exists today: **one maintainer**. Steps marked *exercised* were run in the
MAT-029 drills recorded in [`operations/exercises/`](exercises/); the rest are
written procedure only.

## 1. Patch release (vX.Y.Z → vX.Y.Z+1)

Used for security fixes and urgent corrections to the latest release. Exercised
on 2026-09-19 from `v1.0.1`; see
[exercises/2026-09-19-patch-release.md](exercises/2026-09-19-patch-release.md).

1. **Branch from the release tag**, not from `main`:
   `git switch -c release/vX.Y.Z+1 vX.Y.Z`. `main` carries unreleased work.
   *(exercised)*
2. **Check the tag still builds** before changing anything:
   `cargo test --workspace --locked`. If it fails for reasons unrelated to the
   fix (toolchain drift), record that before continuing. *(exercised)*
3. **Apply the fix.** For a security fix developed in a GitHub private fork
   (created from the advisory), merge that fork's branch here. Add a
   regression test that fails without the fix.
4. **Bump the version** in `Cargo.toml` (both `version` lines), `CITATION.cff`,
   and `.zenodo.json` if it carries a version. Then run
   `cargo update --workspace --offline`: `Cargo.lock` records workspace crate
   versions, and `--locked` builds fail until it is updated. Check that the
   `Cargo.lock` diff touches only workspace crates. *(exercised)*
5. **Test**: `cargo test --workspace --locked`, and check that
   `argorixc --version` and `argorix-vm --version` print the new version.
   *(exercised)*
6. **Commit** with `--signoff`, then create an annotated tag `vX.Y.Z+1`.
   *(exercised under a drill tag name)*
7. **Archive the source** and record its hash:
   `git archive --format=tar.gz --prefix=argorixlang-X.Y.Z+1/ -o argorixlang-X.Y.Z+1.tar.gz vX.Y.Z+1`,
   then `sha256sum`. `git archive` output is reproducible for a given tag.
   *(exercised)*
8. **Publish**: push the branch and tag, create the GitHub release with the
   archive and its SHA-256, and forward-port the fix to `main` with a normal
   PR. *(not exercised; publishing needs the maintainer)*
9. **Advisory**: publish the GitHub security advisory, naming the fixed
   version and crediting the reporter. *(not exercised)*
10. **Update the supported-versions table** in GOVERNANCE.md and SECURITY.md.

Releases are currently **unsigned**: tags are annotated but not GPG/SSH signed,
and no binaries are published. Signing and provenance belong to MAT-023; do not
describe a release as signed until that work lands.

## 2. Vulnerability handling

1. A report arrives through GitHub private vulnerability reporting (enabled
   2026-09-19). There is no other private channel.
2. Reply in the advisory thread when read. There is no response-time
   commitment (see SECURITY.md).
3. Reproduce it on the latest release tag and on `main`. Record affected
   versions in the advisory.
4. If accepted, create a private fork from the advisory, develop the fix and a
   regression test there, then follow §1.
5. Agree the disclosure date with the reporter. Default: publish the advisory
   when the patch release is available.
6. If it is not a vulnerability (for example, one of the declared-not-verified
   limitations in SECURITY.md), say why and close the advisory.

## 3. Incidents

An incident is anything that may have put untrusted code or data into a
release, the repository, or a user's hands under the project's name.

| Incident | First actions |
| --- | --- |
| Maintainer GitHub account compromised | Recover the account (§5), revoke all tokens and SSH keys, review audit log, check every tag and release created since the last known-good time, delete and re-issue any that cannot be verified. |
| Malicious or broken commit reached `main` | Revert with a normal PR; if a release includes it, publish a patch release and an advisory. Do not rewrite published history. |
| Secret committed to the repository | Revoke/rotate the secret first, then remove it. Assume it is public from the moment it was pushed. |
| Compromised dependency | Pin or remove it, run `cargo deny check` and the test suite, issue a patch release if a published release is affected. |
| Unverifiable release artifact | Mark the release as affected in its notes, publish a replacement from a known-good commit, explain how to check it. |

After any incident, write a short account (what happened, when it was noticed,
what was affected, what changed) in the release notes or advisory.

## 4. Maintainer rotation

There is no second maintainer, so there is no rotation and no succession. If
the maintainer becomes unavailable, nobody can merge, release, or answer
security reports. This is the largest operational risk of the project and is
recorded as an open gap below.

To add a maintainer:

1. Give the person write (or admin) access to `argorixlabs/argorixlang` and
   require 2FA.
2. Add them to `.github/CODEOWNERS` and to the role table in GOVERNANCE.md.
3. Give them access to the private vulnerability-reporting advisories.
4. Replace "The maintainer decides" in GOVERNANCE.md with a written rule for
   two people, and enable branch protection on `main` requiring one review.

To remove a maintainer: revoke repository access, remove them from CODEOWNERS
and GOVERNANCE.md, and review any tokens or deploy keys they created.

## 5. Accounts and keys

**Release keys do not exist yet.** No tag, release, or artifact is signed, so
there is no release key to lose or recover today. When MAT-023 introduces
release signing, key rotation, compromise, and recovery must follow the
identity policy in [spec/identity-and-keys.md](../spec/identity-and-keys.md)
(I1-04: recovery creates a new key, never un-revokes an old one, and needs two
independent factors). Keys generated by `argorix-sign` are for evaluation only.

What the project depends on instead:

| Credential | Why it matters | Recovery |
| --- | --- | --- |
| GitHub account `argorixlabs` (repository owner) | Merges, tags, releases, advisories, settings | GitHub 2FA recovery codes stored offline. **Whether 2FA is enabled could not be read with the available token — maintainer to confirm.** |
| Maintainer's personal GitHub account | Authored commits | Same as above |
| Zenodo account | DOI archival of releases | Provider account recovery |
| `argorix-lang.org` domain | Website and email | Registrar account recovery; keep auto-renew on |

Keep an offline copy of the 2FA recovery codes for each account. Losing both
the 2FA device and the recovery codes of the owner account means losing control
of the repository.

## 6. Platform maintenance

- CI (`.github/workflows/ci.yml`) tests `main` and pull requests on
  `ubuntu-latest`, `macos-latest`, and `windows-latest` with Rust stable, plus
  Ubuntu with beta. Nothing tests release tags after they are cut; step 2 of §1
  covers that at patch time.
- Dependabot opens dependency PRs; each is reviewed and merged like any other
  PR, with CI green.
- Supported install method: build from source with Rust stable. No binaries
  are distributed.

## 7. Open gaps

| Gap | Effect | Where it is tracked |
| --- | --- | --- |
| Single maintainer, no successor | Project stops if the maintainer is unavailable | This document §4; needs a person, not a task |
| `main` not branch-protected | Review is a practice, not enforced | GOVERNANCE.md; enable when a second reviewer exists |
| Unsigned tags and releases, no provenance | Users cannot verify who produced a release | MAT-023 |
| Release v1.0.1 reports version 1.0.0 | `--version` is wrong for the latest release | Fix in the next release (§1 step 4) |
| No private conduct channel | Conduct reports have no private intake | CODE_OF_CONDUCT.md |
| Language-change rule enforced by PR checklist only | A PR can skip spec or tests if the reviewer misses it | GOVERNANCE.md; a CI check is future work |
| Independent security review | None has been done | MAT-026 |
