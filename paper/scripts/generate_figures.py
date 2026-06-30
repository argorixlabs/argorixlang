"""Generate the paper's claim-bounded, vector-only academic figures.

The visual grammar is deliberately sober: white background, dark ink, muted
blue/gray controls, solid arrows for implemented flow, dashed boxes for
proposed/not implemented elements, and dotted boundaries for material outside
the current claim.  No bubble charts or decorative fields are used.
"""
from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path

import matplotlib

matplotlib.use("pdf")
import matplotlib.pyplot as plt
from matplotlib.patches import FancyArrowPatch, FancyBboxPatch, Rectangle

from atomic_io import atomic_publish


INK = "#1F2937"
MUTED = "#6B7280"
LINE = "#9CA3AF"
BLUE = "#2563EB"
GREEN = "#059669"
RED = "#DC2626"
AMBER = "#D97706"
PURPLE = "#7C3AED"
GRAY = "#F3F4F6"
PALE_BLUE = "#EFF6FF"
PALE_GREEN = "#ECFDF5"
PALE_RED = "#FEF2F2"
PALE_AMBER = "#FFFBEB"
PALE_PURPLE = "#F5F3FF"

plt.rcParams.update({
    "font.family": "DejaVu Sans",
    "font.size": 8.5,
    "pdf.fonttype": 42,
    "axes.titlesize": 11,
    "axes.titleweight": "bold",
    "text.color": INK,
    "axes.labelcolor": INK,
    "xtick.color": INK,
    "ytick.color": INK,
})
PDF_META = {
    "Creator": "Argorix paper pipeline",
    "Producer": "Matplotlib",
    "CreationDate": None,
    "ModDate": None,
}


def save(fig, path: Path) -> None:
    try:
        atomic_publish(
            path,
            lambda temporary: fig.savefig(
                temporary, format="pdf", bbox_inches="tight", metadata=PDF_META
            ),
        )
    finally:
        plt.close(fig)


def base(title: str, height: float = 3.2):
    fig, ax = plt.subplots(figsize=(7.1, height))
    fig.patch.set_facecolor("white")
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 6)
    ax.axis("off")
    ax.set_title(title, color=INK, pad=8)
    return fig, ax


def box(ax, x, y, w, h, label, edge=BLUE, fill=PALE_BLUE, dashed=False, lw=1.4):
    patch = FancyBboxPatch(
        (x, y),
        w,
        h,
        boxstyle="round,pad=0.06",
        fc=fill,
        ec=edge,
        lw=lw,
        linestyle="--" if dashed else "-",
        zorder=3,
    )
    ax.add_patch(patch)
    ax.text(x + w / 2, y + h / 2, label, ha="center", va="center", fontsize=8.0)
    return patch


def boundary(ax, x, y, w, h, label, edge=LINE, dotted=False):
    rect = Rectangle(
        (x, y),
        w,
        h,
        fc="none",
        ec=edge,
        lw=1.2,
        linestyle=":" if dotted else "--",
        zorder=1,
    )
    ax.add_patch(rect)
    ax.text(x + 0.1, y + h - 0.18, label, ha="left", va="top", color=MUTED, fontsize=8.0)
    return rect


def arrow(ax, start, end, color=BLUE, dashed=False, lw=1.4, rad=0):
    ax.add_patch(
        FancyArrowPatch(
            start,
            end,
            arrowstyle="-|>",
            mutation_scale=10,
            lw=lw,
            color=color,
            linestyle="--" if dashed else "-",
            connectionstyle=f"arc3,rad={rad}",
            zorder=2,
        )
    )


def system_pipeline(path: Path) -> None:
    fig, ax = base("Figure 1. System pipeline and evidence boundary", 3.1)
    boundary(ax, 0.25, 2.25, 5.25, 2.55, "implemented compilation flow", BLUE)
    boundary(ax, 5.65, 2.25, 1.8, 2.55, "fail-closed runtime boundary", RED)
    boundary(ax, 7.65, 2.25, 2.1, 2.55, "offline evidence boundary", GREEN)
    labels = [
        ("Argorix\nsource", 0.45, BLUE, PALE_BLUE),
        ("Parser +\nsemantics", 1.95, BLUE, PALE_BLUE),
        ("Typed IR /\nbytecode", 3.45, BLUE, PALE_BLUE),
        ("Fail-closed\nVM", 5.85, RED, PALE_RED),
        ("Trace /\nledger", 7.75, GREEN, PALE_GREEN),
        ("Evidence /\nreports", 8.9, GREEN, PALE_GREEN),
    ]
    for label, x, color, fill in labels:
        box(ax, x, 3.05, 1.05, 0.85, label, color, fill)
    for x1, x2, color in [(1.5, 1.95, BLUE), (3.0, 3.45, BLUE), (4.5, 5.85, RED), (6.9, 7.75, GREEN), (8.8, 8.9, GREEN)]:
        arrow(ax, (x1, 3.47), (x2, 3.47), color)
    box(ax, 5.85, 1.15, 1.75, 0.65, "simulated only\nexecutable provider", RED, PALE_RED)
    arrow(ax, (6.55, 3.05), (6.55, 1.8), RED)
    ax.text(
        5,
        0.45,
        "Solid paths denote implemented compilation/runtime behavior; metadata layers do not imply external verification.",
        ha="center",
        color=MUTED,
        fontsize=8,
    )
    save(fig, path)


def provider_boundary(path: Path) -> None:
    fig, ax = base("Figure 2. Provider boundary and request lifecycle", 3.25)
    boundary(ax, 0.25, 3.0, 9.45, 1.55, "implemented request lifecycle", BLUE)
    for label, x in [
        ("Caller", 0.55),
        ("Compiler\nvalidation", 2.0),
        ("Policy\nlattice", 3.65),
        ("VM", 5.25),
        ("Executable\nregistry", 6.65),
        ("Evidence", 8.35),
    ]:
        box(ax, x, 3.35, 1.1, 0.75, label)
    for x1, x2 in [(1.65, 2.0), (3.1, 3.65), (4.75, 5.25), (6.35, 6.65), (7.75, 8.35)]:
        arrow(ax, (x1, 3.72), (x2, 3.72), BLUE)
    boundary(ax, 0.9, 1.0, 3.65, 1.25, "declarative provider contract", LINE, dotted=True)
    box(ax, 1.15, 1.35, 1.35, 0.55, "OpenAIProvider\ncontract", LINE, GRAY, dashed=True)
    box(ax, 2.85, 1.35, 1.35, 0.55, "external contract\nnot callable", LINE, GRAY, dashed=True)
    boundary(ax, 5.55, 1.0, 3.35, 1.25, "executable provider adapter", RED)
    box(ax, 5.85, 1.35, 1.25, 0.55, "simulated", RED, PALE_RED)
    box(ax, 7.35, 1.35, 1.15, 0.55, "external\nadapter absent", LINE, GRAY, dashed=True)
    arrow(ax, (4.2, 1.62), (5.85, 1.62), RED, dashed=True)
    ax.text(5.0, 0.45, "Provider contracts are declarations; only registered executable adapters may be called.", ha="center", color=MUTED)
    save(fig, path)


def policy_lattice(path: Path) -> None:
    fig, ax = base("Figure 3. Typed policy lattice v0.2", 3.45)
    box(ax, 0.4, 3.0, 1.25, 0.7, "Policy\ninput", BLUE, PALE_BLUE)
    box(ax, 2.25, 4.15, 1.25, 0.65, "known\nrule", GREEN, PALE_GREEN)
    box(ax, 2.25, 3.0, 1.25, 0.65, "unknown\nrule", AMBER, PALE_AMBER)
    box(ax, 2.25, 1.85, 1.25, 0.65, "malformed\nobject", RED, PALE_RED)
    for end in [(2.25, 4.47), (2.25, 3.32), (2.25, 2.17)]:
        arrow(ax, (1.65, 3.35), end, BLUE)
    outcomes = [
        ("PASS", "provider\ncheck", 5.0, 5.0, GREEN, PALE_GREEN),
        ("DENY", "stop", 5.0, 4.0, RED, PALE_RED),
        ("REVIEW", "human\nreview", 5.0, 3.0, AMBER, PALE_AMBER),
        ("UNKNOWN_RULE", "configuration\ndiagnostic", 5.0, 2.0, PURPLE, PALE_PURPLE),
        ("ERROR", "parse/semantic\nfailure", 5.0, 1.0, RED, PALE_RED),
    ]
    for label, desc, x, y, color, fill in outcomes:
        box(ax, x, y, 1.35, 0.58, label, color, fill)
        ax.text(x + 1.55, y + 0.29, desc, va="center", color=MUTED, fontsize=8.0)
    for y in [5.29, 4.29, 3.29]:
        arrow(ax, (3.5, 4.47), (5.0, y), GREEN)
    arrow(ax, (3.5, 3.32), (5.0, 2.29), AMBER)
    arrow(ax, (3.5, 2.17), (5.0, 1.29), RED)
    ax.text(5, 0.35, "UNKNOWN_RULE is a configuration diagnostic, not an ordinary policy violation.", ha="center", color=MUTED)
    save(fig, path)


def evidence_scope(path: Path) -> None:
    fig, ax = base("Figure 4. EvidenceBundle verification scope", 3.45)
    boundary(ax, 0.4, 3.25, 9.0, 1.45, "historical v0.1 bundle verification", GREEN)
    box(ax, 0.75, 3.68, 1.35, 0.55, "session.argx\nsource", LINE, GRAY, dashed=True)
    for label, x in [("bytecode", 2.65), ("trace", 4.05), ("security\nreport", 5.45), ("ledger\ndigest", 6.95), ("Evidence\nBundle", 8.2)]:
        box(ax, x, 3.68, 1.0, 0.55, label, GREEN, PALE_GREEN)
    arrow(ax, (2.1, 3.96), (2.65, 3.96), LINE, dashed=True)
    for x1, x2 in [(3.65, 4.05), (5.05, 5.45), (6.45, 6.95), (7.95, 8.2)]:
        arrow(ax, (x1, 3.96), (x2, 3.96), GREEN)
    ax.text(1.42, 3.18, "outside v0.1 bundle verification", ha="center", va="top", color=MUTED, fontsize=8.0)
    boundary(ax, 0.4, 1.25, 9.0, 1.35, "controlled v0.2 matrix extension", BLUE)
    box(ax, 1.1, 1.62, 1.3, 0.55, "session.argx\nsource", BLUE, PALE_BLUE)
    box(ax, 3.1, 1.62, 1.3, 0.55, "source_digest", BLUE, PALE_BLUE)
    box(ax, 5.0, 1.62, 1.55, 0.55, "generated\nartifact set", BLUE, PALE_BLUE)
    box(ax, 7.25, 1.62, 1.3, 0.55, "mismatch ->\nVERIFIER_FAIL", RED, PALE_RED)
    for x1, x2, color in [(2.4, 3.1, BLUE), (4.4, 5.0, BLUE), (6.55, 7.25, RED)]:
        arrow(ax, (x1, 1.9), (x2, 1.9), color)
    ax.text(5, 0.45, "v0.1 checks internal consistency; v0.2 fixtures additionally test source-digest matching.", ha="center", color=MUTED)
    save(fig, path)


def claim_boundaries(path: Path) -> None:
    fig, ax = base("Figure 5. Claim boundary taxonomy", 3.1)
    groups = [
        ("Implemented", "VM\ntraces\nEvidenceBundle", GREEN, PALE_GREEN, False),
        ("Declarative", "passports\nans_name\nATrust map", BLUE, PALE_BLUE, False),
        ("Proposed /\nnot implemented", "ANS/DNS\nDID/VC resolver\nsigned bundles", AMBER, PALE_AMBER, True),
        ("Not claimed", "certification\nattestation\nreal identity", RED, PALE_RED, True),
    ]
    for i, (title, examples, color, fill, dashed) in enumerate(groups):
        x = 0.55 + i * 2.35
        box(ax, x, 3.1, 1.75, 0.75, title, color, fill, dashed=dashed)
        box(ax, x, 1.55, 1.75, 1.05, examples, color, "white", dashed=dashed)
        if i:
            arrow(ax, (x - 0.35, 3.47), (x, 3.47), LINE, dashed=True)
    ax.text(5, 0.55, "Digest success is not upgraded into approval, attestation, certification, or real-world identity.", ha="center", color=MUTED)
    save(fig, path)


def controlled_matrix_outcomes(path: Path, results_root: Path) -> None:
    with (results_root / "controlled_matrix.json").open(encoding="utf-8") as handle:
        summary = json.load(handle)["summary"]
    labels = ["PASS", "DENY", "REVIEW", "UNKNOWN_RULE", "ERROR", "VERIFIER_FAIL"]
    values = [
        summary["pass_outcomes"],
        summary["deny_outcomes"],
        summary["review_outcomes"],
        summary["unknown_rule_outcomes"],
        summary["error_outcomes"],
        summary["verifier_fail_outcomes"],
    ]
    colors = [GREEN, RED, AMBER, PURPLE, RED, INK]
    fig, ax = plt.subplots(figsize=(7.1, 3.2))
    fig.patch.set_facecolor("white")
    bars = ax.bar(labels, values, color=colors, width=0.62)
    ax.set_title("Figure 6. Controlled matrix outcome distribution", color=INK, pad=8)
    ax.set_ylabel("Deterministic cases")
    ax.spines[["top", "right"]].set_visible(False)
    ax.grid(axis="y", color="#E5E7EB", lw=0.8)
    ax.set_axisbelow(True)
    ax.set_ylim(0, max(values) + 6)
    ax.tick_params(axis="x", labelrotation=18)
    for bar, value in zip(bars, values):
        ax.text(bar.get_x() + bar.get_width() / 2, value + 0.7, str(value), ha="center", va="bottom", fontsize=8.5, weight="bold")
    ax.text(
        0.5,
        -0.32,
        f"{summary['total_rows']} deterministic controlled cases; local metadata and verifier outcomes only.",
        transform=ax.transAxes,
        ha="center",
        color=MUTED,
        fontsize=8,
    )
    save(fig, path)


def threat_control_mapping(path: Path) -> None:
    fig, ax = base("Figure 7. Threat-to-control mapping", 3.8)
    headers = [("Threat", 0.6), ("Control", 4.0), ("Claim status", 7.35)]
    for text, x in headers:
        ax.text(x, 5.35, text, weight="bold", ha="left", color=INK, fontsize=9)
    rows = [
        ("unauthorized\nprovider execution", "executable-provider\nallowlist", "implemented"),
        ("network\nside effects", "network denied\nprofile", "implemented"),
        ("secret/key\ndisclosure", "secrets/key material\ndenied", "implemented"),
        ("artifact\ntampering", "EvidenceBundle\ndigests", "implemented"),
        ("policy\nconfusion", "typed policy\nlattice", "controlled matrix"),
        ("source\nmismatch", "source_digest", "controlled matrix"),
    ]
    y = 4.65
    for i, (threat, control, status) in enumerate(rows):
        fill = PALE_GREEN if status == "implemented" else PALE_BLUE
        edge = GREEN if status == "implemented" else BLUE
        box(ax, 0.55, y, 2.25, 0.45, threat, LINE, "white")
        box(ax, 3.7, y, 2.3, 0.45, control, edge, fill)
        box(ax, 7.2, y, 1.85, 0.45, status, edge, fill)
        arrow(ax, (2.8, y + 0.22), (3.7, y + 0.22), LINE)
        arrow(ax, (6.0, y + 0.22), (7.2, y + 0.22), LINE)
        y -= 0.66
    box(ax, 7.2, 0.45, 1.85, 0.45, "future assurance:\nsigned bundles", AMBER, PALE_AMBER, dashed=True)
    ax.text(5, 0.12, "Mapping means a control is present or exercised; it is not an adversarial success-rate measurement.", ha="center", color=MUTED, fontsize=8)
    save(fig, path)


def generate(data: Path, out: Path) -> None:
    out.mkdir(parents=True, exist_ok=True)
    repo = Path(__file__).resolve().parents[2]
    system_pipeline(out / "system-pipeline.pdf")
    provider_boundary(out / "provider-boundary.pdf")
    policy_lattice(out / "policy-lattice-flow.pdf")
    evidence_scope(out / "evidence-scope.pdf")
    claim_boundaries(out / "claim-boundaries.pdf")
    controlled_matrix_outcomes(out / "controlled-matrix-outcomes.pdf", repo / "results")
    threat_control_mapping(out / "threat-control-mapping.pdf")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--data", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    generate(args.data, args.output)


if __name__ == "__main__":
    main()
