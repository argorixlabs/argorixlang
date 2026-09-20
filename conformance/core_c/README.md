# Core C execution harness (ESP-008.R)

Runs the Argorix Core runtime cases through the transitional C backend and
checks that the resulting executables do not depend on Rust. It is a subtask of
ESP-008, split by file with Codex (see `coordination/codex.md` and
`coordination/claude.md`). The emitter, the C1 runtime and the cases belong to
ESP-008; this directory holds the data, and
[`crates/argorix_core_c`](../../crates/argorix_core_c) holds the harness that
executes and inspects them.

The harness is the `core-c-harness` binary. It uses only the crates the
workspace already depends on, reads ELF itself instead of calling `readelf`,
and never starts a shell.

## What it checks

For every case in `tests/selfhost/runtime/cases.json`:

1. **Emit**: `argorixc core-emit-c` writes C. It runs twice and the two outputs
   must be byte-identical.
2. **Compile**: with a compiler from `allowed_compiler_names` in
   `bootstrap/c/toolchain.json` (or an explicit absolute path), using that
   file's `required_flags` and C1 runtime sources. The command is an argument
   vector, so case contents never become arguments, and arguments matching
   `forbidden_link_inputs` are refused.
3. **Execute**: stdin closed, with a timeout. stdout, stderr and exit status
   must equal the case's expectations exactly, after removing one trailing
   newline.
4. **Inspect** the ELF binary against [`policy.json`](policy.json):
   - only `libc.so.6` may be `NEEDED`;
   - no Rust symbols (`rust_*`, `__rust*`, and the `_ZN`/`_R` manglings);
   - no imported process or loader functions (`system`, `exec*`, `fork`,
     `posix_spawn`, `dlopen`, …), so the program cannot start `cargo` or
     `rustc`;
   - no embedded Rust toolchain paths (`/rustc/`, `/.cargo/`, …).

A policy pattern is a plain substring, or anchored with `^` (prefix), `$`
(suffix) or both (exact match). There is no regular-expression engine to
misread.

Four **negative controls** must also fire, or the report cannot pass. Without
them, a check that never fails would still look green:

- the comparator must reject a mutated expectation;
- the inspector must flag a sensor binary that imports `system`;
- the inspector must flag a sensor binary that defines `__rust_alloc`;
- the inspector must flag a sensor binary linked against `librust_sensor.so`.

Sensor binaries are compiled but never executed.

## Usage

```sh
cargo build -p argorixc -p argorix-core-c

# Everything in one place, as CI does:
./target/debug/core-c-harness all --argorixc target/debug/argorixc --cc gcc

# Or split across hosts, to execute where no Rust toolchain exists:
./target/debug/core-c-harness emit --argorixc target/debug/argorixc --bundle target/core-c/bundle
./target/debug/core-c-harness run --bundle target/core-c/bundle --cc gcc --require-rust-free-host

# Unit tests (no C compiler needed):
cargo test -p argorix-core-c
```

The bundle (`bundle.json` plus one `.c` per case) records the `argorixc`
binary's SHA-256 and version, the emitting commit and host, and the hash of the
cases manifest. `run` refuses a bundle emitted from a different `cases.json`,
and fails a case whose C file no longer matches its hash.

`--require-rust-free-host` makes `run` fail if `rustc`, `cargo` or `rustup` is
on `PATH`. Exit status: `0` all passed, `1` a case or control failed, `2`
configuration error.

The report (default `target/core-c/report.json`) records both hosts, the
compiler version, input hashes, compile argument vectors, observed output,
dependency findings and control results. `overall_pass` is true only if every
case passes and every negative control is detected.

## Known-gap corpus

`gaps/` holds valid Core programs that the backend gets wrong today, recorded
in [`gaps/gaps.json`](gaps/gaps.json) with the defect each one reproduces
(issue #27). Every program is accepted by `argorixc core-check`, and the
expected output comes from `spec/core/evaluation.md`.

```sh
./target/debug/core-c-harness gaps --argorixc target/debug/argorixc --cc gcc
```

| Status | Meaning | Effect |
| --- | --- | --- |
| `STILL_OPEN` | the recorded defect is reproduced | expected; exit 0 |
| `FIXED` | it now behaves as the spec requires | reported as a notice; promote it into `tests/selfhost/runtime/cases.json` (Codex lane) and remove it from `gaps.json` |
| `CHANGED` | it fails in a different way | **exit 1**: the record is stale and someone must look |

A fixed gap is not a build failure, so fixing the backend never breaks CI. Only
an unexplained change does. The corpus is not a substitute for the runtime
cases: it tracks what does not work yet.

## Differential testing against the spec

`core-c-harness generate` builds random Core programs and computes what each
one must print by evaluating its AST against `spec/core/evaluation.md` and
`spec/core/types.md`. That evaluator never calls `argorixc`, the C backend or
the C runtime, so it is an independent oracle, as the master plan requires of
any acceptance test.

```sh
./target/debug/core-c-harness generate --seed 7 --count 60 --out target/core-c/fuzz
./target/debug/core-c-harness all --argorixc target/debug/argorixc --cc gcc \
    --cases target/core-c/fuzz/cases.json --bundle target/core-c/fuzz-bundle
```

The output is an ordinary case corpus, so emission, compilation, execution,
comparison and dependency inspection all reuse the pipeline above. A seed
reproduces its corpus exactly.

The generator deliberately stays inside the subset the backend claims to
support, avoiding every shape recorded in `gaps/`: no shadowing, no block
operands, no shifts, no signed negation, no `if` statements, no unused locals,
parameters or functions, no recursion, and no `if` expression anywhere inside a
comparison operand. Aggregates stay flat and at function level for the same
reason: nested arrays, structs holding structs, arrays of structs and arrays
declared inside a block are gaps g01, g02, g03 and g06. It also avoids shapes that only upset the C compiler's
warning profile, such as `unsigned < 0` or a literal `^` pair that GCC reads as
a mistyped power. **A failure is therefore a real divergence between the
backend and the specification, not a known gap.**

Coverage: exact-width arithmetic with overflow traps, trapping division by zero
and `MIN / -1`, truncating `/` and `%`, bitwise operators, comparisons,
`&&`/`||` short-circuiting, `if` expressions, `while` loops with counters,
multi-argument calls, mutation through assignment and compound assignment,
fixed arrays read through a `u64` index that is sometimes past the end (which
must trap `INDEX_OUT_OF_BOUNDS`), and reads of flat struct fields — across all
eight integer widths.

## Sanitized runs

`--sanitize` adds `-fsanitize=address,undefined -fno-sanitize-recover=all -g`
on top of the declared profile and runs with `ASAN_OPTIONS=detect_leaks=1`. Any
`LeakSanitizer`, `AddressSanitizer:` or `runtime error:` line fails the case,
whatever its exit status.

```sh
./target/debug/core-c-harness all --argorixc target/debug/argorixc --cc gcc --sanitize
```

This is what catches what a plain run cannot see. Every `Buffer` leaked its
storage for a while — the emitter never called `argorix_buffer_drop` — and the
ordinary run stayed green throughout, because the program still printed the
right answer and exited 0.

Dependency inspection is skipped in this mode: a sanitized binary links
`libasan` and friends, so the policy does not apply to it. The ordinary run is
what enforces dependencies.

**On a host with high ASLR entropy** (`vm.mmap_rnd_bits` of 32, as on the WSL2
kernel behind Docker Desktop) AddressSanitizer intermittently hangs instead of
starting, on a random subset of programs. It is not a defect in the program
under test. Run the harness under `setarch $(uname -m) -R` there; GitHub's
runners are unaffected. A hang is reported as a timeout with that hint.

## CI

`.github/workflows/core-c.yml` runs the harness unit tests, then emits a bundle
on Ubuntu and executes it with GCC and Clang, and again with GCC inside a
`debian:stable-slim` container that has no Rust installed. Separate jobs check
the known-gap corpus, run the differential seeds, run every case under the
sanitizers, and execute with GCC 12 (an older compiler than the runner's,
because the C1 runtime once failed to build there), uploading each report so a
failure can be reproduced exactly.

The execution jobs are skipped only while **both** `argorixc core-emit-c` and
the cases manifest are absent, as on `main` before ESP-008 landed; if only one
of the two exists, the job fails.

## Limits

- Linux ELF only. There is no Windows (PE) or macOS (Mach-O) inspection, so the
  second target of ESP-017 needs its own inspector.
- The inspector checks the dynamic dependencies and symbols of the binary as
  built. A statically linked Rust object with stripped symbols and no toolchain
  paths would not be caught; the compile-argument check and the fixed runtime
  source list are what exclude it.
- The Rust-free job proves the executables and the C build need no Rust
  toolchain. The harness binary itself is built from Rust beforehand and copied
  in, exactly like `argorixc`: this is stage0 tooling, not evidence of
  toolchain independence (ESP-024).
- The generator covers scalars only. Arrays, structs, enums, buffers, arenas,
  bytes and UTF-8 are not generated yet.
