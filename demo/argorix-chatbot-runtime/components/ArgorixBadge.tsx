import type { ChatResponse } from "@/lib/types";

const LABELS: Record<string, string> = {
  blocked: "BLOCKED",
  planned: "PLANNED",
  simulated: "SIMULATED",
  sandboxed_external: "SANDBOXED",
};

const SUBTITLES: Record<string, string> = {
  blocked: "Argorix fail-closed — no provider call",
  planned: "Validated & planned — no network touched",
  simulated: "Planned; no key, answer simulated",
  sandboxed_external: "Validated, planned & sandboxed call performed",
};

export default function ArgorixBadge({ data }: { data: ChatResponse | null }) {
  const status = data?.argorix.status ?? "planned";
  const label = data ? LABELS[status] ?? status.toUpperCase() : "READY";
  return (
    <div className="panel">
      <h2>Argorix Runtime Badge</h2>
      <div className={`badge ${data ? status : ""}`}>
        <span className="dot" />
        <div>
          <div className="status">
            Argorix Runtime: {label}
          </div>
          <div className="sub">
            {data ? SUBTITLES[status] : "Send a message to run the governed pipeline"}
          </div>
        </div>
      </div>
      {data && (
        <div style={{ marginTop: 12 }}>
          <span
            className={`tag ${data.argorix.inputGuard.flagged ? "bad" : "ok"}`}
          >
            input_guard:{" "}
            {data.argorix.inputGuard.flagged
              ? `blocked (${data.argorix.inputGuard.category})`
              : "clean"}
          </span>
          <span className={`tag ${data.argorix.failClosed ? "ok" : "bad"}`}>
            fail_closed: {String(data.argorix.failClosed)}
          </span>
          <span className="tag">network: {data.argorix.network}</span>
          <span className="tag">external: {data.argorix.externalExecution}</span>
          <span className="tag">MCP runtime: {data.argorix.mcpRuntime}</span>
          <span className="tag">A2A runtime: {data.argorix.a2aRuntime}</span>
          <span className="tag">tools: {data.argorix.toolExecution}</span>
          <span className="tag">agents: {data.argorix.agentExecution}</span>
          <span className="tag">security_claims: {data.argorix.securityClaims}</span>
          <span className="tag">governed: {data.argorix.governedStatus}</span>
        </div>
      )}
    </div>
  );
}
