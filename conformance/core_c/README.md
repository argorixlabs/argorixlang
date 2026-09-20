# Core C execution runner (ESP-008.R)

Runs the Argorix Core runtime cases through the transitional C backend and
checks that the resulting executables do not depend on Rust. It is a subtask of
ESP-008, split by file with Codex (see `coordination/codex.md` and
`coordination/claude.md`). The emitter, the C1 runtime, and the cases belong to
ESP-008; this directory only executes and inspects them.

## What it checks

For every case in `tests/selfhost/runtime/cases.json`:

1. **Emit**: `argorixc core-emit-c` writes C. It runs twice and the two outputs
   must be byte-identical.
2. **Compile**: with a compiler from `allowed_compiler_names` in
   `bootstrap/c/toolchain.json` (or an explicit absolute path), using that
   file's `required_flags` and C1 runtime sources. The command is an argument
   list; there is no shell, and case contents never become arguments.
   Arguments matching `forbidden_link_inputs` are refused.
3. **Execute**: stdin closed, with a timeout. stdout, stderr, and exit status
   must equal the case's expectations exactly, after removing one trailing
   newline.
4. **Inspect** the ELF binary with `readelf` against [`policy.json`](policy.json):
   - only `libc.so.6` may be `NEEDED`;
   - no Rust symbols (`rust_*`, `__rust_*`, legacy `_ZN…17h…E` and v0 `_R…`
     manglings);
   - no imported process or loader functions (`system`, `exec*`, `fork`,
     `posix_spawn`, `dlopen`, …), so the program cannot start `cargo` or
     `rustc`;
   - no embedded Rust toolchain paths (`/rustc/`, `/.cargo/`, …).

Four **negative controls** must also fire, or the report cannot pass. Without
them, a check that never fails would still look green:

- the comparator must reject a mutated expectation;
- the inspector must flag a sensor binary that imports `system`;
- the inspector must flag a sensor binary that defines `__rust_alloc`;
- the inspector must flag a sensor binary linked against `librust_sensor.so`.

Sensor binaries are compiled but never executed.

## Usage

```sh
# Everything in one place (CI does this):
python3 conformance/core_c/run.py all --argorixc target/debug/argorixc --cc gcc

# Or split across hosts, to execute where no Rust toolchain exists:
python3 conformance/core_c/run.py emit --argorixc target/debug/argorixc --bundle target/core-c/bundle
python3 conformance/core_c/run.py run --bundle target/core-c/bundle --cc gcc --require-rust-free-host

# Unit tests (no C compiler needed):
python3 -m unittest discover -s conformance/core_c -p "test_*.py"
```

The bundle (`bundle.json` plus one `.c` per case) records the `argorixc`
binary's SHA-256 and version, the emitting commit and host, and the hash of the
cases manifest. `run` refuses a bundle emitted from a different
`cases.json`, and fails a case whose C file no longer matches its hash.

`--require-rust-free-host` makes `run` fail if `rustc`, `cargo`, or `rustup` is
on `PATH`. Exit status: `0` all passed, `1` a case or control failed, `2`
configuration error.

The report (default `target/core-c/report.json`) records the host, compiler
version, input hashes, compile argument vectors, observed output, dependency
findings, and control results. `overall_pass` is true only if every case passes
and every negative control is detected.

## Known-gap corpus

`gaps/` holds valid Core programs that the backend gets wrong today, recorded in
[`gaps/gaps.json`](gaps/gaps.json) with the defect each one reproduces
(issue #27). Every program is accepted by `argorixc core-check`, and the
expected output comes from `spec/core/evaluation.md`.

```sh
python3 conformance/core_c/run.py gaps --argorixc target/debug/argorixc --cc gcc
```

Each gap is classified against its record:

| Status | Meaning | Effect |
| --- | --- | --- |
| `STILL_OPEN` | the recorded defect is reproduced | expected; exit 0 |
| `FIXED` | the program now behaves as the spec requires | reported as a notice; promote it into `tests/selfhost/runtime/cases.json` (Codex lane) and remove it from `gaps.json` |
| `CHANGED` | it fails in a different way | **exit 1**: the record is stale and someone must look |

A fixed gap is not a build failure, so fixing the backend never breaks CI. Only
an unexplained change does. The corpus is not a substitute for the runtime cases:
it tracks what does not work yet.

## CI

`.github/workflows/core-c.yml` runs the unit tests, then emits a bundle on
Ubuntu and executes it with GCC and Clang, and again with GCC inside a
`debian:stable-slim` container that has no Rust installed. A separate job checks
the known-gap corpus. The execution jobs
are skipped only while **both** `argorixc core-emit-c` and the cases manifest
are absent, as on `main` before ESP-008 lands; if only one of the two exists,
the job fails.

## Evidence

[`evidence/2026-09-19-wsl-gcc15-rust-free.json`](evidence/2026-09-19-wsl-gcc15-rust-free.json):
C emitted on Windows by `argorixc` built from `codex/c-backend-esp008` at
`2756e76`, then compiled and run with GCC 15.2.0 in WSL2 Debian with no
`rustc`, `cargo`, or `rustup` on the host. 3/3 cases passed, 4/4 negative
controls detected, and every binary depends only on `libc.so.6`.

## Limits

- Linux ELF only. There is no Windows (PE) or macOS (Mach-O) inspection, so the
  second target of ESP-017 needs its own inspector.
- `readelf` and the policy check the dynamic dependencies and symbols of the
  binary as built. A statically linked Rust object with stripped symbols
  and no toolchain paths would not be caught. The compile argument check and
  the fixed runtime source list are what prevent such an object from getting
  in.
- The runner is Python and `argorixc` is still Rust stage0. This shows that Core
  programs run without a Rust runtime, not that the toolchain is independent of
  Rust (ESP-024).
