import type { ChatResponse } from "@/lib/types";

function Row({ k, v }: { k: string; v: React.ReactNode }) {
  return (
    <div className="row">
      <span className="k">{k}</span>
      <span className="v">{v}</span>
    </div>
  );
}

export default function SecurityPanel({ data }: { data: ChatResponse | null }) {
  const s = data?.argorix.securityReportSummary;
  return (
    <div className="panel">
      <h2>Security Report</h2>
      {!s ? (
        <p className="empty">
          {data ? "No security report (governance blocked)." : "No run yet."}
        </p>
      ) : (
        <>
          <Row k="report_version" v={s.reportVersion} />
          <Row k="module" v={s.module} />
          <Row k="last_event" v={s.lastEvent} />
          <Row
            k="verdict"
            v={
              <span className={`tag ${s.verdictPassed ? "ok" : "bad"}`}>
                {s.verdictPassed ? "passed" : "fail-closed"}
              </span>
            }
          />
          <Row k="severity" v={s.severity} />
          <Row k="runtime_profiles" v={s.runtimeProfilesDeclared} />
          <Row k="sandbox_adapters" v={s.adaptersDeclared} />
          <Row
            k="secret_refs_redacted"
            v={
              <span className={`tag ${s.secretRefsRedacted ? "ok" : "bad"}`}>
                {String(s.secretRefsRedacted)}
              </span>
            }
          />
          {s.reasons.length > 0 && (
            <div className="row">
              <span className="k">reasons</span>
              <span className="v">
                {s.reasons.map((r) => (
                  <span key={r} className="tag">
                    {r}
                  </span>
                ))}
              </span>
            </div>
          )}
        </>
      )}
    </div>
  );
}
