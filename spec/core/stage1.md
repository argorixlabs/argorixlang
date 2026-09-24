# Stage1: the self-hosted compiler and its build file

Status: ESP-014. Stage1 is `compiler/main.argx` and the rest of `compiler/`
and `stdlib/`, built by stage0 through the transitional C backend
(`spec/core/c-backend.md`). Every phase it runs is Argorix: lexing,
parsing, linking, checking, diagnostics, IR, C emission and the build
manifest. C is still its only backend.

## Why a build file

A Core program run under the compiler-host profile gets no command line, no
environment and no directory listing (`spec/core/stdlib.md`). It can only
read files under the package root it is lent, and write files under the build
root. So stage1 reads its whole input from one file, `argorix.build`, at the
package root. The driver's own flags, `--package-root`, `--read-budget`,
`--build-root` and `--write-budget`, belong to the host shim.

Because the build file is the whole input besides the sources, the manifest
of a build can name everything the build depended on.

## `argorix.build`

```text
argorix-build 1
# comments and blank lines are ignored
root compiler/main.argx
module stdlib/bytes.argx
module compiler/token.argx
c compiler.c
manifest compiler.json
diagnostics diagnostics.txt
```

- The first line is `argorix-build 1`: the format and its version.
- Every other line that is not blank and does not start with `#` is
  `key value`: one space, and a value with no space in it. A line may end in
  LF or CR LF.
- `root` is the file whose module holds `argorix_main`. Each `module` line
  adds a file to the locked compilation set, in the order given. These paths
  are relative to the package root.
- `c`, `manifest` and `diagnostics` name the files to write, relative to the
  build root. Their directories must already exist there.
- `root`, `c`, `manifest` and `diagnostics` appear exactly once. `module`
  may repeat, and there is no other key.

Paths are read through `stdlib.compiler_host`, so they must be normalized
relative paths (`stdlib.path`). The host checks them again against its roots.

The locked set is what stage0 builds from directories. For a root in
`compiler/` with `--stdlib stdlib`, that is every other module of
`compiler/`, then `stdlib/`. `argorix.build` at the root of the repository
lists exactly that set, and `crates/argorixc/tests/stage1.rs` keeps the two
in step. A file of the set that does not parse is left out of it, and a
module declared by two files cannot be imported, as in stage0.

## Results and files written

`argorix_main` returns, and the host prints as `ARGORIX_RESULT:<n>`:

| Result | Meaning | Written |
| --- | --- | --- |
| 0 | The package was compiled | C, manifest, empty diagnostics |
| 1 | The package does not link or check | manifest, diagnostics |
| 2 | The C backend refuses the package | manifest, diagnostics |
| 3 | The build file is not valid | its first error, in the diagnostics file, when that line was read before the error; otherwise nothing |
| 1000000 + digest | A file could not be read or written | for a source, `path: reason` in the diagnostics file |

The digest is `stdlib.result`'s `failure_digest`:

- 7000000 plus the host status for a refusal of the host, so 8000002 is
  "not found";
- 4000000 plus the byte position for a path that is not normalized.

The manifest is `compiler.pipeline`'s (`spec/core/c-backend.md`,
ESP-013.D). Its status is `emitted`, `check failed` or `unsupported`.

## Diagnostics

The diagnostics file holds what stage0's `argorixc core-emit-c` prints after
`Error: ` for the same files, byte for byte.

- **A package that does not check.** Stage1 reports the diagnostics of one
  file: the root when it does not lex or parse, otherwise the first module,
  in link order, that does not link or check. Each diagnostic is rendered as
  `CoreDiagnostic::render` renders it:

  ```text
  src/helper.argx:5:24: semantic[TypeMismatch]: expected `u32`, found `u8`
    |
    5 |     let doubled: u32 = value * 2u8;
    |                        ^^^^^^^^^^^
  ```

  The line and column count as the lexer counts them: lines by LF, columns
  by character. The line number takes three columns, right-aligned. The
  carets cover the span, at least one and no more than the line has left
  after the column. Diagnostics are separated by a blank line.
- **A root declared twice.** This is the one message of stage1's own:
  `path: module `name` is declared by more than one file`. Stage0 names a
  directory there, and stage1 reads files, not directories.
- **A package the backend refuses.** The file holds
  `CBackendUnsupported: <reason>`, with stage0's reason
  (`compiler.c_emit.refusal`).

The messages are those of stage0's checker, linker and backend, including:

- types displayed as `Ty::display` does: the linked `a__b__Name` of a type
  from another module;
- names sorted as stage0 sorts them;
- Rust's `{:?}` of a backend type where stage0 prints one.

`crates/argorixc/tests/link_differential.rs` compares the rendering with
stage0's over every package of the differential.

## Bootstrap

`bootstrap/stage1.py` builds stage1 in two steps:

1. `emit` (B0, needs stage0): `argorixc --stdlib stdlib core-emit-c
   compiler/main.argx` writes `stage1.c`, and `stage0.json` records stage0 and
   its toolchain.
2. `build` (B1, needs a C compiler and Python only):
   - compile `stage1` from `stage1.c` and the C1 runtime, with the profile's
     flags and larger step and buffer ceilings;
   - run it on the repository's `argorix.build`;
   - require that its C equals `stage1.c`, that its manifest is right, and
     that the C it wrote, compiled, builds the same three files again;
   - write `stage1-provenance.json`.

`--require-rust-free-host` makes `build` fail if `rustc`, `cargo` or `rustup`
exists. It fails in any case if an executable loads a Rust library. CI runs
`build` that way in a clean Debian container, with gcc and with clang.

The provenance records `bootstrap_dependency: rust`: stage0 is written in
Rust and produced `stage1.c`. That stays in the trusted base until ESP-017
(`spec/provenance.md`, P1-02). Stage2 and stage3 without stage0, with
binary identity, are ESP-015.
