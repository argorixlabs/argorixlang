import type { ChatResponse } from "@/lib/types";

function Row({ k, v }: { k: string; v: React.ReactNode }) {
  return (
    <div className="row">
      <span className="k">{k}</span>
      <span className="v">{v}</span>
    </div>
  );
}

export default function PassportPanel({ data }: { data: ChatResponse | null }) {
  const p = data?.argorix.passport;
  return (
    <div className="panel">
      <h2>Agent Passport</h2>
      {!p ? (
        <p className="empty">No passport loaded yet.</p>
      ) : (
        <>
          <Row k="passport" v={p.name} />
          <Row k="agent" v={`${p.agentName} (${p.agent})`} />
          <Row k="global_id" v={p.globalId} />
          <Row k="identity" v={p.identity} />
          <Row k="provider" v={p.provider} />
          <Row k="version" v={p.version} />
          <Row k="jurisdiction" v={`${p.country} / ${p.jurisdiction}`} />
          <Row k="risk_level" v={p.riskLevel} />
          <Row k="intent" v={p.intent} />
          <div className="row">
            <span className="k">intended_use</span>
            <span className="v">
              {p.intendedUse.map((u) => (
                <span key={u} className="tag ok">
                  {u}
                </span>
              ))}
            </span>
          </div>
          <div className="row">
            <span className="k">prohibited_use</span>
            <span className="v">
              {p.prohibitedUse.map((u) => (
                <span key={u} className="tag bad">
                  {u}
                </span>
              ))}
            </span>
          </div>
          <div className="row">
            <span className="k">attestations</span>
            <span className="v">
              {p.attestations.map((u) => (
                <span key={u} className="tag">
                  {u}
                </span>
              ))}
            </span>
          </div>
        </>
      )}
    </div>
  );
}
