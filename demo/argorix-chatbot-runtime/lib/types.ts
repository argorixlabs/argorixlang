/**
 * Shared response contract between /api/chat and the UI components.
 */
import type {
  AdapterView,
  EvidenceSummary,
  PassportView,
  RuntimeProfileView,
  SecurityReportSummary,
} from "./argorix/parseArgorixOutput";
import type { GuardVerdict } from "./argorix/inputGuard";

export type DemoStatus =
  | "blocked" // Argorix rejected the contract (fail-closed)
  | "planned" // valid contract, plan-only mode, NO external call
  | "simulated" // sandboxed_external requested but no key -> simulated answer
  | "sandboxed_external"; // valid + planned + real sandboxed provider call

export interface ChatResponse {
  requestId: string;
  answer: string;
  argorix: {
    status: DemoStatus;
    governedStatus: string; // raw VM status: blocked | planned | ...
    provider: string;
    failClosed: boolean;
    network: string;
    externalExecution: string;
    mcpRuntime: "disabled";
    a2aRuntime: "disabled";
    toolExecution: string;
    agentExecution: string;
    secretRefRedacted: string;
    endpointRefRedacted: string;
    securityClaims: string;
    sandboxedExternalEnabled: boolean;
    inputGuard: GuardVerdict;
    runtimeProfile: RuntimeProfileView | null;
    adapter: AdapterView | null;
    passport: PassportView | null;
    evidence: EvidenceSummary | null;
    securityReportSummary: SecurityReportSummary | null;
    plan: string[]; // human-readable auditable plan steps
    cli: {
      check: string;
      verifyBytecode: string;
      runtime: string;
    };
  };
}
