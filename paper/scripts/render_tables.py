"""Render deterministic LaTeX table fragments from normalized paper data."""
from __future__ import annotations
import argparse, csv, json, re
from pathlib import Path
from atomic_io import atomic_write_text


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


def render(data: Path, out: Path):
    out.mkdir(parents=True,exist_ok=True)
    summary=json.loads((data/"runtime_summary.json").read_text(encoding="utf-8"))
    verification=json.loads((data/"verification-results.json").read_text(encoding="utf-8"))
    with (data/"sessions.csv").open(encoding="utf-8",newline="") as f: sessions=list(csv.DictReader(f))
    with (data/"event_counts.csv").open(encoding="utf-8",newline="") as f: events=list(csv.DictReader(f))
    complete=[r for r in sessions if r.get("complete","").lower()=="true"]
    event_total=sum(int(r["count"]) for r in events)
    verified_total=sum(bool(item.get("verified")) for item in verification)
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
      "related-work.tex": table(["System","Comparison dimension","Bibliographic scope"],[
        ("Project NANDA","Agent naming and discovery","Project-level comparison only"),
        ("ATrust","Agent trust relationships","Project-level comparison only"),
        ("DCP-AI","Discovery and control plane","Project-level comparison only"),
        ("Open Policy Agent","External policy-as-code engine","Direct architectural comparison"),
        ("Jason / AgentSpeak","BDI agent language and interpreter","Direct language/runtime comparison"),
        ("in-toto","Signed supply-chain provenance","Direct evidence-model comparison"),
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
