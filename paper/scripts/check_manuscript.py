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
    "ablation-study",
    "baseline-comparison",
    "claim-boundaries",
    "controlled-matrix-summary",
    "controlled-evaluation-matrix",
    "dataset-inventory",
    "empirical-results",
    "language-constructs",
    "limitations-future-assurance",
    "matrix-provider-boundary",
    "matrix-tamper-results",
    "passport-jurisdiction-coverage",
    "policy-lattice",
    "policy-lattice-outcomes",
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
    affiliation_requirements = (
        r"Gustavo Venegas\textsuperscript{1}",
        r"Edison Vazquez\textsuperscript{1}",
        r"Danilo Naranjo\textsuperscript{2}",
        r"Benjamin Gonzalez\textsuperscript{1}",
    )
    for requirement in affiliation_requirements:
        if requirement not in main_text:
            fail(f"missing author affiliation marker: {requirement}")
    if main_text.count("Chilean Chamber of Artificial Intelligence") != 1:
        fail("shared Chamber affiliation must appear exactly once")
    if main_text.count(r"\textsuperscript{2}Ocular") != 1:
        fail("Ocular affiliation must appear exactly once")

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
    table_text = "\n".join(
        (ROOT / "tables" / f"{table}.tex").read_text(encoding="utf-8")
        for table in sorted(tables)
    )
    if "local catalog" in table_text.lower():
        fail("stale local catalog implementation claim found")
    if r"local ans\_name binding/metadata" not in table_text:
        fail("claim-boundary table lacks local ans_name binding/metadata")

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
        "0 of 27",
        "six source-only directories",
        "27/27",
        "Source integrity is outside EvidenceBundle verification",
        "controlled artifact matrix",
        "typed policy decision lattice",
        "Generated in controlled matrix",
        "Do not claim until measured",
        "31-country matrix evaluates syntactic and semantic handling of jurisdictional metadata",
        "Country and residency fields are evaluated as local metadata",
        "They do not establish legal compliance or physical storage location",
        r"\code{bytecode_path}",
        r"\code{trace_path}",
        r"\code{security_report_path}",
        r"computes \code{ledger_digest} from \code{trace.events}",
        "the report stores this summary digest, not ledger events",
    ]
    normalized_text = re.sub(r"\s+", " ", text)
    for claim in required_claims:
        if claim not in normalized_text:
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

    for stale_prompt_denominator in ("0/33", "0 of 33"):
        if stale_prompt_denominator in lowered:
            fail(f"stale prompt denominator found: {stale_prompt_denominator}")

    unsupported_bundle_claims = [
        "links source, bytecode",
        "source digest verification",
        "source, bytecode, trace, report, and ledger",
        "verifies source, bytecode",
        "embedded ledger",
        "embedded-ledger",
        "report's embedded ledger",
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
