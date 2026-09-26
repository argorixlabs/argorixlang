# Agent-language IR dump

Status: ESP-018.D.1, done. Two implementations lower a checked agent-language
program to the IR:

- stage0's `IrProgram::from(&Program)` in `crates/argorix_ir/src/ir.rs`,
  which `argorixc emit-ir` prints;
- the Argorix `compiler/agent_ir.argx`, which reads the tree of
  `compiler/agent_parser.argx`.

`crates/argorixc/tests/agent_ir_differential.rs` compares them over every
agent-language `.argx` file of the repository and the parser and checker
samples.

## Dump

`argorixc agent-ir <file>` prints the dump, and so does
`compiler.agent_ir.dump`.

- A program that does not parse or check gives the checker dump
  (`check.md`).
- Otherwise the IR as `serde_json::to_string_pretty` writes it, and a line
  feed: two spaces per level, `"key": value`, `[]` for an empty list, the
  same string escapes as the syntax-tree dump (`ast.md`).

## Packages

`argorixc agent-package-ir <directory or manifest>` prints a package's IR
dump, and so does `compiler.agent_package.ir_dump`:

- a package that does not resolve gives the `error:` line of
  `packages.md`;
- a merged program that does not check gives its diagnostics, one
  `line:column: message` line each;
- otherwise `package_ir`: the merged program's IR, named after the entry
  module, with `modules` (name and path, by name) and `imports` (from and
  to, sorted, without repeats) after `module`.

`crates/argorixc/tests/agent_package_differential.rs` compares these dumps
too, for every package.

## The IR

The IR is the syntax tree without spans:

- each field of an IR type is, in order, a value, an enum's source name,
  an optional value (`null`, or left out where serde skips `None`), a list
  of values or of another IR type (left out where serde skips an empty
  list), an optional IR type, or a constant (`ir_version` `1.0`, `language`
  `Argorix Lang`, a failure's `trace` `required`);
- the emitter of each IR type is generated from `ir.rs` by
  `crates/argorixc/tests/agent_ir_table.rs`, which fails when they drift
  apart;
- the fields of no regular shape are written by hand: a policy rule's
  effect and rule, a tool's provider (`simulated` when none is given), an
  agent's approval (`denied` when none is given), a handler's instructions
  (tagged by `op`), and a package's modules and imports.
