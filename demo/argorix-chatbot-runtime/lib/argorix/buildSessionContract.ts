/**
 * buildSessionContract.ts
 * -----------------------
 * For each chat request we materialise a per-session copy of the base Argorix
 * contract plus its artifact paths, all under `generated/`.
 *
 * Why copy instead of mutate? The user's message is NEVER embedded into the
 * contract — doing so would let untrusted input influence policy. The message
 * only ever flows as the injected `UserPrompt` message at VM time (and, in
 * sandboxed_external mode, to the provider). The contract itself stays a fixed,
 * auditable governance artifact; we copy it per request purely so each request
 * has its own immutable bytecode / trace / evidence set to inspect.
 */

import fs from "node:fs";
import path from "node:path";

export const BASE_CONTRACT = path.resolve(
  process.cwd(),
  "argorix/chatbot-runtime.argx",
);

export const GENERATED_DIR = path.resolve(process.cwd(), "generated");

// Entities declared inside chatbot-runtime.argx — referenced by the VM run.
export const RUNTIME_PROFILE = "ChatbotRuntime";
export const ADAPTER = "OpenAISandbox";
export const OPERATION = "responses.create";
export const PROVIDER = "OpenAIProvider";
export const ASSISTANT_AGENT = "AssistantAgent";
// Injected message edge: User -> AssistantAgent: tell UserPrompt
export const INJECT = "User:AssistantAgent:tell:UserPrompt";

export interface SessionPaths {
  requestId: string;
  dir: string;
  contract: string;
  bytecode: string;
  securityReport: string;
  trace: string;
  evidenceBundle: string;
}

export function buildSessionContract(requestId: string): SessionPaths {
  const dir = path.join(GENERATED_DIR, requestId);
  fs.mkdirSync(dir, { recursive: true });

  const contract = path.join(dir, "session.argx");
  fs.copyFileSync(BASE_CONTRACT, contract);

  return {
    requestId,
    dir,
    contract,
    bytecode: path.join(dir, "session.argbc.json"),
    securityReport: path.join(dir, "session.security.json"),
    trace: path.join(dir, "session.trace.json"),
    evidenceBundle: path.join(dir, "session.evidence.json"),
  };
}
