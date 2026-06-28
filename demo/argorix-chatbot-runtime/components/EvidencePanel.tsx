import type { ChatResponse } from "@/lib/types";

function Row({ k, v }: { k: string; v: React.ReactNode }) {
  return (
    <div className="row">
      <span className="k">{k}</span>
      <span className="v">{v}</span>
    </div>
  );
}

export default function EvidencePanel({ data }: { data: ChatResponse | null }) {
  const ev = data?.argorix.evidence;
  return (
    <div className="panel">
      <h2>Evidence Bundle</h2>
      {!ev ? (
        <p className="empty">
          {data
            ? "No evidence generated (governance blocked before VM run)."
            : "No run yet."}
        </p>
      ) : (
        <>
          <Row k="bundle_version" v={ev.bundleVersion} />
          <Row k="module" v={ev.module} />
          <Row
            k="verify-evidence"
            v={
              <span className={`tag ${ev.verifyPassed ? "ok" : "bad"}`}>
                {ev.verifyPassed === null
                  ? "n/a"
                  : ev.verifyPassed
                    ? "passed"
                    : "failed"}
              </span>
            }
          />
          <Row k="bytecode_digest" v={short(ev.bytecodeDigest)} />
          <Row k="trace_digest" v={short(ev.traceDigest)} />
          <Row k="report_digest" v={short(ev.reportDigest)} />
          <Row k="ledger_digest" v={short(ev.ledgerDigest)} />
        </>
      )}
    </div>
  );
}

function short(d: string | null): string {
  if (!d) return "—";
  return d.length > 24 ? `${d.slice(0, 20)}…${d.slice(-6)}` : d;
}
