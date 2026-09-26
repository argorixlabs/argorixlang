# Agent-language token dump

Status: ESP-018.A. The lexical rules are those of
[`current-v1.md`](current-v1.md), L-01. Two lexers implement them:

- the stage0 lexer in `crates/argorix_parser/src/lexer.rs`;
- the Argorix lexer in `compiler/agent_lexer.argx`.

`crates/argorixc/tests/agent_lexer_differential.rs` checks that they
produce the same dump, byte for byte, for every `.argx` file of the
repository and the adversarial samples in `tests/selfhost/agent/samples/`.

## Dump

`argorixc agent-tokens <file>` prints the dump, and so does
`compiler.agent_lexer.dump`.

- Source that is not UTF-8 gives one line, at its first invalid byte:

      line:column: source is not valid UTF-8

- A source with lexical errors gives one line per error, in the order the
  lexer finds them, and no tokens:

      line:column: message

  The messages are `` unexpected character `c` `` (the character as it is
  in the source), `unterminated string literal` and
  `integer literal exceeds u64 range`.
- Otherwise there is one line per token, the last one `Eof`:

      line:column start end Kind

  - `start` and `end` are byte offsets.
  - `line` counts line feeds and `column` counts characters, both from 1.
  - An `Ident` is followed by its text.
  - A `String` is followed by its value between quotes, escaped as JSON:
    `"`, `\`, LF, CR and tab as `\"`, `\\`, `\n`, `\r`, `\t`, other controls
    as `\u00xx`.
  - An `Integer` is followed by its decimal value.
  - The other kinds are `LeftBrace`, `RightBrace`, `LeftParen`,
    `RightParen`, `LeftBracket`, `RightBracket`, `Comma`, `Colon`, `Arrow`
    and `Eof`.

## Characters

An identifier starts with `_` or an alphabetic character. It goes on with
`_`, `.` or an alphanumeric one:

- alphabetic is Unicode's `Alphabetic` property;
- alphanumeric adds the general categories `Nd`, `Nl` and `No`.

These are what Rust's `char::is_alphabetic` and `char::is_alphanumeric`
say. `compiler/unicode.argx` holds them for the Argorix lexer. It is
versioned data, generated from the standard library the stage0 lexer is
built with. `crates/argorixc/tests/unicode_tables.rs` fails when the file
and that standard library disagree. `ARGORIX_BLESS=1` regenerates it, and
its header names the Unicode version, 17.0.0 today. A new Rust toolchain
with a newer Unicode version therefore changes both lexers together, and
only on purpose.

A digit that is only numeric, such as `٣` (U+0663), can continue an
identifier but not start one. A combining mark continues one only when it
is `Alphabetic`. Anything else outside ASCII is an unexpected character.
