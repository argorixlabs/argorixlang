"""Generate the paper's reproducible, vector-only figure inventory.

Visual language: ArgorixLang brand palette (violet/indigo/electric/cyan) with two
distinguishable semantic accents for permit/deny, plus DanceOPD-style treatments
(radial bars, capability bubbles, and a conceptual velocity-field aesthetic for
the flow diagrams). All text, labels, and exact strings required by the figure
tests are preserved.
"""
from __future__ import annotations

import argparse
import csv
import json
import math
from pathlib import Path
from atomic_io import atomic_publish

import matplotlib
matplotlib.use("pdf")
import matplotlib.pyplot as plt
from matplotlib.colors import LinearSegmentedColormap
from matplotlib.patches import FancyArrowPatch, FancyBboxPatch

# --- ArgorixLang brand palette --------------------------------------------
VIOLET, INDIGO, ELECTRIC, CYAN, CYAN_BRIGHT, WHITE = (
    "#7C3AED", "#5B3DF5", "#2F6BFF", "#2BC4F4", "#5CD6FF", "#FFFFFF",
)
# Semantic accents (brand-consistent): permit reads electric/cyan, deny reads
# violet. Neutral ink and a pale wash support legibility on white.
PERMIT, DENY = ELECTRIC, VIOLET
INK, MUTED, PALE = "#23306B", "#8089B3", "#EEF2FF"
# Convenience cycle for multi-node diagrams.
CYCLE = (INDIGO, ELECTRIC, CYAN, VIOLET, CYAN_BRIGHT, ELECTRIC)
BRAND_CMAP = LinearSegmentedColormap.from_list(
    "argorix", [CYAN_BRIGHT, CYAN, ELECTRIC, INDIGO, VIOLET]
)

plt.rcParams.update({
    "font.family": "DejaVu Sans", "font.size": 9, "pdf.fonttype": 42,
    "axes.titleweight": "bold", "axes.titlesize": 11,
    "axes.edgecolor": MUTED, "text.color": INK,
    "axes.labelcolor": INK, "xtick.color": INK, "ytick.color": INK,
})
PDF_META = {"Creator": "Argorix paper pipeline", "Producer": "Matplotlib",
            "CreationDate": None, "ModDate": None}


def canvas(title: str, wide: bool = False):
    fig, ax = plt.subplots(figsize=(7.2 if wide else 3.35, 3.6))
    fig.patch.set_facecolor("white")
    ax.set(xlim=(0, 10), ylim=(0, 10))
    ax.axis("off")
    ax.set_title(title, color=INDIGO, pad=8)
    return fig, ax


def velocity_field(ax, hue=CYAN, density=7):
    """Faint DanceOPD-style velocity field behind a conceptual diagram."""
    for gx in range(density):
        for gy in range(density):
            x = 0.7 + gx * (8.6 / (density - 1))
            y = 0.9 + gy * (8.2 / (density - 1))
            ang = 0.45 * math.sin(0.6 * x) + 0.25 * math.cos(0.5 * y)
            dx, dy = 0.42 * math.cos(ang), 0.42 * math.sin(ang)
            ax.add_patch(FancyArrowPatch(
                (x, y), (x + dx, y + dy), arrowstyle="-|>", mutation_scale=6,
                lw=0.8, color=hue, alpha=0.16, zorder=0))


def box(ax, x, y, w, h, text, color=INDIGO, dashed=False, fill=PALE):
    patch = FancyBboxPatch((x, y), w, h, boxstyle="round,pad=0.08",
                           fc=fill if not dashed else "white", ec=color, lw=1.8,
                           linestyle="--" if dashed else "-", zorder=3)
    ax.add_patch(patch)
    ax.text(x + w / 2, y + h / 2, text, ha="center", va="center",
            color=INK, fontsize=8, wrap=True)


def arrow(ax, a, b, dashed=False, color=ELECTRIC):
    ax.add_patch(FancyArrowPatch(a, b, arrowstyle="-|>", mutation_scale=11,
                                 lw=1.6, color=color, zorder=2,
                                 linestyle="--" if dashed else "-"))


def arrow_curved(ax, a, b, color=ELECTRIC, rad=-0.28):
    ax.add_patch(FancyArrowPatch(a, b, arrowstyle="-|>", mutation_scale=12,
                                 lw=1.9, color=color, zorder=2,
                                 connectionstyle=f"arc3,rad={rad}"))


def legend_row(ax, y, items):
    """Compact DanceOPD-style legend: colored swatch + label, evenly spaced."""
    xs = [1.1 + i * (8.0 / max(len(items) - 1, 1)) for i in range(len(items))]
    for x, (label, color) in zip(xs, items):
        ax.plot([x, x + 0.42], [y, y], color=color, lw=3.2, solid_capstyle="round",
                zorder=3)
        ax.text(x + 0.55, y, label, va="center", ha="left", fontsize=8, color=INK)


def save(fig, path):
    try:
        atomic_publish(
            path,
            lambda temporary: fig.savefig(
                temporary, format="pdf", bbox_inches="tight", metadata=PDF_META
            ),
        )
    finally:
        plt.close(fig)


def architecture(path):
    fig, ax = plt.subplots(figsize=(7.2, 3.05))
    fig.patch.set_facecolor("white")
    ax.set(xlim=(0, 10), ylim=(0, 6))
    ax.axis("off")
    ax.set_title("ArgorixLang compilation and evidence architecture",
                 color=INDIGO, pad=8)
    velocity_field(ax)
    labels = ["Argorix\nsource", "Parser +\nsemantics", "Typed IR +\nbytecode",
              "Fail-closed\nVM", "Trace +\nledger", "Evidence +\nreports"]
    colors = [INDIGO, ELECTRIC, CYAN, VIOLET, ELECTRIC, INDIGO]
    bw, by, bh = 1.35, 3.05, 1.55
    for i, label in enumerate(labels):
        x = .2 + i * 1.62
        box(ax, x, by, bw, bh, label, colors[i])
        # icon accent in the brand color of the stage
        ax.add_patch(plt.Circle((x + 0.2, by + bh - 0.22), 0.085, color=colors[i],
                                zorder=5))
        if i:
            arrow_curved(ax, (x - .27, by + bh / 2), (x, by + bh / 2), colors[i])
    ax.text(5, 1.95,
            "Offline runtime boundary - simulated is the sole executable provider",
            ha="center", color=MUTED, fontsize=9)
    legend_row(ax, 0.9, [
        ("Compilation flow", ELECTRIC),
        ("Fail-closed boundary", VIOLET),
        ("Offline evidence", CYAN),
    ])
    save(fig, path)


def request_sequence(path):
    fig, ax = canvas("Request lifecycle: controls precede execution", True)
    xs = [1, 3, 5, 7, 9]
    for x, label in zip(xs, ["Caller", "Compiler", "Policy", "VM", "Evidence"]):
        ax.text(x, 8.8, label, ha="center", color=INDIGO, weight="bold")
        ax.plot([x, x], [1.2, 8.3], color=PALE, lw=2, zorder=1)
    steps = [(1, 3, "parse + validate"), (3, 5, "evaluate controls"),
             (5, 7, "permit / deny"), (7, 9, "append events"),
             (9, 1, "bundle result")]
    for row, (a, b, t) in enumerate(steps):
        y = 7.5 - row * 1.25
        arrow(ax, (a, y), (b, y), color=CYCLE[row % len(CYCLE)])
        ax.text((a + b) / 2, y + .18, t, ha="center", fontsize=8, color=MUTED)
    save(fig, path)


def state_machine(path):
    fig, ax = plt.subplots(figsize=(7.2, 2.55))
    fig.patch.set_facecolor("white")
    ax.set(xlim=(0, 10), ylim=(0, 5))
    ax.axis("off")
    ax.set_title("Fail-closed decision state machine", color=INDIGO, pad=8)
    bw, bh = 2.2, 1.1
    nodes = {"Declared": (0.5, 2.85, INDIGO), "Validated": (3.7, 2.85, INDIGO),
             "Permitted": (7.0, 2.85, PERMIT), "Denied": (3.7, 0.5, DENY)}
    for t, (x, y, c) in nodes.items():
        box(ax, x, y, bw, bh, t, c)
    arrow(ax, (2.7, 3.4), (3.7, 3.4))            # Declared -> Validated
    arrow(ax, (5.9, 3.4), (7.0, 3.4), color=PERMIT)   # Validated -> Permitted
    arrow(ax, (4.8, 2.85), (4.8, 1.6), color=DENY)    # Validated -> Denied
    ax.text(5.05, 2.2, "unknown / invalid -> deny", ha="left", color=DENY,
            fontsize=8)
    save(fig, path)


def session_outcomes(summary, out):
    """Ordered horizontal bars plus a compact key-metrics panel (DanceOPD-style)."""
    complete = int(summary["complete_sessions"])
    incomplete = int(summary["incomplete_sessions"])
    total = complete + incomplete
    data = sorted(
        [("Complete", complete, ELECTRIC), ("Source-only", incomplete, VIOLET)],
        key=lambda d: d[1], reverse=True,
    )
    mx = max(d[1] for d in data) or 1

    fig = plt.figure(figsize=(6.9, 2.9), constrained_layout=True)
    grid = fig.add_gridspec(1, 2, width_ratios=[2.4, 1])
    ax = fig.add_subplot(grid[0, 0])
    tx = fig.add_subplot(grid[0, 1])
    tx.axis("off")
    ys = list(range(len(data)))
    ax.barh(ys, [d[1] for d in data], color=[d[2] for d in data], height=0.55,
            zorder=3)
    ax.set_yticks(ys)
    ax.set_yticklabels([d[0] for d in data], fontsize=9)
    ax.invert_yaxis()
    ax.set_xlim(0, mx * 1.18)
    for y, d in zip(ys, data):
        ax.text(d[1] + mx * 0.02, y, str(d[1]), va="center", fontsize=10,
                weight="bold", color=INK)
    ax.set_xlabel("Request directories")
    ax.spines[["top", "right"]].set_visible(False)
    ax.set_title("Normalized session outcomes", color=INDIGO, weight="bold")

    def pct(v):
        return f"{100 * v / total:.1f}%" if total else "0.0%"
    rows = [("Total directories", str(total)),
            ("Complete", f"{complete} ({pct(complete)})"),
            ("Source-only", f"{incomplete} ({pct(incomplete)})")]
    tx.text(0.0, 0.92, "Key metrics", fontsize=10, weight="bold", color=INDIGO,
            transform=tx.transAxes)
    for i, (k, v) in enumerate(rows):
        y = 0.66 - i * 0.22
        tx.text(0.0, y, k, fontsize=8.5, color=MUTED, transform=tx.transAxes)
        tx.text(1.0, y, v, fontsize=9, color=INK, ha="right", weight="bold",
                transform=tx.transAxes)
    save(fig, out / "session-outcomes.pdf")


def policy_bubbles(data, out):
    """DanceOPD-style capability bubbles for policy/evidence event volume."""
    with (data / "sessions.csv").open(encoding="utf-8", newline="") as f:
        rows = list(csv.DictReader(f))
    completed = [r for r in rows if r.get("complete", "").lower() == "true"]
    violations = sum(int(r.get("policy_violation_count") or 0) for r in completed)
    ledger = sum(int(r.get("ledger_events_total") or 0) for r in completed)
    labels = ["Policy\nviolations", "Ledger\nevents"]
    values = [violations, ledger]

    fig = plt.figure(figsize=(7.0, 3.3), constrained_layout=True)
    grid = fig.add_gridspec(1, 2, width_ratios=[15, 1], wspace=.35)
    ax = fig.add_subplot(grid[0, 0])
    cax = fig.add_subplot(grid[0, 1])
    xs = [0.32, 0.68]
    span = max(values) - min(values) or 1
    sizes = [1400 + 5200 * (v - min(values)) / span for v in values]
    sc = ax.scatter(xs, values, s=sizes, c=values, cmap=BRAND_CMAP,
                    edgecolors="white", linewidths=1.6, zorder=3,
                    vmin=min(values), vmax=max(values))
    for x, v in zip(xs, values):
        ax.text(x, v, f"{v:,}", ha="center", va="center", color="white",
                fontsize=9, weight="bold", zorder=4)
    ax.set_xlim(0, 1)
    ax.set_ylim(-max(values) * 0.18, max(values) * 1.22)
    ax.set_xticks(xs)
    ax.set_xticklabels(labels, fontsize=9)
    ax.set_yticks([])
    ax.set_ylabel("Observed total (complete sessions)")
    ax.spines[["top", "right", "left"]].set_visible(False)
    ax.set_title("Policy and evidence event volume", color=INDIGO,
                 weight="bold")
    cbar = fig.colorbar(sc, cax=cax)
    cbar.set_ticks([2000, 4000, 6000])
    cbar.outline.set_edgecolor(MUTED)
    save(fig, out / "policy-heatmap.pdf")


def wrap_label(label):
    if "\n" in label:
        return label
    words = label.split()
    if len(words) <= 1:
        return label
    midpoint = (len(words) + 1) // 2
    return " ".join(words[:midpoint]) + "\n" + " ".join(words[midpoint:])


def flow_figure(path, title, labels, proposed=None, field=False):
    fig, ax = canvas(title, True)
    if field:
        velocity_field(ax)
    n = len(labels)
    gap = 9.55 / n
    for i, label in enumerate(labels):
        x = .15 + i * gap
        is_prop = proposed is not None and i in proposed
        shown = wrap_label(label)
        color = CYCLE[i % len(CYCLE)]
        if is_prop:
            width = gap - .6
            box(ax, x, 4.1, width, 1.8, "", color, True)
            ax.text(x + width / 2, 5.38, "PROPOSED /\nNOT IMPLEMENTED",
                    ha="center", va="center", fontsize=7, color=MUTED,
                    weight="bold")
            ax.text(x + width / 2, 4.60, shown, ha="center", va="center",
                    fontsize=8, color=INK)
        else:
            box(ax, x, 4.1, gap - .6, 1.8, shown, color)
        if i:
            arrow(ax, (x - .48, 5), (x, 5), is_prop)
    save(fig, path)


def evidence_verification_scope(path):
    fig, ax = canvas("Offline evidence verification scope", True)
    velocity_field(ax)
    nodes = [
        (.12, "Source\n(upstream;\nnot bundle-verified)", MUTED, True),
        (2.05, "Bytecode", INDIGO, False),
        (3.85, "Trace +\nevents", ELECTRIC, False),
        (5.65, "Security\nreport", CYAN, False),
        (7.70, "Evidence\nbundle", VIOLET, False),
    ]
    widths = [1.48, 1.25, 1.25, 1.35, 1.55]
    for (x, label, color, dashed), width in zip(nodes, widths):
        box(ax, x, 4.55, width, 1.75, label, color, dashed)
    arrow(ax, (1.60, 5.42), (2.05, 5.42), dashed=True, color=MUTED)
    arrow(ax, (3.30, 5.42), (3.85, 5.42), color=ELECTRIC)
    arrow(ax, (5.10, 5.42), (5.65, 5.42), color=CYAN)
    arrow(ax, (7.00, 5.42), (7.70, 5.42), color=VIOLET)
    ax.text(
        5.0, 2.85,
        "Bundle checks bytecode, trace, and report digests",
        ha="center", color=INK, fontsize=8, weight="bold",
    )
    ax.text(
        5.0, 2.10,
        "ledger_digest = digest(trace.events); report LedgerSummary must match",
        ha="center", color=MUTED, fontsize=8,
    )
    save(fig, path)


def generate(data: Path, out: Path):
    out.mkdir(parents=True, exist_ok=True)
    summary = json.loads((data / "runtime_summary.json").read_text(encoding="utf-8"))
    architecture(out / "architecture.pdf")
    request_sequence(out / "request-sequence.pdf")
    state_machine(out / "decision-state-machine.pdf")
    session_outcomes(summary, out)
    policy_bubbles(data, out)
    evidence_verification_scope(out / "evidence-chain.pdf")
    flow_figure(out / "trust-relationships.pdf", "Declarative trust relationships",
                ["Agent identity", "Passport", "ATrust map", "Trust ledger",
                 "Claim boundary"], field=True)
    flow_figure(out / "threat-mitigation.pdf", "Threat-to-control mapping",
                ["External execution", "Deny by default", "Network denied",
                 "Secrets denied", "Review evidence"])
    flow_figure(out / "evolution-timeline.pdf",
                "Language evolution and bounded future work",
                ["Core runtime", "Provider contracts", "Evidence + governance",
                 "Operational federation"], {3})
    flow_figure(out / "sovereign-discovery.pdf", "Sovereign discovery boundary",
                ["Local declaration", "Semantic validation", "Local ans_name",
                 "Operational DNS"], {3})
    flow_figure(out / "artifact-schema.pdf", "Normalized artifact relationships",
                ["session.\nargx", "session.\nargbc.json", "session.\ntrace.json",
                 "session.\nsecurity.json", "session.\nevidence.json"])
    flow_figure(out / "claim-boundaries.pdf", "Claim boundary taxonomy",
                ["Implemented", "Declarative", "Proposed", "Not claimed"], {2})


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--data", type=Path, required=True)
    p.add_argument("--output", type=Path, required=True)
    a = p.parse_args()
    generate(a.data, a.output)


if __name__ == "__main__":
    main()
