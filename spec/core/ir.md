# Argorix Core IR 0.1

Status: normative bootstrap contract for ESP-007. The Rust implementation in
`argorix_ir::core` is stage0 and must be replaced by Argorix sources in ESP-013.

## Boundary

Core IR is a typed, structured, serializable representation of Core 0.1. It is
not the historical agent IR and it is not Argorix Bytecode. A document carries
both `ir_version: "0.1"` and `core_version: "0.1"`; readers reject unknown
values instead of guessing compatibility.

The lowering boundary accepts only a `VerifiedCoreProgram`. The future backend
boundary accepts only `VerifiedCoreIr`, an opaque proof wrapper created by the
verifier. Deserializing JSON produces raw `CoreIrProgram`, never verified IR.

## Program and operations

A program records its module, imports, locked module set, allowed effects, and
items. Items are functions, structs, enums, and constants. Function bodies use
structured blocks and statements for bindings, assignments, loops, branches,
returns, calls, aggregates, indexing, field access, unary/binary operations,
and exhaustive matches. Types preserve named fixed-width scalars and the Core
containers, including array length.

Structured control flow is intentional in 0.1: it makes lexical scopes and
loop legality explicit while remaining directly lowerable to a C or native
control-flow graph. ESP-008 owns that backend transformation and execution.

## Effects

`effect_policy` is an allow-list. The verifier derives used effects from the
program and rejects undeclared effects. Core 0.1 identifies memory reads,
memory writes, allocation, traps, and named host effects. The only named host
effects accepted are `package.read` and `build.write`, derived from functions
that take a `PackageRead` or `BuildWrite` capability (`stdlib.compiler_host`);
any other named host effect is rejected. Duplicate policy entries are invalid.

## Verification

Verification fails closed and checks:

1. IR and source-language versions;
2. unique effects and locked modules, declared effects, and import locks;
3. reconstruction into the normative Core type/control model;
4. names and callable references, type consistency, exhaustive patterns,
   assignment legality, and loop-control placement through the Core semantic
   verifier.

Only after every diagnostic set is empty is `VerifiedCoreIr` constructed. Its
semantic fingerprint is a SHA-256 digest, prefixed with `sha256:`, over the
canonical compact JSON serialization of the verified representation. Pretty
JSON roundtrips must reproduce that fingerprint.

## Serialization

The JSON shape is described by `core-ir.schema.json`. Serde is currently the
authoritative stage0 decoder; the schema documents the stable envelope and
tagged unions for independent implementations. Fields not present in the
contract are rejected by future strict decoders; compatibility changes require
an IR version change.

## Non-claims

Verified Core IR is not machine execution, C emission, a runtime, self-hosting,
or independence from Rust. Those properties require ESP-008 and later gates.
