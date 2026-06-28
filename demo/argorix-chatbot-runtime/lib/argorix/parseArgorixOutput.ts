/**
 * parseArgorixOutput.ts
 * ---------------------
 * Redaction + parsing helpers that turn raw Argorix CLI / artifact JSON into
 * the compact, secret-free summaries the UI consumes.
 *
 * `redact()` is the single most important function in the demo from a security
 * standpoint: every string that could possibly reach the client passes through
 * it. It strips:
 *   - the live value of OPENAI_API_KEY (if one is configured), and
 *   - anything matching common provider key shapes (sk-..., Bearer tokens).
 */

const KEY_LIKE_PATTERNS: RegExp[] = [
  /sk-[A-Za-z0-9_-]{8,}/g, // OpenAI-style secret keys
  /Bearer\s+[A-Za-z0-9._-]{8,}/gi, // Authorization headers
];

export function redact(input: string): string {
  if (!input) return input;
  let out = input;

  const live = process.env.OPENAI_API_KEY;
  if (live && live.length >= 4) {
    out = out.split(live).join("[REDACTED:OPENAI_API_KEY]");
  }
  for (const re of KEY_LIKE_PATTERNS) {
    out = out.replace(re, "[REDACTED]");
  }
  return out;
}

/** Parse JSON safely; returns null instead of throwing. */
export function safeJson<T = unknown>(text: string): T | null {
  try {
    return JSON.parse(text) as T;
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Bytecode-derived views (real values straight from the compiled contract).
// ---------------------------------------------------------------------------

export interface RuntimeProfileView {
  name: string;
  mode: string;
  provider: string;
  network: string;
  externalExecution: string;
  toolExecution: string;
  agentExecution: string;
  secrets: string;
  keyMaterial: string;
  audit: string;
  evidence: string;
  securityReport: string;
  failClosed: boolean;
  securityClaims: string;
  allowedActions: string[];
  deniedActions: string[];
}

export interface AdapterView {
  name: string;
  provider: string;
  adapterKind: string;
  protocol: string;
  endpointRef: string;
  secretRef: string;
  redacted: boolean;
  allowedOperations: string[];
  deniedOperations: string[];
  network: string;
  externalExecution: string;
  failClosed: boolean;
  securityClaims: string;
}

export interface PassportView {
  name: string;
  agent: string;
  agentName: string;
  globalId: string;
  identity: string;
  provider: string;
  version: string;
  country: string;
  jurisdiction: string;
  riskLevel: string;
  intent: string;
  intendedUse: string[];
  prohibitedUse: string[];
  attestations: string[];
}

interface Bytecode {
  module?: string;
  runtime_execution_profiles?: Array<Record<string, unknown>>;
  sandboxed_provider_adapters?: Array<Record<string, unknown>>;
  passports?: Array<Record<string, unknown>>;
}

const str = (v: unknown, d = ""): string => (typeof v === "string" ? v : d);
const arr = (v: unknown): string[] => (Array.isArray(v) ? (v as string[]) : []);
const bool = (v: unknown): boolean => v === true;

export function extractRuntimeProfile(
  raw: Record<string, unknown>,
  name: string,
): RuntimeProfileView | null {
  const bc = raw as Bytecode;
  const p = (bc.runtime_execution_profiles ?? []).find(
    (x) => str(x.name) === name,
  );
  if (!p) return null;
  return {
    name: str(p.name),
    mode: str(p.mode),
    provider: str(p.provider),
    network: str(p.network),
    externalExecution: str(p.external_execution),
    toolExecution: str(p.tool_execution),
    agentExecution: str(p.agent_execution),
    secrets: str(p.secrets),
    keyMaterial: str(p.key_material),
    audit: str(p.audit),
    evidence: str(p.evidence),
    securityReport: str(p.security_report),
    failClosed: bool(p.fail_closed),
    securityClaims: str(p.security_claims),
    allowedActions: arr(p.allowed_actions),
    deniedActions: arr(p.denied_actions),
  };
}

export function extractAdapter(
  raw: Record<string, unknown>,
  name: string,
): AdapterView | null {
  const bc = raw as Bytecode;
  const a = (bc.sandboxed_provider_adapters ?? []).find(
    (x) => str(x.name) === name,
  );
  if (!a) return null;
  return {
    name: str(a.name),
    provider: str(a.provider),
    adapterKind: str(a.adapter_kind),
    protocol: str(a.protocol),
    endpointRef: str(a.endpoint_ref),
    secretRef: str(a.secret_ref),
    redacted: bool(a.redacted),
    allowedOperations: arr(a.allowed_operations),
    deniedOperations: arr(a.denied_operations),
    network: str(a.network),
    externalExecution: str(a.external_execution),
    failClosed: bool(a.fail_closed),
    securityClaims: str(a.security_claims),
  };
}

export function extractPassport(
  raw: Record<string, unknown>,
  agent: string,
): PassportView | null {
  const bc = raw as Bytecode;
  const p = (bc.passports ?? []).find((x) => str(x.agent) === agent);
  if (!p) return null;
  return {
    name: str(p.name),
    agent: str(p.agent),
    agentName: str(p.agent_name),
    globalId: str(p.global_id),
    identity: str(p.identity),
    provider: str(p.provider),
    version: str(p.version),
    country: str(p.country),
    jurisdiction: str(p.jurisdiction),
    riskLevel: str(p.risk_level),
    intent: str(p.intent),
    intendedUse: arr(p.intended_use),
    prohibitedUse: arr(p.prohibited_use),
    attestations: arr(p.attestations),
  };
}

// ---------------------------------------------------------------------------
// Security-report + evidence-bundle summaries.
// ---------------------------------------------------------------------------

export interface SecurityReportSummary {
  module: string;
  reportVersion: string;
  verdictPassed: boolean;
  severity: string;
  reasons: string[];
  lastEvent: string;
  runtimeProfilesDeclared: number;
  adaptersDeclared: number;
  secretRefsRedacted: boolean;
}

export function summarizeSecurityReport(
  report: Record<string, unknown> | null,
): SecurityReportSummary | null {
  if (!report) return null;
  const verdict = (report.verdict ?? {}) as Record<string, unknown>;
  const ledger = (report.ledger ?? {}) as Record<string, unknown>;
  const rep = (report.runtime_execution_profiles ?? {}) as Record<
    string,
    unknown
  >;
  const adp = (report.sandboxed_provider_adapters ?? {}) as Record<
    string,
    unknown
  >;
  const redactedCounts = (adp.secret_refs_redacted ?? {}) as Record<
    string,
    number
  >;
  return {
    module: str(report.module),
    reportVersion: str(report.report_version),
    verdictPassed: bool(verdict.passed),
    severity: str(verdict.severity),
    reasons: arr(verdict.reasons),
    lastEvent: str(ledger.last_event),
    runtimeProfilesDeclared:
      typeof rep.total === "number" ? (rep.total as number) : 0,
    adaptersDeclared: typeof adp.total === "number" ? (adp.total as number) : 0,
    secretRefsRedacted: (redactedCounts["true"] ?? 0) > 0,
  };
}

export interface EvidenceSummary {
  bundleVersion: string;
  module: string;
  bytecodeDigest: string;
  traceDigest: string | null;
  reportDigest: string;
  ledgerDigest: string;
  verifyPassed: boolean | null;
}

export function summarizeEvidence(
  bundle: Record<string, unknown> | null,
  verify: Record<string, unknown> | null,
): EvidenceSummary | null {
  if (!bundle) return null;
  return {
    bundleVersion: str(bundle.bundle_version),
    module: str(bundle.module),
    bytecodeDigest: str(bundle.bytecode_digest),
    traceDigest: bundle.trace_digest ? str(bundle.trace_digest) : null,
    reportDigest: str(bundle.report_digest),
    ledgerDigest: str(bundle.ledger_digest),
    verifyPassed: verify ? bool(verify.passed) : null,
  };
}

/** Pull the `status` field out of a `--runtime` VM JSON run. */
export function runtimeStatus(vmJson: Record<string, unknown> | null): string {
  if (!vmJson) return "unknown";
  return str(vmJson.status, "unknown");
}
