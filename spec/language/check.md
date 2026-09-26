# Agent-language checker dump

Status: ESP-018.C, in progress. Two checkers implement the checks of the
existing language:

- the stage0 checker in `crates/argorix_semantics/src/checker.rs`;
- the Argorix checker in `compiler/agent_check.argx`, which reads the tree of
  `compiler/agent_parser.argx`.

`crates/argorixc/tests/agent_check_differential.rs` compares them over every
agent-language `.argx` file of the repository and the samples in
`tests/selfhost/agent/parser_samples/` and `check_samples/`.

## Dump

`argorixc agent-check <file>` prints the dump, and so does
`compiler.agent_check.dump`. It checks with stage0's default options:
capabilities must be declared.

- Source that does not lex or parse gives the syntax-tree dump's lines, its
  first error (`ast.md`).
- A program the checker accepts gives the single line `ok`.
- Otherwise each diagnostic is one line, `line:column: message`, in the
  order the checker reports them.

## Coverage

The checks are ported one group of declarations at a time, in stage0's
order. A program that declares something whose checks are not ported yet
dumps `unsupported`, and the differential counts it instead of comparing it.
Every program that is compared must match; `MATCHED_AT_LEAST` records how
far the port has come.

| Group | Checks | Programs matched |
| --- | --- | --- |
| 1 | Symbols; assertions, policies and failures; tools and models; message types; agents, their capabilities and handlers; protocols | 533 of 1,430 |
| 2 | Provider contracts, features, secrets, cryptos, crypto boundaries (no checks), DID methods, and every duplicate `collect_symbols` reports | 737 of 1,430 |
| 3 | Harnesses and adapters | 782 of 1,430 |
| 4 | Adapter profiles | 789 of 1,430 |
| 5 | Passports, including `asn` registry, number and country | 846 of 1,430 |
| 6 | A-Trust boundaries, identities, credential contracts and handshakes | 976 of 1,430 |
| 7 | Trust ledgers and their hash chains | 1,025 of 1,430 |
| 8 | MCP and A2A bridge contracts | 1,083 of 1,430 |
| 9 | A-Trust evidence maps | 1,135 of 1,430 |
| 10 | Governance profiles and regulatory mappings | 1,185 of 1,430 |
| 11 | Third-party verifiers and public conformance reports | 1,240 of 1,430 |
| 12 | Runtime hardening profiles and threat models | 1,294 of 1,430 |
