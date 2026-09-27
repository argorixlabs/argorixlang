# Agent bytecode verification dump (ESP-018.D.3)

`argorixc agent-verify <file>` is the stage0 oracle for serialized bytecode.
`compiler.agent_verify.dump` emits the same UTF-8 bytes for a supported input:
`ok\n`, one diagnostic per line, or `invalid bytecode JSON: ...` with the
serde_json line and byte column. Validation preserves the order of the stage0
checks. A declaration of an external provider or trust boundary is metadata;
verification does not execute it or authenticate it.

The differential in `crates/argorixc/tests/agent_verify_differential.rs`
compiles `tests/selfhost/agent/verify_files.argx` through the transitional C
backend and compares its output to stage0. The current corpus has 891 inputs,
including eight mutations of v0.34, v0.35, v0.36 and v1.0 metadata. Every
input must be decided and match byte for byte. The test runs on Unix with a C
compiler; Windows builds the Rust tests but does not run this differential.

The JSON reader conservatively emits `unsupported` for forms whose exact
serde_json diagnostic spelling has not been reproduced: noncanonical float
diagnostics, integers past u64 or outside f64, struct representations as
arrays, and Unicode Debug formatting outside printable Latin-1. The corpus
contains none of these undecided forms. A future corpus addition that reaches
one fails the differential; it must be implemented or recorded as an explicit
compatibility decision before the acceptance gate is raised.

Passing this dump comparison establishes the tested compiler boundary. It
does not establish runtime policy enforcement, live provider execution, or
production maturity.
