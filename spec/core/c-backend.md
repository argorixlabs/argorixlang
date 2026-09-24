# Transitional C backend contract

Status: ESP-008 work in progress. This backend is a bootstrap bridge and is
not the native backend required for the final Argorix toolchain. Two
implementations follow this contract: the stage0 emitter in
`crates/argorix_ir/src/core_c.rs`, and `compiler/c_emit.argx` (ESP-013.C). The
Argorix one writes the same C byte for byte, and reproduces itself
(`crates/argorixc/tests/link_differential.rs`).

## Trusted boundary

The emitter implements `CoreIrBackend` and accepts only `VerifiedCoreIr`.
Deserialized or manually constructed `CoreIrProgram` values must pass the
ESP-007 verifier before emission. The emitter never repairs malformed IR.

## Deterministic lowering

- Every non-trivial Core operand is assigned to a generated temporary in
  source evaluation order before the enclosing operation is emitted.
- `&&` and `||` lower to explicit branches; they do not rely on C argument
  evaluation order.
- Fixed-width integer arithmetic calls runtime helpers that check overflow,
  division by zero, `MIN / -1`, and shift width before performing the C
  operation. Signed C overflow is never used as a check.
- Index and handle operations call bounds/generation helpers before access.
- Generated identifiers are derived from validated symbols plus deterministic
  numeric suffixes. Source strings never become compiler options or commands.

## Toolchain invocation

The build driver resolves an allow-listed compiler executable (`cc`, `clang`,
`gcc`, or explicitly configured absolute path), then invokes it directly with
an argument array. It never starts a shell and never concatenates source text
into a command line. The initial profile is C11 with warnings as errors and no
compiler-specific runtime dependency.

## Runtime ABI C1

`bootstrap/c/argorix_core_runtime.h` exposes the C1 bootstrap ABI. Traps write
exactly `ARGORIX_TRAP:<code>` to standard error and exit with status 70. Normal
test entrypoints write `ARGORIX_RESULT:<value>` to standard output. The step
budget is explicit state, not a process-global hidden counter.

The runtime may use the C standard library types and I/O needed by this
profile. It must not link Rust libraries, load Cargo artifacts, or invoke Rust.
Its canonical `argorix_handle` is 48 bytes and is checked against an explicit
arena view before access: arena identity and epoch, slot and generation, type,
range, permission, and reserved bytes all fail closed. C1 does not yet provide
the collection allocator or public `Arena<T>`/`Buffer<T>` construction API;
those belong to ESP-009.

## Evidence required to close ESP-008

Closure requires compilation and execution with a declared C compiler,
expected results and traps, binary dependency inspection, workspace regression,
and a recorded toolchain version. Emitted-C snapshots alone are insufficient.
