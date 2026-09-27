# Building the Argorix compiler natively

The compiler in `compiler/` builds itself to native x86-64 code on Linux
(ESP-016) and on Windows (ESP-017). This guide covers building it on each
system, what each step needs, and what it checks. The specification is
`spec/core/native-x86-64.md` and the inventory is `toolchain.json`.

Each system has two matrices, and each ends in a native compiler that
writes itself byte for byte:

| System | Seed (a C compiler, once) | Bootstrap (no C compiler, no Rust) |
| --- | --- | --- |
| Linux x86-64 | `bootstrap/native.py seed`: gcc or clang | `bootstrap/native.py bootstrap`: `ld`, the C library, Python |
| Windows x86-64 | `bootstrap/native-windows.ps1 seed`: MSVC `cl` | `bootstrap/native-windows.ps1 bootstrap`: `link.exe` and the C library |

Digests are compared within one system and one fixed environment, never
across systems.

## Linux

In a Rust environment, stage0 writes the compiler's C:

```sh
cargo build -p argorixc
python3 bootstrap/stage1.py emit --argorixc target/debug/argorixc --out target/stage1
```

With a C compiler, the shim and stage1 are built, and stage1 writes the
seed object:

```sh
python3 bootstrap/native.py seed --from target/stage1 --cc gcc --out target/native
```

With only binutils, the C library and Python (the CI job runs this in a
container with no network):

```sh
python3 bootstrap/native.py bootstrap --out target/native \
    --require-no-c-compiler --require-rust-free-host --require-no-network
```

The result is `target/native/native3/argorixc`, with its report in
`target/native/native-report.json`.

## Windows

From an MSVC developer prompt ("x64 Native Tools", or `vcvars64.bat`) with
Rust installed:

```powershell
cargo build -p argorixc
./bootstrap/native-windows.ps1 seed -Argorixc target/debug/argorixc.exe -Out target/native-windows
./bootstrap/native-windows.ps1 linker -Out target/native-windows
```

`linker` copies `link.exe` and the libraries it loads to
`target/native-windows/linker`. The bootstrap then runs with only that
linker and the system directory on the path:

```powershell
$out = (Resolve-Path target/native-windows).Path
$env:LIB = (Get-Content -Raw "$out/linker.json" | ConvertFrom-Json).lib
$env:PATH = "$out\linker;$env:SystemRoot\System32;$env:SystemRoot\System32\WindowsPowerShell\v1.0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File bootstrap/native-windows.ps1 bootstrap -Out $out -RequireNoCCompiler -RequireRustFreeHost
```

The result is `target/native-windows/native3/argorixc.exe`, with its report
in `target/native-windows/native-report.json`.

## Using the native compiler

A build is described by `argorix.build` at the package root
(`compiler/main.argx`). For a native program, use `object` in place of `c`,
and on Windows add `target x86_64-windows`:

```text
argorix-build 1
root main.argx
object program.obj
target x86_64-windows
manifest program.json
diagnostics diagnostics.txt
```

Run the compiler with the package and build roots and their byte budgets:

```text
argorixc --package-root <package> --read-budget <bytes> --build-root <build> --write-budget <bytes>
```

It prints `ARGORIX_RESULT:0` when it wrote the object. Then link the object
with the shim, using the link line of the specification for the system.

## When something fails

- **A trap:** the program writes the trap's code on standard error and
  exits with 70.
- **A refusal:** the build's diagnostics file says why the package did not
  check or the backend refused it.
- **The machine code:** on Linux, `objdump -d` and `nm` on the object. On
  Windows, `dumpbin /disasm /symbols` on the object, and `link /MAP:<file>`
  to see where each function landed. There is no debug information: a
  debugger sees `main`, the shim's functions and addresses.
