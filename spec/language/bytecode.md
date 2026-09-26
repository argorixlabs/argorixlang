# Agent-language bytecode dump

Status: ESP-018.D.2, done. Two implementations lower a checked
agent-language program to bytecode:

- stage0's `lower_ir` in `crates/argorix_bytecode/src/lower.rs`, which
  `argorixc emit-bytecode` and `emit-bytecode-package` print after
  verifying;
- the Argorix `compiler/agent_ir.argx` (`bytecode_dump`, and
  `compiler.agent_package.bytecode_dump` for packages).

`crates/argorixc/tests/agent_bytecode_differential.rs` compares them over
every agent-language file and sample, and
`agent_package_differential.rs` over every package.

## Dump

`argorixc agent-bytecode <file>` and `agent-package-bytecode <directory or
manifest>` print the dump:

- a file or package that does not resolve, parse or check gives the IR
  dump's text for it (`ir.md`);
- otherwise the bytecode as serde_json's pretty JSON and a line feed. A
  file's carries `source_digest`, `sha256:` and the lowercase SHA-256 of its
  bytes; a package's has none.

The bytecode is not verified here. What the verifier decides is a dump of
its own (ESP-018.D.3), so a program that checks but that the verifier
rejects, such as one with no agent, is still compared.

## The bytecode

- **The metadata** (providers, harnesses, the declarations, agents,
  capabilities, tools, models, ...) copies IR fields. Its emitters are
  generated from `lower_ir`'s literal and the bytecode types by
  `crates/argorixc/tests/agent_ir_table.rs`, composed with the IR emitters:
  each bytecode field is written with the shape of the IR field it copies.
- **Written by hand:** the source digest, the enum names, and the
  instructions, in `lower_ir`'s order: providers, assertion declarations,
  failures, capabilities, tools, models, each agent (its declaration, its
  capabilities with `RequireApproval` for one that needs approval or is
  restricted or dangerous, tools, models, and handlers with their
  instructions and `EndHandler`), each protocol (its steps and
  `protocol <name> completed`), assertion checks, `PolicyReport` and `End`.
  Each instruction is an object tagged by `op` with the variant's name.
