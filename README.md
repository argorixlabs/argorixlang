# Argorix Lang

Argorix Lang is a compiled language for secure, verifiable communication
between AI agents. Rust bootstraps the compiler; Argorix Lang remains its own
language with a path toward progressive self-hosting.

Version 0.6 adds controlled per-agent state and the `facu` and `marron`
runtime intrinsics on top of the reactive mailbox runtime:

```text
.argx
  -> lexer / parser / AST
  -> semantic and security verification
  -> Argorix IR 0.6
  -> Argorix Bytecode 0.5
  -> Argorix VM
  -> agent mailboxes
  -> deterministic scheduler
  -> reactive handlers
  -> agent state and causal guards
  -> trace ledger
```

The VM does not call LLMs, tools, MCP, A2A, networks, shells, or other external
systems. It validates bytecode and simulates protocol message flow only.

## Requirements

- Stable Rust toolchain
- Cargo

## Build and verify

```bash
cargo fmt
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Compiler commands:

```bash
cargo run -p argorixc -- check examples/prompt_defense_v02.argx
cargo run -p argorixc -- emit-ir examples/prompt_defense_v02.argx
cargo run -p argorixc -- graph examples/prompt_defense_v02.argx
cargo run -p argorixc -- capabilities examples/prompt_defense_v02.argx
cargo run -p argorixc -- emit-bytecode examples/prompt_defense_v02.argx
cargo run -p argorixc -- verify-bytecode examples/prompt_defense_v02.argx
cargo run -p argorixc -- check examples/prompt_defense_v05.argx
cargo run -p argorixc -- emit-ir examples/prompt_defense_v05.argx
cargo run -p argorixc -- emit-bytecode examples/prompt_defense_v05.argx
```

VM commands:

```bash
cargo run -p argorix-vm -- run examples/prompt_defense.argbc.json --dry-run
cargo run -p argorix-vm -- run examples/prompt_defense.argbc.json --dry-run --json
cargo run -p argorix-vm -- run examples/prompt_defense.argbc.json --dry-run --mailboxes
cargo run -p argorix-vm -- run examples/prompt_defense_v05.argbc.json --dry-run --reactive --inject User:PromptScanner:tell:UserPrompt
cargo run -p argorix-vm -- run examples/prompt_defense_v05.argbc.json --dry-run --reactive --inject User:PromptScanner:tell:UserPrompt --json
cargo run -p argorixc -- check examples/prompt_defense_v06.argx
cargo run -p argorixc -- emit-ir examples/prompt_defense_v06.argx
cargo run -p argorixc -- emit-bytecode examples/prompt_defense_v06.argx
cargo run -p argorix-vm -- run examples/prompt_defense_v06.argbc.json --dry-run --reactive --inject User:PromptScanner:tell:UserPrompt --state
```

## Runtime intrinsics v0.6

Handlers may invoke two built-in runtime operations:

```argx
on Decision as decision {
    marron(decision)
    facu(decision)
    trace decision
    halt
}
```

`facu(binding)` requires `state.write`. It updates the agent's handled-message
metadata and creates a deterministic checkpoint containing the message ID,
message type, binding, and checkpoint index.

`marron(binding)` requires `runtime.guard`. It verifies that the current
envelope was delivered by the scheduler, belongs to the active handler, and
contains non-empty `id`, `from`, `to`, `act`, and `message_type` fields.
Failures transition the runtime to `failed` while retaining the trace ledger.

Only `facu` and `marron` are recognized. Both must use the exact binding
declared by the enclosing handler.

## Reactive handlers v0.5

Agents can react to received message types:

```argx
agent PromptScanner {
    receives UserPrompt
    sends Finding to PolicyJudge

    on UserPrompt as prompt {
        emit Finding to PolicyJudge
    }
}
```

Handlers support only `emit MessageType to AgentName`, `trace binding`, and
`halt`. The compiler verifies input types, matching `receives` and `sends`
contracts, destinations, trace bindings, duplicate handlers, and the
`runtime.halt` capability and approval policy.

Reactive execution requires an initial message in this format:

```text
--inject FROM:TO:ACT:MESSAGE_TYPE
```

Payloads are `{}` in v0.5. The scheduler delivers the injected envelope,
executes the matching handler, queues emitted messages, and repeats until
`halt` or until no pending messages remain.

## Bytecode

`argorix_bytecode` lowers validated IR into JSON-serializable bytecode:

```json
{
  "bytecode_version": "0.6",
  "language": "Argorix Lang",
  "module": "Argorix.Security",
  "agents": [],
  "capabilities": [],
  "instructions": [
    {
      "op": "SendMessage",
      "from": "PromptScanner",
      "to": "PolicyJudge",
      "act": "propose",
      "message_type": "Finding"
    },
    {
      "op": "End"
    }
  ]
}
```

The instruction model supports:

- `DeclareAgent`
- `DeclareCapability`
- `DeclareProtocol`
- `DeclareHandler`
- `EmitMessage`
- `TraceValue`
- `HandlerHalt`
- `EndHandler`
- `InvokeIntrinsic`
- `SendMessage`
- `RequireCapability`
- `RequireApproval`
- `Trace`
- `Halt`
- `End`

Lowering emits declarations and security requirements before protocol message
instructions. `Halt` is supported by the format and causes dry-run execution to
stop with an error; the compiler does not emit it for a valid protocol merely
because a capability happens to be named `runtime.halt`.

## Bytecode verification

The verifier requires:

- Bytecode version `0.6` for newly compiled programs. Versions `0.3` and `0.5`
  remain accepted for compatibility.
- At least one agent.
- At least one protocol or `SendMessage`.
- Complete, non-empty message fields.
- Known or explicitly external senders and receivers.
- Existing agents for approval and capability requirements.
- A final `End` instruction.

Allowed external entities remain `User`, `System`, `Runtime`, `Memory`, and
`Tool`.

## VM runtime

The VM verifies bytecode again before execution and initializes one FIFO
mailbox for every internal agent. The deterministic scheduler converts each
`SendMessage` into a serializable message envelope:

```json
{
  "id": "msg_001",
  "from": "User",
  "to": "PromptScanner",
  "act": "tell",
  "message_type": "UserPrompt",
  "payload": {}
}
```

Each internal message is scheduled, delivered to the receiver mailbox, and
processed in bytecode order. External entities do not receive internal
mailboxes. No network calls, tools, LLMs, or concurrent tasks are executed.

Text output:

```text
Argorix VM v0.6

Loaded bytecode: examples/prompt_defense.argbc.json
Execution mode: dry-run

Step 1: User --tell UserPrompt--> PromptScanner
Step 2: PromptScanner --propose Finding--> PolicyJudge
Step 3: PolicyJudge --commit Decision--> RuntimeGate

Security checks: passed
Trace: generated
Status: completed
```

With `--mailboxes`, the CLI shows initialization and the three scheduler phases
for each message. With `--json`, execution returns runtime state summaries and
the complete ledger:

```json
{
  "vm_version": "0.5",
  "status": "completed",
  "mode": "dry-run",
  "scheduler": "deterministic",
  "steps": [
    {
      "index": 1,
      "from": "User",
      "to": "PromptScanner",
      "act": "tell",
      "message_type": "UserPrompt",
      "status": "ok"
    }
  ],
  "mailboxes": [
    {
      "agent": "PromptScanner",
      "delivered": 1,
      "processed": 1
    }
  ],
  "events": [],
  "security_checks": "passed"
}
```

Runtime status progresses through `initialized`, `running`, and `completed`,
or transitions to `failed`. The public `RuntimeState` retains agents,
mailboxes, pending messages, completed-step count, status, and `TraceLedger`.
The ledger records `VmStarted`, declarations, message scheduling, delivery and
processing, then `VmCompleted` or `VmFailed`. Because the scheduler mutates a
caller-owned state, failure diagnostics do not discard the ledger.

Reactive JSON uses `vm_version: "0.6"` and
`mode: "reactive-dry-run"`. Each step records the agent, handled message,
emitted messages, traced bindings, and whether the handler halted execution.

In v0.6 the same JSON also includes `agent_state` summaries and an ordered
`intrinsics` ledger. Runtime state stores full `AgentState` and
`StateCheckpoint` records for internal use and testing.

## Source security model

Argorix v0.2 security remains enforced before bytecode generation:

- Capabilities have `safe`, `restricted`, or `dangerous` levels.
- Restricted and dangerous capabilities require `approval granted`.
- Every used capability must exist in the module registry.
- Protocol steps must match agent `sends` and `receives` contracts.

Registry-free v0.1 sources require explicit compatibility mode:

```bash
cargo run -p argorixc -- --legacy-capabilities check examples/prompt_defense.argx
```

## Workspace

```text
crates/argorixc          Source compiler CLI
crates/argorix_parser    Lexer, parser, AST, spans, diagnostics
crates/argorix_semantics Source-level security and protocol verifier
crates/argorix_ir        Argorix IR 0.6 with handlers and intrinsics
crates/argorix_bytecode  IR lowering and Bytecode 0.3/0.5/0.6 verifier
crates/argorix_vm        Linear/reactive schedulers, mailboxes, VM, ledger
crates/argorix-vm        Bytecode VM CLI
examples                 Source and bytecode fixtures
tests                    End-to-end compiler tests
```

## Examples

- `prompt_defense_v02.argx`: valid secure source program.
- `prompt_defense_v05.argx`: valid reactive source program.
- `prompt_defense_v05.argbc.json`: generated reactive Bytecode 0.5.
- `prompt_defense_v06.argx`: reactive program with state and causal guards.
- `prompt_defense_v06.argbc.json`: generated Bytecode 0.6 fixture.
- `prompt_defense.argbc.json`: generated valid bytecode.
- `invalid_bytecode_missing_end.argbc.json`: verifier failure fixture.
- `restricted_without_approval.argx`: source approval failure.
- `unknown_capability.argx`: undeclared capability failure.

## Roadmap

1. v0.1: compiled structure
2. v0.2: compiled security
3. v0.3: compiled execution through bytecode and dry-run VM
4. v0.4: agent mailboxes, deterministic scheduling, runtime state, trace ledger
5. v0.5: declarative handlers and reactive dry-run execution
6. v0.6: controlled agent state, deterministic checkpoints, causal guards
7. Sandboxed capability providers and cryptographic identities
8. Optional WASM/native backends
9. Progressive self-hosting in Argorix Lang

> Rust is the forge. Argorix Lang is the sword.
