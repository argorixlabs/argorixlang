/**
 * POST /api/chat
 * --------------
 * The governed pipeline. Every chat turn is validated by Argorix BEFORE any
 * thought is given to an external provider call:
 *
 *   1. check        — syntax + semantics of the session contract
 *   2. emit-bytecode
 *   3. verify-bytecode
 *   4. VM reactive run -> security report + trace + evidence bundle (+ verify)
 *   5. VM runtime run  -> governed decision (blocked | planned)
 *
 * Decision matrix:
 *   - any of 1–3 fail                              -> status "blocked"
 *   - ARGORIX_SANDBOXED_EXTERNAL=false             -> status "planned" (no net)
 *   - =true, governed != planned                   -> status "blocked"
 *   - =true, planned, no OPENAI_API_KEY            -> status "simulated"
 *   - =true, planned, key present                  -> status "sandboxed_external"
 *
 * Secrets never enter this response: the contract only carries the reference
 * `env:OPENAI_API_KEY`, and all CLI output is redacted in runArgorix.ts.
 */

import { NextResponse } from "next/server";
import { randomUUID } from "node:crypto";
import fs from "node:fs";

import {
  ADAPTER,
  INJECT,
  OPERATION,
  PROVIDER,
  RUNTIME_PROFILE,
  ASSISTANT_AGENT,
  buildSessionContract,
} from "@/lib/argorix/buildSessionContract";
import * as argorix from "@/lib/argorix/runArgorix";
import {
  extractAdapter,
  extractPassport,
  extractRuntimeProfile,
  runtimeStatus,
  safeJson,
  summarizeEvidence,
  summarizeSecurityReport,
} from "@/lib/argorix/parseArgorixOutput";
import { callOpenAI } from "@/lib/openai/callOpenAI";
import { inspect } from "@/lib/argorix/inputGuard";
import type { ChatResponse, DemoStatus } from "@/lib/types";

// child_process / fs require the Node.js runtime.
export const runtime = "nodejs";
export const dynamic = "force-dynamic";

function readJsonFile(p: string): Record<string, unknown> | null {
  try {
    return safeJson<Record<string, unknown>>(fs.readFileSync(p, "utf8"));
  } catch {
    return null;
  }
}

export async function POST(req: Request) {
  const requestId = randomUUID();

  let message = "";
  try {
    const body = (await req.json()) as { message?: unknown };
    message = typeof body.message === "string" ? body.message.trim() : "";
  } catch {
    return NextResponse.json(
      { error: "invalid JSON body" },
      { status: 400 },
    );
  }
  if (!message) {
    return NextResponse.json({ error: "empty message" }, { status: 400 });
  }

  const sandboxedExternalEnabled =
    process.env.ARGORIX_SANDBOXED_EXTERNAL === "true";

  // Input boundary: enforce the threat_model's declared mitigations (prompt
  // injection / secret exfiltration) BEFORE any provider call is considered.
  const guard = inspect(message);

  const paths = buildSessionContract(requestId);

  // ---- 1. check -----------------------------------------------------------
  const checkRes = argorix.check(paths.contract);

  // ---- 2/3. emit + verify bytecode ---------------------------------------
  const emitRes = checkRes.ok
    ? argorix.emitBytecode(paths.contract, paths.bytecode)
    : { ok: false, exitCode: 1, stdout: "", stderr: "skipped: check failed" };
  const verifyRes = emitRes.ok
    ? argorix.verifyBytecode(paths.contract)
    : { ok: false, exitCode: 1, stdout: "", stderr: "skipped: emit failed" };

  const bytecode = emitRes.ok ? readJsonFile(paths.bytecode) : null;
  const runtimeProfile = bytecode
    ? extractRuntimeProfile(bytecode, RUNTIME_PROFILE)
    : null;
  const adapter = bytecode ? extractAdapter(bytecode, ADAPTER) : null;
  const passport = bytecode ? extractPassport(bytecode, ASSISTANT_AGENT) : null;

  // If governance validation failed, fail closed immediately.
  const governanceOk = checkRes.ok && emitRes.ok && verifyRes.ok;

  // ---- 4. reactive evidence run ------------------------------------------
  let securityReportSummary = null;
  let evidence = null;
  if (governanceOk) {
    argorix.runReactiveEvidence(paths.bytecode, {
      inject: INJECT,
      securityReport: paths.securityReport,
      traceOut: paths.trace,
      evidenceBundle: paths.evidenceBundle,
    });
    securityReportSummary = summarizeSecurityReport(
      readJsonFile(paths.securityReport),
    );
    const verifyEvidenceRes = argorix.verifyEvidence(paths.evidenceBundle);
    evidence = summarizeEvidence(
      readJsonFile(paths.evidenceBundle),
      safeJson<Record<string, unknown>>(verifyEvidenceRes.stdout),
    );
  }

  // ---- 5. governed runtime decision --------------------------------------
  let governedStatus = "blocked";
  let runtimeCliOut = "skipped: governance failed";
  if (governanceOk) {
    const rt = argorix.runRuntimeProfile(paths.bytecode, {
      runtime: RUNTIME_PROFILE,
      adapter: ADAPTER,
      operation: OPERATION,
      sandboxedExternal: sandboxedExternalEnabled,
    });
    runtimeCliOut = rt.stdout || rt.stderr;
    governedStatus = runtimeStatus(safeJson<Record<string, unknown>>(rt.stdout));
  }

  // ---- decide final demo status + answer ---------------------------------
  let status: DemoStatus;
  let answer: string;

  const moduleName =
    typeof bytecode?.module === "string"
      ? bytecode.module
      : "ArgorixChatbotRuntime";
  const plan = [
    `Argorix compiled and verified contract module "${moduleName}".`,
    `runtime_execution_profile ${RUNTIME_PROFILE} selected in mode "${
      runtimeProfile?.mode ?? "sandboxed_external"
    }" (fail_closed=${runtimeProfile?.failClosed ?? true}).`,
    `sandboxed_provider_adapter ${ADAPTER} bound to provider ${PROVIDER}; secret_ref="${
      adapter?.secretRef ?? "env:OPENAI_API_KEY"
    }" (value absent, redacted=${adapter?.redacted ?? true}).`,
    `operation "${OPERATION}" is in allowed_operations; tool/shell/filesystem operations are denied.`,
    `Network is "${runtimeProfile?.network ?? "declared_only"}"; external_execution is "${
      runtimeProfile?.externalExecution ?? "sandboxed"
    }".`,
    `Evidence bundle + security report generated and integrity-verified locally.`,
  ];

  if (!governanceOk) {
    status = "blocked";
    const why =
      (!checkRes.ok && checkRes.stderr) ||
      (!emitRes.ok && emitRes.stderr) ||
      (!verifyRes.ok && verifyRes.stderr) ||
      "governance validation failed";
    answer = `⛔ Blocked by Argorix runtime (fail-closed). Governance validation did not pass:\n${why}`;
  } else if (guard.flagged) {
    // Policy review: the input guard enforced a declared threat mitigation.
    status = "blocked";
    const label =
      guard.category === "secret_exfiltration"
        ? "secret exfiltration attempt"
        : "prompt-injection attempt";
    answer =
      `⛔ Blocked by Argorix input guard (fail-closed, action=review).\n\n` +
      `Detected a ${label}, which maps to ${guard.mappedThreat} in ` +
      `threat_model ChatbotThreatModel. Per the declared mitigation, the turn ` +
      `is held for policy review and NO provider call was made.\n\n` +
      `Note: even if such a payload reached the model, the runtime profile keeps ` +
      `tool_execution, agent_execution and unsanctioned network disabled.`;
  } else if (!sandboxedExternalEnabled) {
    // Plan-only: contract is valid; we deliberately do NOT touch the network.
    status = "planned";
    answer =
      `📝 Plan-only mode (ARGORIX_SANDBOXED_EXTERNAL=false).\n\n` +
      `Argorix compiled and verified the contract. Because there is no operator ` +
      `opt-in, the governed runtime decision is "${governedStatus}" (fail-closed) ` +
      `and no external provider was contacted.\n\n` +
      `This is the auditable plan that WOULD run if you set ` +
      `ARGORIX_SANDBOXED_EXTERNAL=true:\n` +
      plan.map((s, i) => `  ${i + 1}. ${s}`).join("\n");
  } else if (governedStatus !== "planned") {
    status = "blocked";
    answer =
      `⛔ Blocked: even with ARGORIX_SANDBOXED_EXTERNAL=true the governed ` +
      `runtime decision was "${governedStatus}", so no provider call is permitted.`;
  } else {
    // sandboxed_external requested AND Argorix planned the call -> may proceed.
    const outcome = await callOpenAI(message);
    if (outcome.kind === "no_key") {
      status = "simulated";
      answer =
        `🧪 Simulated answer (sandboxed_external enabled but OPENAI_API_KEY is not set).\n\n` +
        `Argorix planned the sandboxed call; with a server-side key configured, ` +
        `the adapter would now perform it. Echoing your message instead:\n\n` +
        `"${message}"`;
    } else if (outcome.kind === "error") {
      status = "blocked";
      answer = `⛔ Sandboxed provider call failed (fail-closed): ${outcome.message}`;
    } else {
      status = "sandboxed_external";
      answer = outcome.text;
    }
  }

  const response: ChatResponse = {
    requestId,
    answer,
    argorix: {
      status,
      governedStatus,
      provider: PROVIDER,
      failClosed: runtimeProfile?.failClosed ?? true,
      network: runtimeProfile?.network ?? "declared_only",
      externalExecution: runtimeProfile?.externalExecution ?? "sandboxed",
      mcpRuntime: "disabled",
      a2aRuntime: "disabled",
      toolExecution: runtimeProfile?.toolExecution ?? "disabled",
      agentExecution: runtimeProfile?.agentExecution ?? "disabled",
      secretRefRedacted: adapter?.secretRef ?? "env:OPENAI_API_KEY",
      endpointRefRedacted: adapter?.endpointRef ?? "env:OPENAI_BASE_URL",
      securityClaims: runtimeProfile?.securityClaims ?? "none",
      sandboxedExternalEnabled,
      inputGuard: guard,
      runtimeProfile,
      adapter,
      passport,
      evidence,
      securityReportSummary,
      plan,
      cli: {
        check: checkRes.stdout || checkRes.stderr,
        verifyBytecode: verifyRes.stdout || verifyRes.stderr,
        runtime: runtimeCliOut,
      },
    },
  };

  return NextResponse.json(response);
}
