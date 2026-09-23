# Argorix Core tokens and the canonical token dump

Status: ESP-010. Two lexers implement this: the stage0 lexer in
`crates/argorix_parser/src/core.rs` and the Argorix lexer in
`compiler/lexer.argx`. They must produce the same dump for every input, and
`crates/argorixc/tests/lexer_differential.rs` checks that they do.

## Input

The source is a sequence of bytes that must be valid UTF-8. If it is not,
lexing stops with a single `InvalidUtf8` diagnostic. That diagnostic points at
the first byte of the first invalid sequence, and its line and column are
counted over the valid text before it.

## Positions

A span is a byte range `start..end`. Its line and column are those of its
first character:

- lines are counted by LF, from 1;
- columns are counted in Unicode scalar values from the start of the line,
  from 1;
- CR is ordinary whitespace, so a CRLF file counts its lines by the LF.

## Tokens

Tokens are separated by whitespace (space, tab, CR, LF) and by comments.

- A `//` comment runs to the end of the line, and the LF is not part of it.
- A `/* */` comment nests.

| Token | Spelling |
| --- | --- |
| `Ident` | `_` or an ASCII letter, then ASCII letters, digits and `_`. Keywords are identifiers; the parser gives them meaning. |
| `Integer` | A digit, then digits and `_`, then an optional run of ASCII letters and digits: the suffix. The value, with the `_` removed, must fit in `u64`. The parser checks that the suffix names an integer type. |
| `String` | `"` … `"` on one line. Escapes: `\n`, `\r`, `\t`, `\"`, `\\`. Any other character is copied verbatim. |
| operators | `{ } ( ) [ ] , ; . ^ : :: -> => = += -= *= /= %= + - * / % ! == != < <= > >= << >> & && \| \|\|` |
| `Eof` | An empty span at the end of the input. |

Operators match greedily over two characters. `>>=` is therefore `>>`
followed by `=`, and `<<=` is `<<` followed by `=`.

## Diagnostics

Lexing continues after an error, so one pass reports every lexical problem.

| Code | Span | Message |
| --- | --- | --- |
| `UnexpectedCharacter` | the character | ``unexpected character `c` `` |
| `UnterminatedComment` | the opening `/*` | `unterminated block comment` |
| `IntegerOutOfRange` | the whole literal, suffix included | `integer literal exceeds u64` |
| `UnterminatedString` | from the opening quote to the LF or the end of the input | `unterminated string literal` |
| `InvalidEscape` | from the opening quote to after the escaped character | ``invalid escape `\c` `` |
| `InvalidUtf8` | empty, at the first invalid byte | `source is not valid UTF-8` |

## Canonical dump

`argorixc core-tokens <file>` prints the dump, and so does
`compiler.token_dump.dump`. When lexing found any diagnostic, the dump holds
one line per diagnostic, in order:

    line:column: lexical[Code]: message

Otherwise it holds one line per token, `Eof` included:

    line:column start end Kind[ payload]

The payload depends on the kind:

- `Ident`: the identifier.
- `Integer`: the value in decimal, then the suffix if there is one.
- `String`: the decoded value in double quotes, escaped as
  `stdlib.text.append_json_escaped` does.

Every line ends in LF. Kind names are those of the table above:
`Ident`, `Integer`, `String`, `LeftBrace`, …, `ShiftRight`, `Eof`.

## Discrepancies found by the port

| # | Stage0 behaviour | Resolution |
| --- | --- | --- |
| 1 | A string ending in `\` at the end of the input produced no diagnostic and no token, so the file lexed cleanly. | Fixed in both lexers: `UnterminatedString`. |
| 2 | `grammar.ebnf` lists `byte_literal`, but neither lexer nor parser has one. | Open, as a language decision: the grammar entry stays until byte literals are implemented or removed. |
| 3 | An escaped LF inside a string (`"a\` then LF) reports `InvalidEscape` and lets the string continue on the next line. | Kept, and specified above, so that both lexers agree. Only an unescaped LF ends a string. |
| 4 | A suffix can be any alphanumeric run (`12abc`). | Kept: the lexer tokenizes and the parser checks that the suffix names an integer type. |
