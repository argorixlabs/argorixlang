# Agent-language syntax tree dump

Status: ESP-018.B, in progress. The grammar is that of
[`current-v1.md`](current-v1.md) and [`declarations.md`](declarations.md).
Two parsers implement it:

- the stage0 parser in `crates/argorix_parser/src/parser.rs`, whose tree
  is `ast.rs`;
- the Argorix parser in `compiler/agent_parser.argx`, whose tree is
  `compiler/agent_ast.argx`, shaped by `compiler/agent_schema.argx`.

`crates/argorixc/tests/agent_parser_differential.rs` compares them over
every agent-language `.argx` file of the repository and the samples in
`tests/selfhost/agent/parser_samples/`.

## Dump

`argorixc agent-ast <file>` prints the dump, and so does
`compiler.agent_parser.dump`.

- Source that is not UTF-8, or that does not lex, gives the token dump's
  lines (`tokens.md`).
- Source that does not parse gives its first error, as
  `line:column: message`. The parser stops at its first error.
- Otherwise the program is one line of compact JSON, as `serde_json` writes
  the stage0 tree:
  - a struct is an object of its fields, in declaration order;
  - `Vec` is an array and `Option` is the value or `null`;
  - `Spanned<T>` is `{"value":…,"span":{"start":…,"end":…,"line":…,"column":…}}`;
  - an enum variant is its name when it has no data, `{"Name":value}` for
    one value, and `{"Name":{fields}}` for fields;
  - strings are escaped as `serde_json` escapes them:
    - `"` and `\` get a backslash;
    - `\b`, `\t`, `\n`, `\f` and `\r` use their short forms;
    - other control characters are `\u00xx` in lowercase hex;
    - everything else is written as it is.

## The schema

`compiler/agent_schema.argx` is generated from `ast.rs` and `span.rs` by
`crates/argorixc/tests/agent_schema.rs`. The test fails when the file and
the sources disagree, and `ARGORIX_BLESS=1` regenerates it. The schema holds:

- every struct, and every enum variant with fields, as a numbered struct
  with its fields in order;
- every enum, with its variants and their shapes.

So a change to the stage0 tree shows up as a schema change, not as a
silent divergence.

## Coverage

The port goes one group of declarations at a time. A program that uses a
declaration not ported yet dumps `unsupported`, and the differential counts
it instead of comparing it. Every program that is compared must match.
`MATCHED_AT_LEAST` in the test records how far the port has come.

| Group | Declarations | Programs matched |
| --- | --- | --- |
| 1 | `module`, `import`, `type`, `enum`, `capability`, `tool`, `model`, `failure`, `assert`, `agent` (with its handlers), `protocol` | 267 of 1,385: 224 of the corpus and 43 samples |
