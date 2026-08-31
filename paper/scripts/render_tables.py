"""Render deterministic LaTeX table fragments from normalized paper data."""
from __future__ import annotations
import argparse, csv, json, re
from pathlib import Path
from atomic_io import atomic_write_text


ROOT = Path(__file__).resolve().parents[2]


def esc(value) -> str:
    replacements = {
        "\\": r"\textbackslash{}", "&": r"\&", "%": r"\%", "$": r"\$",
        "#": r"\#", "_": r"\_", "{": r"\{", "}": r"\}",
        "~": r"\textasciitilde{}", "^": r"\textasciicircum{}",
    }
    return re.sub(r"[\\&%$#_{}~^]", lambda match: replacements[match.group()], str(value))


def table(headers, rows, width=r"\linewidth"):
    cols="@{}" + "X" * len(headers) + "@{}"
    lines=[f"\\begin{{tabularx}}{{{width}}}{{{cols}}}", "\\toprule",
           " & ".join(map(esc,headers))+r" \\", "\\midrule"]
    lines += [" & ".join(esc(v) for v in row)+r" \\" for row in rows]
    lines += ["\\bottomrule","\\end{tabularx}",""]
    return "\n".join(lines)


def display_id(value: str) -> str:
    return str(value).replace("_", " ")


def render(data: Path, out: Path):
    out.mkdir(parents=True,exist_ok=True)
    summary=json.loads((data/"runtime_summary.json").read_text(encoding="utf-8"))
    verification=json.loads((data/"verification-results.json").read_text(encoding="utf-8"))
    with (data/"sessions.csv").open(encoding="utf-8",newline="") as f: sessions=list(csv.DictReader(f))
    with (data/"event_counts.csv").open(encoding="utf-8",newline="") as f: events=list(csv.DictReader(f))
    complete=[r for r in sessions if r.get("complete","").lower()=="true"]
    event_total=sum(int(r["count"]) for r in events)
    verified_total=sum(bool(item.get("verified")) for item in verification)
    matrix_path = ROOT / "results" / "controlled_matrix.json"
    matrix = json.loads(matrix_path.read_text(encoding="utf-8")) if matrix_path.exists() else {"summary": {}, "rows": []}
    matrix_summary = matrix.get("summary", {})
    matrix_rows = matrix.get("rows", [])
    coverage_path = ROOT / "results" / "passport_country_coverage.csv"
    coverage = []
    if coverage_path.exists():
        with coverage_path.open(encoding="utf-8", newline="") as f:
            coverage = list(csv.DictReader(f))
    lattice_path = ROOT / "results" / "policy_lattice_summary.csv"
    lattice_rows = []
    if lattice_path.exists():
        with lattice_path.open(encoding="utf-8", newline="") as f:
            lattice_rows = list(csv.DictReader(f))
    tamper_rows = [row for row in matrix_rows if row.get("Outcome") in {"correct_tamper_detection", "correct_source_detection"}]
    provider_rows = [row for row in matrix_rows if row.get("Case") in {
        "valid_simulated_provider", "external_provider_without_adapter",
        "network_denied_profile", "plaintext_secret_attempt", "key_material_attempt",
    }]
    files={
      "dataset-inventory.tex": table(["Normalized source","Rows / records","Role"],[
        ("runtime_summary.json",len(summary.get("sessions",[])),"Session-level normalized summary"),
        ("sessions.csv",len(sessions),"Tabular session observations"),
        ("event_counts.csv",len(events),"Per-session event-kind counts"),
        ("verification-results.json",len(verification),"Verification result groups")]),
      "language-constructs.tex": table(["Construct family","Representation","Boundary"],[
        ("Agents and messages","Typed declarations","Implemented"),
        ("Provider contracts","Declarative metadata","Declarative"),
        ("Policies and passports","Semantic plus runtime controls","Implemented"),
        ("ATrust evidence maps","Validated evidence links","Declarative")]),
      "runtime-controls.tex": table(["Control","Observed behavior","Scope"],[
        ("Provider execution","simulated only","Implemented"),
        ("External execution","Blocked","Implemented"),
        ("Network access","Denied by runtime profiles","Implemented"),
        ("Secrets and key material","Denied","Implemented")]),
      "policy-lattice.tex": table(["Outcome","Meaning","Current status"],[
        ("PASS","Known rule satisfied","Implemented in controlled matrix"),
        ("DENY","Known rule violated and execution must stop","Implemented in controlled matrix"),
        ("REVIEW","Known rule requires human review","Implemented in controlled matrix"),
        ("UNKNOWN_RULE","Unsupported policy identifier or configuration error","Implemented as diagnostic outcome"),
        ("ERROR","Malformed policy object or parse/semantic failure","Implemented in controlled matrix")]),
      "controlled-evaluation-matrix.tex": table(["Case","Expected outcome","Claim status"],[
        ("31 same-country passport cases","PASS; no external side effect","Generated in controlled matrix"),
        ("six cross-border residency cases","PASS / REVIEW / DENY by profile","Generated in controlled matrix"),
        ("valid EvidenceBundle","offline verifier passes","Observed for current bundles"),
        ("valid source digest match","source digest matches fixture source","Generated in controlled matrix"),
        ("external provider without executable adapter","DENY","Observed as blocked boundary"),
        ("network attempt under denied profile","DENY","Observed as denied event"),
        ("plaintext secret or key material attempt","DENY","Observed as denied class"),
        ("residency mismatch or unknown jurisdiction","REVIEW","Generated in controlled matrix"),
        ("modified trace / report / bytecode / ledger","VERIFIER_FAIL","Generated in controlled matrix"),
        ("source mismatch","VERIFIER_FAIL","Generated in controlled matrix"),
        ("unknown policy rule","UNKNOWN_RULE configuration error","Generated in controlled matrix"),
        ("malformed policy object","ERROR parse/semantic failure","Generated in controlled matrix")]),
      "passport-jurisdiction-coverage.tex": table(["Region group","Countries","Valid PASS cases"],[
        (region, len([row for row in coverage if row.get("region_group") == region]),
         len([row for row in coverage if row.get("region_group") == region and row.get("observed") == "PASS"]))
        for region in ("home","latin_america","north_america","europe","asia_pacific","africa_middle_east")
      ]),
      "controlled-matrix-summary.tex": table(["Measure","Value","Interpretation"],[
        ("Total controlled rows", matrix_summary.get("total_rows",""), "Generated deterministic fixtures"),
        ("Countries tested", matrix_summary.get("countries_tested",""), "National ISO alpha-2 metadata only"),
        ("Valid same-country passport cases", matrix_summary.get("valid_passport_cases",""), "One PASS fixture per country"),
        ("False allow / deny / review", f"{matrix_summary.get('false_allow_count','')} / {matrix_summary.get('false_deny_count','')} / {matrix_summary.get('false_review_count','')}", "Outcome-classification checks"),
        ("Evidence pass / fail", f"{matrix_summary.get('evidence_verification_pass_count','')} / {matrix_summary.get('evidence_verification_fail_count','')}", "Bundle or digest-consistency status"),
      ]),
      "policy-lattice-outcomes.tex": table(["Outcome","Count","Scope"],[
        (row.get("outcome",""), row.get("count",""), "Controlled matrix")
        for row in lattice_rows
      ]),
      "matrix-tamper-results.tex": table(["Case","Observed","Evidence","Source"],[
        (display_id(row.get("Case","")), row.get("Observed",""), display_id(row.get("Evidence","")), display_id(row.get("Source","")))
        for row in tamper_rows
      ]),
      "matrix-provider-boundary.tex": table(["Case","Observed","Provider","Side effect"],[
        (display_id(row.get("Case","")), row.get("Observed",""), display_id(row.get("Provider","")), row.get("Side Effect",""))
        for row in provider_rows
      ]),
      "ablation-study.tex": table(["Variant","Removed boundary","Do not claim until measured"],[
        ("Full ArgorixLang v0.2","none","false allow, false deny, review accuracy, tamper rate, latency"),
        ("w/o typed policy lattice","typed outcomes","unknown-rule collapse and aggregation errors"),
        ("w/o executable-provider allowlist","adapter gate","external-provider false allows"),
        ("w/o source digest","source binding","source mismatch detection"),
        ("w/o ledger digest","trace-event ledger binding","ledger tamper detection"),
        ("w/o report cross-check","report-to-bundle linkage","summary/detail inconsistency detection"),
        ("w/o fail-closed default","default denial","missing-authorization false allows"),
        ("JSON audit logs only baseline","cross-artifact verification","tamper and linkage gaps")]),
      "baseline-comparison.tex": table(["Baseline","What it covers","Gap relative to ArgorixLang"],[
        ("Plain application logs","chronological observations","no compiled policy or digest bundle"),
        ("JSON audit report","structured summary","no cross-artifact verification"),
        ("OPA/Rego-style external policy","policy decision engine","no compiled passport/evidence constructs unless integrated"),
        ("in-toto-style provenance","signed supply-chain steps","different scope; Argorix currently unsigned local runtime evidence"),
        ("MCP/A2A declaration only","protocol surface description","no compiled fail-closed provider boundary")]),
      "empirical-results.tex": table(["Measure","Derived value","Source"],[
        ("Complete sessions",summary["complete_sessions"],"runtime_summary.json"),
        ("Incomplete sessions",summary["incomplete_sessions"],"runtime_summary.json"),
        ("Normalized sessions",len(sessions),"sessions.csv"),
        ("Complete-session policy violations",sum(int(r.get("policy_violation_count") or 0) for r in complete),"sessions.csv"),
        ("Counted ledger events",event_total,"event_counts.csv"),
        ("Prompt-content traces inspected / present",
         f"{summary['traces_inspected_for_prompt_content']} / {summary['traces_with_prompt_content']}",
         "runtime_summary.json"),
        ("Verified evidence bundles",f"{verified_total} / {len(verification)}","verification-results.json")]),
      "threat-mapping.tex": table(["Threat","Mitigation","Claim status"],[
        ("Unauthorized provider execution","Executable-provider allowlist","Implemented"),
        ("Network side effects","Offline and network-denied profiles","Implemented"),
        ("Secret disclosure","Secret and key-material denial","Implemented"),
        ("Evidence tampering","Digest-linked artifact bundle","Implemented")]),
      "limitations-future-assurance.tex": table(["Limitation","Current status","Future assurance requirement"],[
        ("Live provider execution","Not implemented","sandboxed adapter plus host telemetry"),
        ("Real identity authentication","Not implemented","DID/VC or equivalent resolver and issuer policy"),
        ("Source integrity","Outside current bundle verification","source digest and source-to-bytecode binding"),
        ("Producer authentication","Not implemented","signed bundles and key governance"),
        ("Replacement detection","Not implemented","append-only log or transparency service"),
        ("Large workload diversity","Not present","controlled workload matrix and pre-registered metrics")]),
      "related-work.tex": table(["System","Comparison dimension","Bibliographic scope"],[
        ("Project NANDA / AgentFacts / ANS","Agent naming and discovery","Project-level comparison only"),
        ("ATrust","Agent trust relationships","Project-level comparison only"),
        ("DCP-AI","Discovery and control plane","Project-level comparison only"),
        ("OAuth/OIDC, FIDO, AP2, Verifiable Intent","Delegation and agent authorization","Conceptual comparison only"),
        ("SPIFFE/SPIRE and OWASP NHI","Non-human/workload identity","Scope comparison only"),
        ("Open Policy Agent","External policy-as-code engine","Direct architectural comparison"),
        ("Cedar","Authorization policy language","Conceptual comparison only"),
        ("Jason / AgentSpeak","BDI agent language and interpreter","Direct language/runtime comparison"),
        ("in-toto / SLSA / VET Your Agent","Evidence, provenance, and external evaluation","Scope comparison only"),
        ("WASI","Capability-based host isolation","Direct isolation-boundary comparison")]),
      "claim-boundaries.tex": table(
        ["Concept","Implemented","Declarative","Proposed","Not claimed"],[
        ("Provider boundary","simulated allowlist","contract metadata","--","external execution"),
        ("Trust evidence","digest bundle","ATrust map","--","live attestation"),
        ("Discovery","local ans_name binding/metadata","sovereign metadata","federation","operational DNS"),
        ("Security scope","fail-closed controls","threat mappings","deployment study","certification"),
      ], width=r"\textwidth"),
    }
    for name, text in files.items():
        atomic_write_text(out / name, text)


def main():
    p=argparse.ArgumentParser(); p.add_argument("--data",type=Path,required=True); p.add_argument("--output",type=Path,required=True)
    a=p.parse_args(); render(a.data,a.output)
if __name__=="__main__": main()
