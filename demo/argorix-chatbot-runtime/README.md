# ArgorixLang Chatbot Runtime Demo

A technical demo that proves **Argorix Lang v1.0 is a *governed runtime*, not an
OpenAI wrapper**. A chatbot turn only reaches an external provider after Argorix
has compiled a contract, verified its bytecode, generated an evidence bundle and
a security report, and returned a **fail-closed** governed decision.

It exercises the real v1.0 surface:

- `runtime_execution_profile` (`ChatbotRuntime`)
- `sandboxed_provider_adapter` (`OpenAISandbox`)
- agent passport (`AssistantPassport`)
- ATrust evidence map (`ChatbotEvidenceMap`)
- governance profile (`ChatbotGovernance`)
- runtime hardening + threat model (`ChatbotRuntimeHardening`, `ChatbotThreatModel`)
- `fail_closed true`, `network declared_only`, `external_execution sandboxed`
- `secret_ref "env:OPENAI_API_KEY"` — surfaced only as a **redacted reference**
- trace, security report and evidence bundle artifacts

---

## How it works

```
POST /api/chat
  1. argorixc check          session.argx          (syntax + semantics)
  2. argorixc emit-bytecode  -> session.argbc.json
  3. argorixc verify-bytecode
  4. argorix-vm run --reactive --inject … --security-report --trace-out --evidence-bundle
        -> session.security.json, session.trace.json, session.evidence.json
        -> argorix-vm verify-evidence (integrity check)
  5. argorix-vm run --runtime ChatbotRuntime --adapter OpenAISandbox
                   --operation responses.create [--sandboxed-external]
        -> governed decision: "blocked" or "planned"
```

**Decision matrix**

| Condition | UI status |
|---|---|
| Any of steps 1–3 fail | `blocked` |
| `ARGORIX_SANDBOXED_EXTERNAL=false` (default) | `planned` — auditable plan, **no network** |
| `=true` but governed decision ≠ `planned` | `blocked` |
| `=true`, planned, no `OPENAI_API_KEY` | `simulated` |
| `=true`, planned, key present | `sandboxed_external` — real call |

The Argorix VM core itself **never makes a network call** — the most it ever
does is *plan* one. The actual OpenAI request is performed server-side by
`lib/openai/callOpenAI.ts`, and only after Argorix returns `planned`.

---

## Install

Prerequisites: Node.js ≥ 18.18 and a Rust toolchain.

### 1. Build the Argorix binaries (from the repo root)

```bash
cd ../../
cargo build --workspace
```

This produces `target/debug/argorixc.exe` and `target/debug/argorix-vm.exe`,
which the demo invokes (paths are configurable in `.env.local`).

### 2. Install and configure the demo

```bash
cd demo/argorix-chatbot-runtime
npm install
cp .env.example .env.local   # then edit .env.local
```

---

## Run

```bash
npm run dev
# open http://localhost:3000
```

### Plan-only mode (default — no OpenAI)

With `ARGORIX_SANDBOXED_EXTERNAL=false`, the demo validates the contract and
returns an **auditable plan + evidence**, but never contacts any provider. No
API key is required.

Smoke test the API directly:

```bash
curl -s localhost:3000/api/chat -H 'content-type: application/json' \
  -d '{"message":"hello"}' | jq '.argorix.status'   # => "planned"
```

### Enable the sandboxed external call

```dotenv
# .env.local
ARGORIX_SANDBOXED_EXTERNAL=true
OPENAI_API_KEY=sk-...          # server-side only, never committed
OPENAI_MODEL=gpt-5.2
OPENAI_BASE_URL=https://api.openai.com/v1
```

Restart `npm run dev`. Now a valid turn yields status `sandboxed_external` and a
real provider answer. With `ARGORIX_SANDBOXED_EXTERNAL=true` but no key set, you
get status `simulated` instead.

---

## Why the API key is never in the frontend

- The key is read **only** in `lib/openai/callOpenAI.ts`, a server module
  imported solely by the `/api/chat` route handler (`runtime = "nodejs"`).
- There is no `NEXT_PUBLIC_OPENAI_API_KEY`; nothing prefixed `NEXT_PUBLIC_` is
  ever introduced, so the value cannot be inlined into client bundles.
- Argorix only ever sees the **reference** `env:OPENAI_API_KEY`. In the compiled
  bytecode the adapter carries `secret_value: null` and `redacted: true`.
- All CLI output is passed through `redact()` (`lib/argorix/parseArgorixOutput.ts`)
  before it can reach the response — the live key value and `sk-…` / `Bearer …`
  shapes are stripped.
- The key is never written to a trace, security report or evidence bundle.

`.env`, `.env.local` and `generated/*` are git-ignored.

## Why this does not break the Argorix core

- No core crate is modified. The demo only **invokes** the existing
  `argorixc` / `argorix-vm` binaries as black boxes.
- The contract `argorix/chatbot-runtime.argx` is derived from the shipped
  `examples/runtime_mvp_v100.argx` (renamed entities, `secret_ref` pointed at
  `env:OPENAI_API_KEY`). It compiles and verifies with the stock v1.0 toolchain.
- No real MCP runtime, no real A2A runtime, no external tool execution, no free
  shell — those remain contracts/declarations only, exactly as v1.0 intends.

## What evidence Argorix generates (per request, under `generated/<requestId>/`)

- `session.argx` — the governance contract used for this turn
- `session.argbc.json` — compiled, verified bytecode
- `session.security.json` — security report (verdict, severity, runtime/adapter posture)
- `session.trace.json` — execution trace
- `session.evidence.json` — evidence bundle with `sha256` digests, verified by
  `argorix-vm verify-evidence`

## What "fail-closed" means here

The runtime denies by default. If governance validation fails, if the governed
decision is anything other than `planned`, or if the sandboxed call errors, the
demo returns `blocked` and **no provider is contacted**. External execution is
only ever *enabled* by an explicit `--sandboxed-external` planning step plus an
explicit `ARGORIX_SANDBOXED_EXTERNAL=true` operator opt-in.

---

## Layout

```
argorix/chatbot-runtime.argx     Argorix v1.0 governance contract
app/page.tsx                     UI shell
app/api/chat/route.ts            governed pipeline (server, Node runtime)
components/                      ArgorixBadge, ChatWindow, Runtime/Passport/Evidence/Security panels
lib/argorix/                     runArgorix, buildSessionContract, parseArgorixOutput
lib/openai/callOpenAI.ts         server-only sandboxed provider adapter
generated/                       per-request artifacts (git-ignored)
```
