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

A fifth joins them with `--repeat` above 1: the repeated run must see a
difference in a sensor that prints its own process id.

Sensor binaries are compiled and never executed, except that one: a check on
repeated execution has nothing to prove without running something twice.

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
in [`gaps/gaps.json`](gaps/gaps.json) with the defect each one reproduces.
Every program is accepted by `argorixc core-check`, and the expected output
comes from `spec/core/evaluation.md`.

The sixteen of issue #27 were fixed in PR #37 and the seven of
[issue #39](https://github.com/argorixlabs/argorixlang/issues/39) (g17–g23:
`loop` as a statement, `loop` with a value break, `match` on a bool, a module
`const`, `match` on an integer with `_`, a match guard, and `x = x;`) in
ESP-009.F. All twenty-three now live in `regression/`, so the gap corpus is
empty until the next defect is found.

A gap can belong to one compiler: `x = x;` lowers to a C self-assignment,
which clang rejects under the declared `-Werror` profile and GCC does not
diagnose at all. Such a record carries a `compilers` list, and a run with any
other compiler reports it `SKIPPED` instead of announcing it fixed. The CI job
runs the corpus with both.

```sh
./target/debug/core-c-harness gaps --argorixc target/debug/argorixc --cc gcc
```

| Status | Meaning | Effect |
| --- | --- | --- |
| `STILL_OPEN` | the recorded defect is reproduced | expected; exit 0 |
| `FIXED` | it now behaves as the spec requires | reported as a notice; promote it into `tests/selfhost/runtime/cases.json` (Codex lane) and remove it from `gaps.json` |
| `CHANGED` | it fails in a different way | **exit 1**: the record is stale and someone must look |
| `SKIPPED` | it is recorded for another compiler | nothing is run for it; exit 0 |

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
support, avoiding every shape recorded in `gaps/`. What it avoids today is
short: an aggregate declared inside an `if` or a loop body, which is gap g06,
and unused locals, parameters or functions, which are valid Core whose C fails
the `-Werror` profile. It also avoids shapes that only upset the C compiler's
warning profile, such as `unsigned < 0`, a literal `^` pair that GCC reads as
a mistyped power, or `x = x`, whose C clang rejects as a self-assignment
(gap g23, found by this generator in 2 of 120 programs). **A failure is therefore a real divergence between the
backend and the specification, not a known gap.**

The sixteen defects of issue #27 were fixed in PR #37, so the shapes the
generator used to steer around are now generated on purpose: shifts, signed
negation, `if` used as a statement, blocks whose local shadows an outer name,
`if` expressions inside comparison operands and inside aggregate literals, and
the nested aggregates that were gaps g01, g02 and g03. Alongside them the
generator gained the control flow the backend already supported and no
generated program had used: `else if` chains, `return` inside a branch,
`continue` and `break` inside a loop, one nested loop, and a function that
calls itself.

One shape is spelt a particular way for a reason. A final `if` statement is
rendered with the `;` that `expression_stmt` spells out in the grammar,
because the frontend parses `if c { .. } (tail)` as a call of the `if` and
rejects the program; the `;`-less form is still generated wherever another
statement follows it. That divergence is [issue #38](https://github.com/argorixlabs/argorixlang/issues/38),
not a silent workaround.

Coverage: exact-width arithmetic with overflow traps, trapping division by zero
and `MIN / -1`, truncating `/` and `%`, bitwise operators, comparisons,
`&&`/`||` short-circuiting, `if` expressions, `while` loops with counters,
multi-argument calls, mutation through assignment and compound assignment,
fixed arrays read through a `u64` index that is sometimes past the end (which
must trap `INDEX_OUT_OF_BOUNDS`), reads of flat struct fields, buffers filled
by `push` and read the same way, and arenas with one allocation read through a
handle — sometimes after `release()`, which must trap `ARENA_RELEASED` — and
enums read back through a function whose body is an exhaustive `match`, with
both a variant that carries a field and a fieldless one. Since ESP-009.C it
also covers `<<` and `>>` with amounts inside and past the width (which must
trap `SHIFT_OUT_OF_RANGE`), signed negation including the minimum (which must
trap `INTEGER_OVERFLOW`), `if` used as a statement with and without `else`,
block expressions whose local shadows an outer name, nested arrays read
through two `u64` indexes that may each point past the end, structs that hold
structs read through `q.f0.f1`, and arrays of structs read through
`t[i].f0`. Control flow covers `else if` chains of up to three links, an
early `return`, `continue` and `break` after a loop's counter has advanced, a
nested counted loop, and a self-recursive function whose literal argument
always reaches its base case. All of it across all eight integer widths.

Since ESP-009.D it also covers the parts of the standard library that already
execute:

- **Bytes and UTF-8.** A fixed byte array, `as_bytes()`,
  `decode_utf8_or_trap()` and the byte length `spec/core/stdlib.md` gives
  `stdlib.text`. Sequences are drawn both from random code points across all
  four widths and from a table of the ends a validator gets wrong — the
  shortest and longest form of each width, both ends of the surrogate block,
  the overlong encodings, and the first value past U+10FFFF — and half the
  time a mistake is planted on top: a truncated sequence, a lone
  continuation, or a continuation byte that is not one. The oracle validates
  from Table 3-7 of the Unicode Standard, not from the runtime, and an
  invalid sequence must trap `UTF8_INVALID`. Only a `u64` program carries it,
  because `length()` is a `u64` and Core has no casts.
- **Resource ceilings.** A `Buffer<u64>` filled until it passes the profile's
  byte ceiling and an arena allocated until its slots run out, both trapping
  `RESOURCE_LIMIT`. The oracle models the rule from the spec — capacity
  doubles from four elements; a push that would pass the ceiling traps
  instead of growing — with the numbers
  `crates/argorix_ir/src/core_c.rs` compiles into every program (1 MiB and
  1024 slots). If the profile changes, this corpus has to change with it.

Running the generated corpus with `--sanitize` is worth doing for the memory
constructs in particular: it is the combination that would have caught the
`Buffer` leak on the first run.

## Repeated runs

`spec/core/stdlib.md` requires that "repeated execution with gcc and clang
produce byte-identical output". `--repeat N` runs each executable N times and
fails the case if any run differs from the first in stdout, stderr or exit
status.

```sh
./target/debug/core-c-harness run --bundle target/core-c/bundle --cc gcc --repeat 3
```

With `--repeat` above 1 the run adds a negative control of its own,
`repeat_detects_nondeterminism`: a sensor that prints its own process id, so
the check has something it must catch. A run whose controls are enabled fails
if that sensor comes back identical twice.

Identity *between* compilers needs no separate check: every case declares its
expected output, so gcc and clang agreeing with the manifest is the same as
agreeing with each other.

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
- The generator does not produce `Slice`, string comparison or escaping,
  or an aggregate declared inside an `if` or a loop body (gap g06). Programs
  of more than one module are covered by the regression corpus
  (`m01_imports`, `m02_shared_types`), not by the generator. Handle mutation
  and arena slot reuse are exercised by the runtime cases.
- The arena's byte ceiling is modelled but unreachable from a generated
  program: its slot limit of 1024 binds first for every element the
  generator builds. Only the slot path is exercised end to end.
- The oracle evaluates on the host stack, so it stops at `MAX_CALL_DEPTH`
  (200 frames) and marks a deeper program unusable. The runtime's own limit
  is far higher, so a program that recurses between those two depths is
  skipped rather than compared.
