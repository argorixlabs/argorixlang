# Argorix Core checker diagnostics dump

Status: ESP-012. Two checkers implement the rules of `evaluation.md`,
`types.md` and `modules.md`: the stage0 checker in
`crates/argorix_semantics/src/core.rs` and `core_link.rs`, and the Argorix
checker and linker in `compiler/check.argx` and `compiler/link.argx`.
`crates/argorixc/tests/check_differential.rs` (one file at a time) and
`link_differential.rs` (whole packages) check that they report the same
diagnostics, at the same places and in the same order.

## Dump

`argorixc core-check-dump <file>` prints the dump, and so does
`compiler.check.dump`. The file is checked on its own, so its locked module
set holds only itself and any import is `ImportNotLocked`.

- A file that does not lex or parse gives the single line `parse failed`.
- A file with no diagnostics gives the single line `ok`.
- Otherwise the dump has one line per diagnostic, in the order the checker
  reports them:

      line:column: phase[Code]

Messages are not part of the dump yet. The code and the position are what
the two checkers must agree on.

## Package dump

`argorixc core-check-package-dump <root> <files...>` prints the dump of a
package, and so does `compiler.link.dump`. The root comes first and the files
of its locked set after it, in the order given. The package is checked the way
`argorixc core-check` checks it (`check_core_package`):

1. A file of the set that does not lex or parse is left out of it. When two
   files declare the same module, the first is kept and the module cannot be
   imported (`DuplicateModule`).
2. The imports are followed depth first from the root, which orders the
   modules with dependencies first. The first import that names a duplicated
   module, closes a cycle (`ImportCycle`) or names a module outside the set
   (`ImportNotLocked`) ends linking.
3. Then each module, in that order, is linked and checked:
   - Its import bindings, and the public structs and enums its imports bring
     in by bare name, must not clash with its own items or with each other
     (`DuplicateImportedName`, at the import).
   - `b.f` and `b.C`, where `b` is an import binding and not a local, must
     name a public function or constant of that module
     (`UnknownImportedSymbol`, `PrivateSymbol`, at the item's name).
   - Otherwise the module is checked with the rules above, its names looked up
     among its own items and the public types of its direct imports.
4. The first module that fails is the one reported; the modules after it are
   not.

The dump is `parse failed` when the root does not parse, `root module declared
twice` when another file of the set declares the root's module, `ok`, or one
line per diagnostic of the module that failed:

    file:line:column: phase[Code]

`file` is the position of that module's file in the list, from 0 for the root.
Link errors are in the `resolution` phase.

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
| ESP-012.C (done) | `DuplicateModule`, `ImportCycle`, `ImportNotLocked` (from linking), `DuplicateImportedName`, `UnknownImportedSymbol`, `PrivateSymbol` |

## How the Argorix checker sees modules

Stage0 turns a package into one program: it renames every item outside the
root to `a__b__name` and concatenates the modules. The Argorix linker merges
the files' trees into one instead, and every declaration keeps the file that
declares it. A type name is looked up among the module's own structs and
enums, then among the public ones of its direct imports; any other name among
the module's own items. `b.f` and `b.C` become paths that carry the item they
name.

The two agree on every package of the differential. They can disagree only
where stage0's renaming is visible, which the Argorix linker does not copy:
a root item spelled like the renamed item of a dependency, such as a root
`fn demo__helper__bump` next to a private `bump` of `demo.helper`, is a
`DuplicateDeclaration` in stage0 and nothing in the Argorix checker. No
source has a reason to spell such a name, and the renaming is stage0's
choice for its C backend, not a rule of `modules.md`.
