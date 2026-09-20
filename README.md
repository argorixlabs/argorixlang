<div align="center">
  <img width="520" src="https://argorix-lang.org/assets/argorix-lockup.png" alt="Argorix Lang" />

  <p><strong>Secure, verifiable programs for governed AI-agent systems.</strong></p>

  <p>
    <a href="https://argorix-lang.org">Website</a> ·
    <a href="#quickstart">Quickstart</a> ·
    <a href="#project-status">Status</a> ·
    <a href="#roadmap">Roadmap</a> ·
    <a href="./LICENSE">Apache-2.0</a>
  </p>
</div>

[![CI](https://github.com/argorixlabs/argorixlang/actions/workflows/ci.yml/badge.svg)](https://github.com/argorixlabs/argorixlang/actions/workflows/ci.yml)
[![Security & licenses](https://github.com/argorixlabs/argorixlang/actions/workflows/security.yml/badge.svg)](https://github.com/argorixlabs/argorixlang/actions/workflows/security.yml)
[![DCO](https://github.com/argorixlabs/argorixlang/actions/workflows/dco.yml/badge.svg)](https://github.com/argorixlabs/argorixlang/actions/workflows/dco.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

# Argorix Lang

Argorix Lang is a compiled language and runtime for communication between AI
agents under explicit policy, capability, evidence, and provider boundaries.
Programs are parsed, checked, lowered to Argorix IR and bytecode, verified, and
executed by a deterministic VM.

The current toolchain is implemented in Rust. The project is progressively
bootstrapping a smaller systems subset, **Argorix Core 0.1**, so that the
compiler and runtime can ultimately be implemented in Argorix itself.

> Rust is the forge. Argorix Lang is the sword.

## Project status

| Area | Current state |
| --- | --- |
| Latest release | `v1.0.1` — see [supported versions](GOVERNANCE.md#supported-versions) |
| Workspace version | `1.0.0` |
| Compiler and VM | Rust implementation |
| Agent language | Parser, semantic checks, IR, bytecode, verification, and deterministic VM |
| Runtime profiles | `dry_run`, `simulated`, and governed `sandboxed_external` planning |
| Core bootstrap | Core 0.1 parsing and semantic checking in Rust stage0 |
| Self-hosting | In progress; not achieved yet |
| Rust independence | Planned and gated; not achieved yet |

The current release is an early-stage secure multi-agent runtime MVP. It is not
a production certification, a general-purpose sandbox, or proof that every
declared trust or security property has been externally verified.

## Quickstart

### Requirements

- Rust stable with `cargo`.
- Node.js 18.18 or newer only for the optional web demo.

### Build and test

```bash
cargo build --workspace --locked
cargo test --workspace --locked
```

The main binaries are written to `target/debug/`:

- `argorixc` — compiler and verifier;
- `argorix-vm` — deterministic VM and evidence tooling;
- `argorix-conformance` — conformance-suite runner.

### Validate and compile an Argorix program

```bash
cargo run -p argorixc -- check examples/runtime_mvp_v100.argx
cargo run -p argorixc -- emit-bytecode examples/runtime_mvp_v100.argx
cargo run -p argorixc -- verify-bytecode examples/runtime_mvp_v100.argx
```

### Run and verify evidence

```bash
cargo run -p argorix-vm -- run examples/runtime_mvp_v100.argbc.json \
  --dry-run \
  --reactive \
  --inject "User:ResearchAgent:tell:UserPrompt" \
  --security-report report.security.json \
  --trace-out report.trace.json \
  --evidence-bundle report.evidence.json

cargo run -p argorix-vm -- verify-evidence report.evidence.json
cargo run -p argorix-conformance -- run conformance/suite.v100.json
```

PowerShell users can place each command on one line or replace Bash line
continuations with PowerShell backticks.

## Argorix Core bootstrap

Core 0.1 is the systems subset used by the self-hosting program. Core files are
explicitly versioned and begin with:

```argx
core 0.1;
module example.main;
```

The stage0 frontend can currently perform syntax and semantic validation:

```bash
cargo run -p argorixc -- core-check tests/selfhost/spec/valid/lexer.argx
```

This command does **not** yet lower Core to executable IR or run Core programs.
Those capabilities belong to the next bootstrap gates. The historical agent
compiler and the Core frontend remain deliberately isolated so that one syntax
cannot be silently interpreted as the other.

Core 0.1 currently covers the constructs needed to express a lexer, recursive
parser, AST, and symbol table, including fixed-width integers, arrays, slices,
buffers, arenas, typed handles, structs, enums, exhaustive matching, functions,
recursion, lexical scopes, loops, modules, and imports.

See [`spec/core/README.md`](spec/core/README.md) and the executable corpus under
[`tests/selfhost/spec`](tests/selfhost/spec).

## Runtime boundaries

Argorix follows one governing rule:

```text
Runtime may execute only what governance explicitly permits.
```

- `dry_run` validates and records trace/evidence without provider execution.
- `simulated` uses the deterministic in-process simulated provider.
- `sandboxed_external` remains blocked unless the program and invocation pass
  every adapter, operation, policy, hardening, evidence, governance, audit, and
  fail-closed check.

Core Argorix does not ship a mandatory provider SDK and does not make arbitrary
HTTP, shell, tool, MCP, or A2A calls. Provider endpoints and secret references
are represented as redacted references; secret values must never be embedded in
source, bytecode, traces, reports, or evidence bundles.

Several language declarations describe governance or trust metadata. A
declaration is not evidence that an external event occurred:

- a declared MCP or A2A bridge is not a connected bridge;
- a dry-run identity or handshake is not authenticated identity;
- a trust-ledger hash chain is not a blockchain or immutability guarantee;
- a mapped control is not regulatory approval or legal certification;
- post-quantum readiness metadata is not proof of post-quantum security.

For the maintained security policy and disclosure process, read
[`SECURITY.md`](SECURITY.md).

## Architecture

```text
.argx source or package
  -> lexer and parser
  -> semantic, capability, and policy checks
  -> Argorix IR
  -> versioned Argorix Bytecode
  -> bytecode verification
  -> deterministic VM and scheduler
  -> governed provider/tool boundary
  -> trace, SecurityReport, and EvidenceBundle
```

The bootstrap path is separate and deliberately incremental:

```text
Rust stage0
  -> parse and check Core 0.1
  -> executable Core IR and verifier
  -> temporary C backend and minimal runtime
  -> compiler phases written in .argx
  -> stage1/stage2 equivalence
  -> native backend and removal of Rust from required product builds
```

## Repository map

| Path | Purpose |
| --- | --- |
| `crates/argorixc` | Compiler CLI |
| `crates/argorix_parser` | Historical agent parser and isolated Core frontend |
| `crates/argorix_semantics` | Semantic and security checks |
| `crates/argorix_ir` | Intermediate representation |
| `crates/argorix_bytecode` | Bytecode lowering and verification |
| `crates/argorix_vm` | VM library |
| `crates/argorix-vm` | VM CLI |
| `crates/argorix_conformance` | Conformance runners and executable models |
| `spec` | Normative contracts and bootstrap specifications |
| `conformance` | Versioned suites and policy/model fixtures |
| `tests/selfhost` | Core bootstrap corpus |
| `examples` | Valid programs, packages, and bytecode fixtures |
| `demo/argorix-chatbot-runtime` | Optional governed chatbot demo |

## Demo

The web demo exercises contracts, policy, evidence, and fail-closed input
handling. Build the Rust workspace first, then:

```bash
cd demo/argorix-chatbot-runtime
npm install
cp .env.example .env.local
npm run dev
```

The default configuration is plan-only and does not contact an external
provider. See the [demo documentation](demo/argorix-chatbot-runtime/README.md)
for the explicit sandboxed-provider configuration and its boundaries.

<div align="center">
  <a href="https://www.youtube.com/watch?v=ZhQMps17CFo">
    <img width="560" src="https://img.youtube.com/vi/ZhQMps17CFo/maxresdefault.jpg" alt="Watch the Argorix Lang demo" />
  </a>
</div>

## Roadmap

The project separates three outcomes that must not be conflated:

1. **Independent toolchain** — compiler, verifier, VM, packages, signatures,
   and essential tools no longer require Rust in product builds.
2. **Functionally complete agent platform** — authenticated communication,
   governed execution, recovery, tooling, and complete reference applications.
3. **Production readiness** — sustained reliability, independent security
   validation, external beta evidence, governance, support, and release gates.

The current source of truth is:

- [`PLAN_MAESTRO_ARGORIXLANG.md`](PLAN_MAESTRO_ARGORIXLANG.md) — maturity and
  product plan;
- [`PLAN_ESPADA_INDEPENDIENTE.md`](PLAN_ESPADA_INDEPENDIENTE.md) — technical
  path from Rust stage0 to an independent Argorix toolchain;
- [`tasks/madurez/BACKLOG.json`](tasks/madurez/BACKLOG.json) — machine-readable
  status and dependencies.

Passing a bootstrap gate does not by itself prove production maturity or a
security guarantee. Each completed gate links to reproducible evidence and
states what remains unproven.

## Contributing

Contributions are welcome. Please read [`CONTRIBUTING.md`](CONTRIBUTING.md) and
[`GOVERNANCE.md`](GOVERNANCE.md), use Conventional Commits, include a DCO
sign-off, and keep behavioral claims tied to tests or reproducible evidence.

```bash
git commit --signoff -m "feat: describe the change"
```

## License

Argorix Lang is licensed under the [Apache License 2.0](LICENSE).
