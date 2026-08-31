"""Render the adversarial measurement-boundary figure from `summary.json`.

Vector only, no decoration, and every count comes from the campaign summary so
the figure cannot drift from the numbers in the tables.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import matplotlib

matplotlib.use("pdf")
import matplotlib.pyplot as plt  # noqa: E402
from matplotlib.patches import FancyArrowPatch, FancyBboxPatch, Rectangle  # noqa: E402

INK = "#1F2937"
MUTED = "#6B7280"
LINE = "#9CA3AF"
BLUE = "#2563EB"
GREEN = "#059669"
RED = "#DC2626"
AMBER = "#D97706"
PALE_BLUE = "#EFF6FF"
PALE_GREEN = "#ECFDF5"
PALE_RED = "#FEF2F2"
PALE_AMBER = "#FFFBEB"

plt.rcParams.update(
    {
        "font.family": "DejaVu Sans",
        "font.size": 8.5,
        "pdf.fonttype": 42,
        "axes.titlesize": 11,
        "axes.titleweight": "bold",
        "text.color": INK,
    }
)
PDF_META = {"Creator": "Argorix adversarial campaign", "CreationDate": None, "ModDate": None}


def box(ax, x, y, w, h, label, edge=BLUE, fill=PALE_BLUE, dashed=False, fontsize=8.0):
    ax.add_patch(
        FancyBboxPatch(
            (x, y),
            w,
            h,
            boxstyle="round,pad=0.06",
            fc=fill,
            ec=edge,
            lw=1.4,
            linestyle="--" if dashed else "-",
            zorder=3,
        )
    )
    ax.text(x + w / 2, y + h / 2, label, ha="center", va="center", fontsize=fontsize)


def boundary(ax, x, y, w, h, label, edge=LINE, dotted=False):
    ax.add_patch(
        Rectangle(
            (x, y),
            w,
            h,
            fc="none",
            ec=edge,
            lw=1.2,
            linestyle=":" if dotted else "--",
            zorder=1,
        )
    )
    ax.text(x + 0.12, y + h + 0.06, label, ha="left", va="bottom", color=MUTED, fontsize=8.0)


def arrow(ax, start, end, color=BLUE, dashed=False, lw=1.4):
    ax.add_patch(
        FancyArrowPatch(
            start,
            end,
            arrowstyle="-|>",
            mutation_scale=10,
            lw=lw,
            color=color,
            linestyle="--" if dashed else "-",
            zorder=2,
        )
    )


def render(summary: dict, path: Path) -> None:
    metrics = summary["primary_metrics"]
    rejected = metrics["boundary_rejection"]
    sink = metrics["destination_asr"]
    tamper = metrics["tamper_detection"]

    # Column-width aspect: the figure is placed at \columnwidth, so a narrow
    # canvas keeps the rendered type size legible.
    fig, ax = plt.subplots(figsize=(3.6, 4.3))
    fig.patch.set_facecolor("white")
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 12)
    ax.axis("off")

    boundary(ax, 0.1, 7.5, 9.8, 3.7, "mediation inside the release", RED)
    stages = [
        ("Semantic\ncheck", 0.7, 9.7),
        ("Bytecode\nverifier", 5.3, 9.7),
        ("Provider\nregistry", 0.7, 8.3),
        ("Dry-run\nVM", 5.3, 8.3),
    ]
    for label, x, y in stages:
        box(ax, x, y, 4.0, 1.15, label, RED, PALE_RED, fontsize=8.5)
    arrow(ax, (4.75, 10.28), (5.3, 10.28), RED)
    arrow(ax, (2.7, 9.65), (2.7, 9.5), RED)
    arrow(ax, (4.75, 8.88), (5.3, 8.88), RED)
    ax.text(
        5.0,
        7.75,
        f"{rejected['text']} prohibited conditions rejected here",
        ha="center",
        color=INK,
        fontsize=7.8,
    )

    arrow(ax, (8.6, 7.45), (8.6, 6.85), GREEN, dashed=True)

    boundary(ax, 0.1, 3.9, 9.8, 2.9, "sensors outside the release", GREEN)
    box(ax, 0.7, 5.4, 4.0, 1.15, "loopback\nsink", GREEN, PALE_GREEN, fontsize=8.5)
    box(
        ax,
        5.3,
        5.4,
        4.0,
        1.15,
        "file + secret\ncanaries",
        GREEN,
        PALE_GREEN,
        fontsize=8.5,
    )
    ax.text(
        5.0,
        4.85,
        f"{sink['text']} prohibited proposals reached a sensor",
        ha="center",
        color=INK,
        fontsize=7.8,
    )
    ax.text(
        5.0,
        4.25,
        "every positive control fired",
        ha="center",
        color=MUTED,
        fontsize=8.0,
    )

    box(
        ax,
        0.1,
        2.5,
        9.8,
        1.0,
        f"evidence verifier detects {tamper['text']} post-generation\nmodifications of the referenced artifacts",
        BLUE,
        PALE_BLUE,
        fontsize=7.5,
    )
    box(
        ax,
        0.1,
        1.35,
        9.8,
        1.0,
        "not detected: source-only edit (no source binding);\n"
        "coordinated unsigned replacement (no trust anchor)",
        AMBER,
        PALE_AMBER,
        dashed=True,
        fontsize=7.5,
    )
    ax.text(
        5.0,
        0.42,
        "The registry admits no instrumentable adapter, so adapter\n"
        "non-reachability is out of scope; the sensors observe the\n"
        "release process, not the operating system.",
        ha="center",
        va="center",
        color=MUTED,
        fontsize=7.6,
    )

    path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(path, format="pdf", bbox_inches="tight", metadata=PDF_META)
    plt.close(fig)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Render the adversarial boundary figure")
    parser.add_argument("--summary", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args(argv)
    summary = json.loads(Path(args.summary).read_text(encoding="utf-8"))
    render(summary, Path(args.out))
    print(f"figure written to {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
