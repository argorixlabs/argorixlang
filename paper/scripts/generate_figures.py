"""Generate the paper's reproducible, vector-only figure inventory."""
from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path

import matplotlib
matplotlib.use("pdf")
import matplotlib.pyplot as plt
from matplotlib.patches import FancyArrowPatch, FancyBboxPatch

NAVY, BLUE, SKY, ORANGE, GREEN, RED, GREY, PALE = (
    "#17324D", "#0072B2", "#56B4E9", "#E69F00",
    "#009E73", "#D55E00", "#667788", "#EEF4F7",
)
plt.rcParams.update({
    "font.family": "DejaVu Sans", "font.size": 9, "pdf.fonttype": 42,
    "axes.titleweight": "bold", "axes.titlesize": 11,
})
PDF_META = {"Creator": "Argorix paper pipeline", "Producer": "Matplotlib",
            "CreationDate": None, "ModDate": None}


def canvas(title: str, wide: bool = False):
    fig, ax = plt.subplots(figsize=(7.2 if wide else 3.35, 3.6))
    fig.patch.set_facecolor("white")
    ax.set(xlim=(0, 10), ylim=(0, 10))
    ax.axis("off")
    ax.set_title(title, color=NAVY, pad=8)
    return fig, ax


def box(ax, x, y, w, h, text, color=BLUE, dashed=False):
    patch = FancyBboxPatch((x, y), w, h, boxstyle="round,pad=0.08",
                           fc="white", ec=color, lw=1.5,
                           linestyle="--" if dashed else "-")
    ax.add_patch(patch)
    ax.text(x + w / 2, y + h / 2, text, ha="center", va="center",
            color=NAVY, fontsize=8, wrap=True)


def arrow(ax, a, b, dashed=False, color=GREY):
    ax.add_patch(FancyArrowPatch(a, b, arrowstyle="-|>", mutation_scale=10,
                                 lw=1.2, color=color,
                                 linestyle="--" if dashed else "-"))


def save(fig, path):
    fig.savefig(path, format="pdf", bbox_inches="tight", metadata=PDF_META)
    plt.close(fig)


def architecture(path):
    fig, ax = canvas("Argorix compilation and evidence architecture", True)
    labels = ["Argorix source", "Parser + semantics", "Typed IR + bytecode",
              "Fail-closed VM", "Trace + ledger", "Evidence + reports"]
    labels = ["Argorix\nsource", "Parser +\nsemantics", "Typed IR +\nbytecode",
              "Fail-closed\nVM", "Trace +\nledger", "Evidence +\nreports"]
    for i, label in enumerate(labels):
        x = .2 + i * 1.65
        box(ax, x, 4.2, 1.3, 1.5, label, [NAVY, BLUE, SKY, GREEN, ORANGE, RED][i])
        if i: arrow(ax, (x-.27, 4.95), (x, 4.95))
    ax.text(5, 2.5, "Offline runtime boundary • simulated is the sole executable provider",
            ha="center", color=GREY)
    save(fig, path)


def request_sequence(path):
    fig, ax = canvas("Request lifecycle: controls precede execution", True)
    xs = [1, 3, 5, 7, 9]
    for x, label in zip(xs, ["Caller", "Compiler", "Policy", "VM", "Evidence"]):
        ax.text(x, 8.8, label, ha="center", color=NAVY, weight="bold")
        ax.plot([x, x], [1.2, 8.3], color="#CBD6DE", lw=1)
    steps = [(1,3,"parse + validate"), (3,5,"evaluate controls"),
             (5,7,"permit / deny"), (7,9,"append events"), (9,1,"bundle result")]
    for row, (a,b,t) in enumerate(steps):
        y=7.5-row*1.25; arrow(ax,(a,y),(b,y), color=[BLUE,ORANGE,GREEN,RED,NAVY][row])
        ax.text((a+b)/2,y+.18,t,ha="center",fontsize=8,color=GREY)
    save(fig, path)


def state_machine(path):
    fig, ax = canvas("Fail-closed decision state machine")
    nodes = [(1,7,"Declared"),(6.5,7,"Validated"),(6.5,3,"Permitted"),(1,3,"Denied")]
    for x,y,t in nodes: box(ax,x,y,2.4,1.3,t, GREEN if t=="Permitted" else RED if t=="Denied" else BLUE)
    arrow(ax,(3.4,7.65),(6.5,7.65)); arrow(ax,(7.7,7),(7.7,4.3),color=GREEN)
    arrow(ax,(6.5,7.3),(3.4,3.9),color=RED)
    ax.text(4.2,5.7,"unknown / invalid → deny",ha="center",color=RED,fontsize=8)
    save(fig,path)


def empirical(data, out):
    summary=json.loads((data/"runtime_summary.json").read_text(encoding="utf-8"))
    complete=int(summary["complete_sessions"]); incomplete=int(summary["incomplete_sessions"])
    fig,ax=plt.subplots(figsize=(3.35,3.2))
    ax.bar(["Complete","Incomplete"],[complete,incomplete],color=[GREEN,ORANGE],width=.58)
    ax.set_title("Normalized session outcomes",color=NAVY,weight="bold")
    ax.set_ylabel("Sessions"); ax.spines[["top","right"]].set_visible(False)
    for i,v in enumerate([complete,incomplete]): ax.text(i,v+.4,str(v),ha="center",weight="bold")
    save(fig,out/"session-outcomes.pdf")

    with (data/"sessions.csv").open(encoding="utf-8",newline="") as f:
        rows=list(csv.DictReader(f))
    completed=[r for r in rows if r.get("complete","").lower()=="true"]
    values=[[sum(int(r.get("policy_violation_count") or 0) for r in completed),
             sum(int(r.get("ledger_events_total") or 0) for r in completed)]]
    fig=plt.figure(figsize=(7.0,3.2), constrained_layout=True)
    grid=fig.add_gridspec(1,2,width_ratios=[15,1],wspace=.35)
    ax=fig.add_subplot(grid[0,0]); cax=fig.add_subplot(grid[0,1])
    im=ax.imshow(values,cmap=matplotlib.colors.LinearSegmentedColormap.from_list("argorix",[PALE,ORANGE,RED]))
    ax.set_xticks([0,1],["Policy\nviolations","Ledger\nevents"]); ax.set_yticks([0],["Observed total"])
    for j,v in enumerate(values[0]): ax.text(j,0,f"{v:,}",ha="center",va="center",color=NAVY,weight="bold")
    ax.set_title("Policy and evidence event volume",color=NAVY,weight="bold")
    fig.colorbar(im,cax=cax)
    save(fig,out/"policy-heatmap.pdf")


def wrap_label(label):
    if "\n" in label: return label
    words=label.split()
    if len(words) <= 1: return label
    midpoint=(len(words)+1)//2
    return " ".join(words[:midpoint])+"\n"+" ".join(words[midpoint:])


def flow_figure(path, title, labels, proposed=None):
    fig,ax=canvas(title, True)
    n=len(labels); gap=9.55/n
    for i,label in enumerate(labels):
        x=.15+i*gap; is_prop=proposed is not None and i in proposed
        shown=wrap_label(label)
        if is_prop:
            width=gap-.6
            box(ax,x,4.1,width,1.8,"",[BLUE,GREEN,ORANGE,RED][i%4],True)
            ax.text(x+width/2,5.38,"PROPOSED /\nNOT IMPLEMENTED",
                    ha="center",va="center",fontsize=7,color=NAVY,weight="bold")
            ax.text(x+width/2,4.60,shown,ha="center",va="center",
                    fontsize=8,color=NAVY)
        else:
            box(ax,x,4.1,gap-.6,1.8,shown,[BLUE,GREEN,ORANGE,RED][i%4])
        if i: arrow(ax,(x-.48,5),(x,5),is_prop)
    save(fig,path)


def evidence_verification_scope(path):
    fig, ax = canvas("Offline evidence verification scope", True)
    nodes = [
        (.12, "Source\n(upstream;\nnot bundle-verified)", GREY, True),
        (2.05, "Bytecode", BLUE, False),
        (3.85, "Trace +\nevents", GREEN, False),
        (5.65, "Security\nreport", ORANGE, False),
        (7.70, "Evidence\nbundle", RED, False),
    ]
    widths = [1.48, 1.25, 1.25, 1.35, 1.55]
    for (x, label, color, dashed), width in zip(nodes, widths):
        box(ax, x, 4.55, width, 1.75, label, color, dashed)
    arrow(ax, (1.60, 5.42), (2.05, 5.42), dashed=True)
    arrow(ax, (3.30, 5.42), (3.85, 5.42), color=GREEN)
    arrow(ax, (5.10, 5.42), (5.65, 5.42), color=ORANGE)
    arrow(ax, (7.00, 5.42), (7.70, 5.42), color=RED)
    ax.text(
        5.0,
        2.85,
        "Bundle checks bytecode, trace, and report digests",
        ha="center",
        color=NAVY,
        fontsize=8,
        weight="bold",
    )
    ax.text(
        5.0,
        2.10,
        "ledger_digest = digest(trace.events); report LedgerSummary must match",
        ha="center",
        color=GREY,
        fontsize=8,
    )
    save(fig, path)


def generate(data: Path, out: Path):
    out.mkdir(parents=True,exist_ok=True)
    architecture(out/"architecture.pdf")
    request_sequence(out/"request-sequence.pdf")
    state_machine(out/"decision-state-machine.pdf")
    empirical(data,out)
    evidence_verification_scope(out/"evidence-chain.pdf")
    flow_figure(out/"trust-relationships.pdf","Declarative trust relationships",
                ["Agent identity","Passport","ATrust map","Trust ledger","Claim boundary"])
    flow_figure(out/"threat-mitigation.pdf","Threat-to-control mapping",
                ["External execution","Deny by default","Network denied","Secrets denied","Review evidence"])
    flow_figure(out/"evolution-timeline.pdf","Language evolution and bounded future work",
                ["Core runtime","Provider contracts","Evidence + governance","Operational federation"],{3})
    flow_figure(out/"sovereign-discovery.pdf","Sovereign discovery boundary",
                ["Local declaration","Semantic validation","Offline catalog","Operational DNS"],{3})
    flow_figure(out/"artifact-schema.pdf","Normalized artifact relationships",
                ["session.\nargx","session.\nargbc.json","session.\ntrace.json",
                 "session.\nsecurity.json","session.\nevidence.json"])
    flow_figure(out/"claim-boundaries.pdf","Claim boundary taxonomy",
                ["Implemented","Declarative","Proposed","Not claimed"],{2})


def main():
    p=argparse.ArgumentParser(); p.add_argument("--data",type=Path,required=True); p.add_argument("--output",type=Path,required=True)
    a=p.parse_args(); generate(a.data,a.output)


if __name__=="__main__": main()
