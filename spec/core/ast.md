# Argorix Core syntax tree and the canonical AST dump

Status: ESP-011. Two parsers implement this: the stage0 parser in
`crates/argorix_parser/src/core.rs` and the Argorix parser in
`compiler/parser.argx`. For every input they must produce the same dump, and
`crates/argorixc/tests/parser_differential.rs` checks that they do.

## Parsing rules the dump makes visible

- **Grammar.** The grammar is `grammar.ebnf`.
- **Binary operators.** They bind by these levels, lowest first, all
  left-associative:

  | Level | Operators |
  | --- | --- |
  | 1 | `\|\|` |
  | 2 | `&&` |
  | 3 | `==` `!=` |
  | 4 | `<` `<=` `>` `>=` |
  | 5 | `\|` |
  | 6 | `^` |
  | 7 | `&` |
  | 8 | `<<` `>>` |
  | 9 | `+` `-` |
  | 10 | `*` `/` `%` |

  The prefix operators `!` and `-` bind tighter than any binary operator.
- **Aggregates.** `path { field: value }` is an aggregate only where an
  aggregate may appear. It never is in the condition of `if` or `while`, or in
  the scrutinee of `match`, so `if x { … }` reads the braces as a block.
- **Type arguments.** A `>>` that closes two type arguments, as in
  `Buffer<Buffer<u8>>`, closes both. The inner type ends one byte into the
  token.
- **Nesting.** Expressions, blocks, types and patterns may nest at most 256
  levels (`CORE_NESTING_LIMIT`, `compiler.parser.NESTING_LIMIT`). Past that the
  parser reports `NestingTooDeep` instead of recursing. The stage0 driver runs
  on a 64 MiB stack so that it reaches the limit instead of overflowing.
- **Recovery.** An error inside an item abandons that item, and parsing
  resumes at the next `fn`, `struct`, `enum`, `const` or `pub`. An error
  before the first item (the version, the module or an import) ends the
  parse.

## Dump

`argorixc core-ast <file>` prints the dump, and so does
`compiler.parser.dump`.

- **Lexing fails:** the dump is the lexical diagnostics of
  `spec/core/tokens.md`.
- **Parsing fails:** one line per syntax error, in order:

      line:column: syntax[Code]: message

- **Otherwise:** one line per node, in preorder. Each line has two spaces of
  indentation per level, then the label, then the node's byte span:

      Label @start..end

| Node | Label | Children, in order |
| --- | --- | --- |
| program | `Program 0.1 <module>` | imports, items |
| import | `Import <path>[ as <alias>]` | — |
| function | `[pub ]Fn <name>` | `Param`s, return type, body |
| parameter | `Param <name>` | type |
| struct | `[pub ]Struct <name>` | `Field`s |
| enum | `[pub ]Enum <name>` | `Variant`s |
| variant | `Variant <name>` | `Field`s |
| field | `Field <name>` | type |
| const | `[pub ]Const <name>` | type, value |
| type | `Type <name>`, `Type Array <n>` | element type, for a container |
| block | `Block` | statements, then the value if the block has one |
| let | `Let [mut ]<name>` | type if annotated, value |
| assignment | `Assign <op>` | place, value |
| while | `While` | condition, body |
| break, return | `Break`, `Return` | value, if any |
| continue | `Continue` | — |
| expression statement | `Expr` | expression |
| literals | `Integer <n>[ <suffix>]`, `String "<escaped>"`, `Bool <b>`, `Unit` | — |
| path | `Path a::b` | — |
| aggregate | `Aggregate a::b` | `FieldValue <name>`, each over its value |
| array | `Array` | elements |
| if | `If` | condition, then block, else branch if any |
| match | `Match` | scrutinee, `Arm`s |
| arm | `Arm` or `Arm guarded` | pattern, guard if any, value |
| loop | `Loop` | body |
| call | `Call` | callee, arguments |
| index, field | `Index`, `Field <name>` | value (and index) |
| operators | `Unary <op>`, `Binary <op>` | operands |
| patterns | `Pattern _`, `Pattern <b>`, `Pattern <n>`, `Pattern binding <x>`, `Pattern a::b` | `PatternField <name>`, each over its nested pattern if any |

A string is escaped as `stdlib.text.append_json_escaped` does. Paths are
written with `::` and module paths with `.`, whatever spacing the source used.

## Spans

A span is `first token start .. last token end`, with these rules the stage0
parser has always followed and the port keeps:

- Parentheses widen their expression's span and add no node.
- A `struct`'s span ends at its last field, including the comma, or at its
  name. An `enum`'s span ends at its last variant or at its name. A function's
  span ends at its body, and a `const`'s at its value.
- An expression statement's span is its expression's, without the `;`.
- A `FieldValue` or `PatternField` spans its name.
- `=> return value` in a match arm is a `Block` holding one `Return`. The
  block's span is the arm's first token.

## Discrepancies found by the port

| # | Stage0 behaviour | Resolution |
| --- | --- | --- |
| 1 | No nesting limit: the stage0 parser recursed until the host stack ran out. A debug `argorixc` crashed near 55 levels of parentheses on Windows. | Fixed: `NestingTooDeep` past 256 levels in both parsers, and the stage0 driver runs on a 64 MiB stack. |
| 2 | The spans of `struct`, `enum` and `=> return` arms follow the rules under "Spans", not the closing brace. | Kept and specified above, so that both parsers agree; changing them would move every diagnostic that points there. |
