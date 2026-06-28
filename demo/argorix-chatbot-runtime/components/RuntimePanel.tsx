import type { ChatResponse } from "@/lib/types";

function Row({ k, v }: { k: string; v: React.ReactNode }) {
  return (
    <div className="row">
      <span className="k">{k}</span>
      <span className="v">{v}</span>
    </div>
  );
}

export default function RuntimePanel({ data }: { data: ChatResponse | null }) {
  const rp = data?.argorix.runtimeProfile;
  const ad = data?.argorix.adapter;

  return (
    <div className="panel">
      <h2>Runtime Profile &amp; Adapter</h2>
      {!data ? (
        <p className="empty">No run yet.</p>
      ) : (
        <>
          <Row k="provider" v={data.argorix.provider} />
          {rp && (
            <>
              <Row k="runtime_execution_profile" v={rp.name} />
              <Row k="mode" v={rp.mode} />
              <Row k="allowed_actions" v={rp.allowedActions.join(", ")} />
              <Row k="denied_actions" v={rp.deniedActions.join(", ")} />
              <Row k="audit" v={rp.audit} />
              <Row k="evidence" v={rp.evidence} />
              <Row k="security_report" v={rp.securityReport} />
              <Row k="key_material" v={rp.keyMaterial} />
            </>
          )}
          {ad && (
            <>
              <Row k="sandboxed_provider_adapter" v={ad.name} />
              <Row k="adapter_kind" v={ad.adapterKind} />
              <Row k="protocol" v={ad.protocol} />
              <Row k="endpoint_ref" v={ad.endpointRef} />
              <Row
                k="secret_ref"
                v={
                  <span>
                    {ad.secretRef}{" "}
                    <span className="tag ok">redacted</span>
                  </span>
                }
              />
              <Row k="allowed_operations" v={ad.allowedOperations.join(", ")} />
              <Row k="denied_operations" v={ad.deniedOperations.join(", ")} />
            </>
          )}
        </>
      )}
    </div>
  );
}
