# Security Policy

## Reporting a vulnerability

**Do not report security issues through public GitHub issues, discussions, or
pull requests.**

Report privately through GitHub's private vulnerability reporting:
<https://github.com/argorixlabs/argorixlang/security/advisories/new>

This is the only private channel. The project has no security mailbox; earlier
versions of this file listed `security@argorixlabs.dev`, but that domain does
not exist, so mail sent there could not be delivered. If you reported something
by email, please resend it through the link above.

Please include:

- A description of the vulnerability and its impact.
- Steps to reproduce (a minimal `.argx` snippet or bytecode artifact is ideal).
- Affected version(s) or commit SHA.

## What to expect

Argorix Lang has a single maintainer (see [GOVERNANCE.md](GOVERNANCE.md)).
Reports are handled on a **best-effort basis with no guaranteed response
time**. In practice:

1. The report is acknowledged in the advisory thread when the maintainer reads
   it.
2. The maintainer assesses it, asks for clarification if needed, and says
   whether it is accepted as a vulnerability.
3. Accepted issues are fixed on `main` and, if the latest release is affected,
   in a patch release following
   [`operations/maintenance.md`](operations/maintenance.md).
4. The disclosure date is agreed with the reporter in the advisory thread. You
   are credited in the advisory unless you prefer to remain anonymous.

If you have heard nothing after 14 days, you may add a comment to the advisory
to ask for a status update. This is not a service-level commitment.

## Supported versions

| Version | Security fixes |
| --- | --- |
| v1.0.1 (latest release) | Yes, best effort |
| v1.0.0 and all v0.x | No — upgrade to v1.0.1 |
| `main` (unreleased) | Fixed on `main`; no advisory for unreleased code |

The table in [GOVERNANCE.md](GOVERNANCE.md#supported-versions) is the source of
truth and is updated with each release.

## Scope

In scope: the compiler (`argorixc`), bytecode verifier, VM (`argorix-vm`),
evidence verification and signing (`argorix-sign`), the conformance runner, and
any case where the runtime performs an effect that governance did not permit.

Declarations are not guarantees. The following are documented limitations, not
vulnerabilities, unless the code claims more than the documentation says:

- a declared MCP or A2A bridge is not a connected bridge;
- a dry-run identity or handshake is not authenticated identity;
- a trust-ledger hash chain is not an immutability guarantee;
- a mapped control is not regulatory approval;
- post-quantum readiness metadata is not proof of post-quantum security;
- `argorix-sign` keys are for evaluation and have no storage, rotation,
  revocation, or timestamping.

Out of scope: the optional web demo under `demo/`, the paper sources, and the
evaluation harness, except where they show a flaw in the runtime itself.
