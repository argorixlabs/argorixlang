"""Static integrity checks for the modular ArgorixLang manuscript."""

from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MAIN = ROOT / "main.tex"
EXPECTED_SECTIONS = [
    "abstract",
    "introduction",
    "background",
    "language-architecture",
    "sovereign-identity",
    "atrust-dcp",
    "runtime",
    "evidence",
    "methodology",
    "results",
    "security-analysis",
    "discussion",
    "limitations",
    "future-work",
    "conclusion",
]
EXPECTED_FIGURES = {
    "architecture.pdf",
    "artifact-schema.pdf",
    "claim-boundaries.pdf",
    "decision-state-machine.pdf",
    "evidence-chain.pdf",
    "evolution-timeline.pdf",
    "policy-heatmap.pdf",
    "request-sequence.pdf",
    "session-outcomes.pdf",
    "sovereign-discovery.pdf",
    "threat-mitigation.pdf",
    "trust-relationships.pdf",
}
EXPECTED_TABLES = {
    "claim-boundaries",
    "dataset-inventory",
    "empirical-results",
    "language-constructs",
    "related-work",
    "runtime-controls",
    "threat-mapping",
}


def fail(message: str) -> None:
    raise SystemExit(f"manuscript check failed: {message}")


def main() -> None:
    main_text = MAIN.read_text(encoding="utf-8")
    for section in EXPECTED_SECTIONS:
        if rf"\input{{sections/{section}}}" not in main_text:
            fail(f"missing modular input: {section}")

    tex_paths = [MAIN, *sorted((ROOT / "sections").glob("*.tex")),
                 *sorted((ROOT / "appendices").glob("*.tex"))]
    text = "\n".join(path.read_text(encoding="utf-8") for path in tex_paths)

    figures = set(re.findall(r"\\includegraphics(?:\[[^\]]*\])?\{([^}]+)\}", text))
    if figures != EXPECTED_FIGURES:
        fail(f"figure set differs: expected {sorted(EXPECTED_FIGURES)}, got {sorted(figures)}")
    for figure in figures:
        if not (ROOT / "figures" / figure).is_file():
            fail(f"missing figure file: {figure}")

    tables = set(re.findall(r"\\input\{tables/([^}]+)\}", text))
    if tables != EXPECTED_TABLES:
        fail(f"table set differs: expected {sorted(EXPECTED_TABLES)}, got {sorted(tables)}")
    for table in tables:
        if not (ROOT / "tables" / f"{table}.tex").is_file():
            fail(f"missing table file: {table}")

    labels = set(re.findall(r"\\label\{([^}]+)\}", text))
    refs = set(re.findall(r"\\(?:c|C)?ref\{([^}]+)\}", text))
    if refs - labels:
        fail(f"unresolved labels: {sorted(refs - labels)}")

    float_labels = {label for label in labels if label.startswith(("fig:", "tab:"))}
    prose = re.sub(
        r"\\begin\{(?:figure|table)\}.*?\\end\{(?:figure|table)\}",
        "",
        text,
        flags=re.DOTALL,
    )
    prose_refs: set[str] = set()
    for group in re.findall(r"\\(?:c|C)?ref\{([^}]+)\}", prose):
        prose_refs.update(label.strip() for label in group.split(","))
    if float_labels - prose_refs:
        fail(
            "figure/table labels without non-caption prose references: "
            f"{sorted(float_labels - prose_refs)}"
        )

    bib_text = (ROOT / "references.bib").read_text(encoding="utf-8")
    bib_keys = set(re.findall(r"@\w+\{([^,]+),", bib_text))
    cited: set[str] = set()
    for group in re.findall(r"\\cite\w*\{([^}]+)\}", text):
        cited.update(key.strip() for key in group.split(","))
    if cited - bib_keys:
        fail(f"unknown citation keys: {sorted(cited - bib_keys)}")

    placeholder_pattern = re.compile(
        r"\b(?:TO" + r"DO|TBD|FIXME|PLACEHOLDER)\b", re.IGNORECASE
    )
    if placeholder_pattern.search(text):
        fail("placeholder marker found")

    absolute_path = re.compile(r"(?:[A-Za-z]:\\|/Users/|/home/)")
    if absolute_path.search(text):
        fail("absolute user path found")

    # Lightweight TeX syntax check for environments and unescaped braces. A
    # real LaTeX build remains authoritative when an engine is available.
    environments: list[str] = []
    for action, name in re.findall(r"\\(begin|end)\{([^}]+)\}", text):
        if action == "begin":
            environments.append(name)
        elif not environments or environments.pop() != name:
            fail(f"mismatched TeX environment ending at {name}")
    if environments:
        fail(f"unclosed TeX environments: {environments}")

    brace_depth = 0
    for line in text.splitlines():
        content = line.split("%", 1)[0]
        for index, char in enumerate(content):
            escaped = index > 0 and content[index - 1] == "\\"
            if char == "{" and not escaped:
                brace_depth += 1
            elif char == "}" and not escaped:
                brace_depth -= 1
            if brace_depth < 0:
                fail("closing TeX brace without opening brace")
    if brace_depth:
        fail(f"unbalanced TeX braces: depth {brace_depth}")

    required_claims = [
        "Architecting the Internet of AI Agents",
        "33 request directories",
        "27 complete",
        "six contain source only",
        "1,188",
        "7,155",
        "0 of 33",
        "27/27",
        "Source integrity is outside EvidenceBundle verification",
        r"\code{bytecode_path}",
        r"\code{trace_path}",
        r"\code{security_report_path}",
    ]
    for claim in required_claims:
        if claim not in text:
            fail(f"required traceable claim absent: {claim}")

    forbidden = [
        "guarantees security",
        "blockchain immutable",
        "post-quantum secure",
        "certified compliant",
        "prevents all prompt injection",
        "production-grade isolation is implemented",
    ]
    lowered = text.lower()
    for phrase in forbidden:
        if phrase in lowered:
            fail(f"forbidden claim phrase found: {phrase}")

    unsupported_bundle_claims = [
        "links source, bytecode",
        "source digest verification",
        "source, bytecode, trace, report, and ledger",
        "verifies source, bytecode",
    ]
    for phrase in unsupported_bundle_claims:
        if phrase in lowered:
            fail(f"unsupported EvidenceBundle source claim found: {phrase}")

    print(
        f"manuscript check passed: {len(tex_paths)} TeX files, "
        f"{len(figures)} figures, {len(tables)} tables, {len(cited)} citation keys; "
        "environments and braces balanced; all figure/table labels referenced in prose"
    )


if __name__ == "__main__":
    main()
