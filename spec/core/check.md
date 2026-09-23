# Argorix Core checker diagnostics dump

Status: ESP-012. Two checkers implement the rules of `evaluation.md`,
`types.md` and `modules.md`: the stage0 checker in
`crates/argorix_semantics/src/core.rs` and the Argorix checker in
`compiler/check.argx`. `crates/argorixc/tests/check_differential.rs` checks
that they report the same diagnostics, at the same places and in the same
order.

## Dump

`argorixc core-check-dump <file>` prints the dump, and so does
`compiler.check.dump`. The file is checked on its own, so its locked module
set holds only itself and any import is `ImportNotLocked`. Linking modules
comes with ESP-012.C.

- A file that does not lex or parse gives the single line `parse failed`.
- A file with no diagnostics gives the single line `ok`.
- Otherwise the dump has one line per diagnostic, in the order the checker
  reports them:

      line:column: phase[Code]

Messages are not part of the dump yet. The code and the position are what
the two checkers must agree on.

## Order

The checker runs these passes, and each pass reports in program order:

1. It collects declarations. A duplicate name, or a capability held in a
   container type, is reported here.
2. It checks imports.
3. It finds structs that contain themselves without a `Handle`. This pass
   used to follow the iteration order of a hash map; it now follows program
   order.
4. It checks constants.
5. It checks functions, statement by statement and operand by operand, left
   to right.
6. It checks where capabilities are held.

Where one construct reports several diagnostics at the same place, such as the
missing fields of an aggregate, only their number matters. Their messages are
now also sorted.

## Codes by subtask

| Subtask | Codes |
| --- | --- |
| ESP-012.A (done) | `DuplicateDeclaration`, `ImportNotLocked`, `InfiniteType`, `TypeMismatch`, `ImmutableAssignmentOrUnknownName`, `LiteralOutOfRangeOrConstantTrap`, `ControlOutsideLoop`, `StringNotByteIndexable`, `NonExhaustiveMatch`, `CapabilityEscapes` |
| ESP-012.B (done) | `UseAfterMove`, `MoveInLoop`, `MoveOutOfPlace`, `ResourceTemporary`, `ResourceInArena`, `SliceEscapes`, `SliceAliasesMove` |
| ESP-012.C | module linking: `UnknownImportedSymbol` and the other resolution errors of `core_link` |

With A and B in place, the differential compares the whole stage0 dump. The
linker (C) is the remaining piece: until it lands, files are checked one at a
time.
