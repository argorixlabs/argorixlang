# ESP-018 compatibility matrix

The agent-language compiler in `compiler/` uses the Core toolchain to reproduce
the stage0 parser, semantic checker, module linker, IR, bytecode and serialized
bytecode verifier. The matrix below records the first historical version of
each declaration group. A version entry means the declaration is accepted and
verified at the compiler boundary; it does **not** grant a runtime effect.

The supported bytecode versions are `0.3`, `0.5` through `0.36`, and `1.0`.
`0.4` and unlisted versions remain unsupported. The bytecode version gates in
`crates/argorix_bytecode/src/bytecode.rs` are the source of truth if this table
and the implementation disagree.

| First version | Feature group | Compiler status | Runtime boundary |
| --- | --- | --- | --- |
| 0.3, 0.5–0.10 | Base agents, capabilities, instructions, assertions and failures | Stage0 and Argorix dumps match over the maintained corpus | No new external effect |
| 0.11–0.15 | Provider contracts and legacy compatibility | Same bytecode and verifier decisions | Provider use remains governed |
| 0.16 | Modules and imports | Same package graph and bytecode | No execution from an import alone |
| 0.17 | Policies | Same checks and bytecode | DENY, REVIEW and UNKNOWN keep their stage0 meaning |
| 0.18 | Typed messages, types and enums | Same checks and bytecode | No new effect |
| 0.19 | Agent passports | Same metadata and verifier decisions | Declaration is not identity proof |
| 0.20 | Provider harnesses | Same metadata and verifier decisions | Network and secret boundaries remain denied unless governed |
| 0.21 | Features and secrets | Same metadata and verifier decisions | Secret values are not embedded |
| 0.22 | Adapter framework | Same metadata and verifier decisions | No automatic dispatch |
| 0.23 | Adapter profiles | Same metadata and verifier decisions | No automatic dispatch |
| 0.24 | Crypto primitives | Same metadata and verifier decisions | Declaration is not cryptographic assurance |
| 0.25 | Crypto boundaries | Same metadata and verifier decisions | Key material remains denied |
| 0.26 | DID methods and A-Trust boundaries | Same source and bytecode representation | Declarative only |
| 0.27 | A-Trust identities | Same source and bytecode representation | No authenticated identity inferred |
| 0.28 | Credential contracts | Same version gate and bytecode | Declarative only |
| 0.29 | Handshakes | Same validation and bytecode | Dry-run metadata is not an authenticated handshake |
| 0.30 | Trust ledgers | Same validation and bytecode | Hash chain is not an immutability guarantee |
| 0.31 | MCP and A2A bridge contracts | Same validation and bytecode | Declaration is not a live connector |
| 0.32 | A-Trust evidence maps | Same validation and bytecode | Mapping is not external evidence |
| 0.33 | Governance and regulatory mappings | Same validation and bytecode | No legal approval or certification |
| 0.34 | Third-party verifiers and public conformance reports | Same validation and bytecode | Declared independence is not externally verified |
| 0.35 | Runtime hardening profiles and threat models | Same validation and bytecode | Boundaries remain denied by default |
| 0.36 | Spec freezes and release candidates | Same validation and bytecode | Release metadata enables no runtime |
| 1.0 | Runtime execution profiles and sandboxed provider adapters | Same validation and bytecode | External execution still requires the governed runtime path |

## Evidence and decision boundary

- The lexer, parser, semantic checker, package graph, IR and bytecode
  differentials cover the historical source corpus. The bytecode emitted from
  every checked program and package is byte-identical to stage0's in the
  tested corpus. See `tasks/espada/ESP-018.md` for the measured counts.
- The serialized bytecode verifier differential decides 891 inputs, including
  eight malformed metadata controls. It reports 891 identical dumps, zero
  differences and zero undecided inputs on Unix with a C compiler.
- `conformance/suite.v100.json` passes 137 of 137 cases on the existing
  stage0 VM. This suite exercises its policy and runtime contracts.
- Because the migrated compiler emits the same bytecode for the tested
  programs and the VM remains the same, the VM receives the same policy rules
  and instructions. Preservation of DENY, REVIEW and UNKNOWN here is an
  inference from bytecode identity and the stage0 VM contract. ESP-019 must
  test those outcomes again when the VM itself is migrated.

The matrix describes tested compiler compatibility. It does not claim that
every possible source or serialized JSON input is covered. The JSON reader's
conservative limits are in [verify.md](verify.md).
