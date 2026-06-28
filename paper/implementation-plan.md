# ArgorixLang arXiv Preprint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce a reproducible, approximately 20-page academic-English arXiv preprint grounded in ArgorixLang source code and the 33 existing chatbot-runtime session directories.

**Architecture:** A deterministic Python analysis pipeline will read immutable runtime inputs and emit normalized JSON/CSV summaries. A separate figure generator will consume only those normalized outputs and create vector PDF figures. Modular LaTeX sources will consume generated tables, figures, and a primary-source bibliography; a reproducible build and rendered-page inspection will form the final acceptance gate.

**Tech Stack:** Python 3, standard library, pytest, matplotlib, LaTeX (`latexmk` or Tectonic), BibTeX/Biber-compatible `.bib`, Rust/Cargo verification commands.

---

## File structure

- `paper/main.tex` — document class, packages, authors, section assembly.
- `paper/sections/*.tex` — one focused paper section per file.
- `paper/references.bib` — Zotero-importable primary-source bibliography.
- `paper/scripts/analyze_runtime.py` — immutable artifact ingestion and normalized metrics.
- `paper/scripts/generate_figures.py` — vector figure generation from normalized metrics.
- `paper/scripts/render_tables.py` — deterministic LaTeX tables from normalized metrics.
- `paper/tests/test_analyze_runtime.py` — parser and aggregation regression tests.
- `paper/tests/test_generate_figures.py` — expected figure inventory and non-empty output tests.
- `paper/data/runtime_summary.json` — normalized aggregate results.
- `paper/data/sessions.csv` — one row per request directory.
- `paper/data/event_counts.csv` — normalized ledger event counts.
- `paper/tables/*.tex` — generated LaTeX table fragments.
- `paper/figures/*.pdf` — generated vector figures.
- `paper/appendices/artifact-excerpts.tex` — bounded, redacted runtime excerpts.
- `paper/README.md` — reproduction and build instructions.
- `paper/Makefile` — analysis, figure, table, and PDF targets.
- `paper/argorixlang-preprint.pdf` — final compiled manuscript.

### Task 1: Build the runtime-data analyzer with regression tests

**Files:**
- Create: `paper/tests/test_analyze_runtime.py`
- Create: `paper/scripts/analyze_runtime.py`
- Create: `paper/data/.gitkeep`

- [ ] **Step 1: Write fixture-based failing tests**

Create tests that construct temporary complete and incomplete session
directories. Assert that `inventory_sessions()` returns `total_sessions=2`,
`complete_sessions=1`, `incomplete_sessions=1`, and that the complete row
contains all five artifact flags.

```python
from pathlib import Path
import json

from paper.scripts.analyze_runtime import inventory_sessions


def write_json(path: Path, value: dict) -> None:
    path.write_text(json.dumps(value), encoding="utf-8")


def test_inventory_distinguishes_complete_and_incomplete(tmp_path: Path) -> None:
    complete = tmp_path / "complete"
    incomplete = tmp_path / "incomplete"
    complete.mkdir()
    incomplete.mkdir()
    (complete / "session.argx").write_text("module Test {}", encoding="utf-8")
    (incomplete / "session.argx").write_text("module Test {}", encoding="utf-8")
    for name in (
        "session.argbc.json",
        "session.trace.json",
        "session.security.json",
        "session.evidence.json",
    ):
        write_json(complete / name, {})

    result = inventory_sessions(tmp_path)

    assert result["total_sessions"] == 2
    assert result["complete_sessions"] == 1
    assert result["incomplete_sessions"] == 1
    assert result["sessions"][0]["artifact_count"] in {1, 5}
    assert {row["artifact_count"] for row in result["sessions"]} == {1, 5}
```

- [ ] **Step 2: Run the focused test and confirm failure**

Run:

```powershell
python -m pytest paper/tests/test_analyze_runtime.py -q
```

Expected: collection fails because `paper.scripts.analyze_runtime` does not
exist.

- [ ] **Step 3: Implement deterministic inventory and JSON parsing**

Implement constants for the five artifact names, sorted directory traversal,
UTF-8 reads, explicit JSON parse errors, file-size collection, and normalized
extraction for execution status, policy result, review requirement,
`security_checks`, ledger event kinds, passport totals, runtime profiles,
adapters, and evidence digest fields. Do not read files outside the supplied
root and do not follow non-directory entries as sessions.

The CLI must be:

```powershell
python paper/scripts/analyze_runtime.py `
  --input demo/argorix-chatbot-runtime/generated `
  --summary paper/data/runtime_summary.json `
  --sessions paper/data/sessions.csv `
  --events paper/data/event_counts.csv
```

- [ ] **Step 4: Add assertions for contradictory detailed outcomes**

Extend the test fixture so `session.security.json` contains:

```json
{
  "security_checks": "passed",
  "policy": {
    "passed": false,
    "review_required": true,
    "violations": [{"rule": "example", "reason": "unknown policy rule"}]
  }
}
```

Assert that the normalized row reports `security_checks="passed"`,
`policy_passed=false`, `review_required=true`, and `policy_violations=1`.

- [ ] **Step 5: Run tests and real-data analysis**

Run:

```powershell
python -m pytest paper/tests/test_analyze_runtime.py -q
python paper/scripts/analyze_runtime.py --input demo/argorix-chatbot-runtime/generated --summary paper/data/runtime_summary.json --sessions paper/data/sessions.csv --events paper/data/event_counts.csv
```

Expected: tests pass; summary reports 33 sessions, 27 complete, and 6
incomplete.

- [ ] **Step 6: Commit analyzer and normalized outputs**

```powershell
git add paper/scripts/analyze_runtime.py paper/tests/test_analyze_runtime.py paper/data
git commit -m "feat(paper): add reproducible runtime analysis"
```

### Task 2: Verify runtime evidence and capture reproducibility results

**Files:**
- Create: `paper/scripts/verify_runtime.ps1`
- Create: `paper/data/verification-results.json`
- Create: `paper/tests/test_verification_results.py`

- [ ] **Step 1: Write a failing schema test**

Assert that `verification-results.json` has one entry per complete session and
that every entry contains `request_id`, `exit_code`, `verified`, and
`evidence_path`.

- [ ] **Step 2: Run the schema test and confirm failure**

```powershell
python -m pytest paper/tests/test_verification_results.py -q
```

Expected: fail because the verification output does not exist.

- [ ] **Step 3: Implement the verification runner**

The PowerShell script must locate
`C:\Users\nanos\.cargo\bin\cargo.exe`, build `argorix-vm` once, iterate only
directories containing all five artifacts, and execute:

```powershell
& $cargo run -q -p argorix-vm -- verify-evidence $evidencePath
```

Capture exit code and redacted stdout/stderr without modifying runtime inputs.
Write stable, request-ID-sorted JSON.

- [ ] **Step 4: Run all offline evidence checks**

```powershell
powershell -ExecutionPolicy Bypass -File paper/scripts/verify_runtime.ps1
python -m pytest paper/tests/test_verification_results.py -q
```

Expected: exactly 27 verification records. Preserve failures as results; do not
rewrite or delete failing artifacts.

- [ ] **Step 5: Commit verification tooling and results**

```powershell
git add paper/scripts/verify_runtime.ps1 paper/tests/test_verification_results.py paper/data/verification-results.json
git commit -m "test(paper): verify runtime evidence corpus"
```

### Task 3: Generate vector figures and LaTeX tables from normalized data

**Files:**
- Create: `paper/tests/test_generate_figures.py`
- Create: `paper/scripts/generate_figures.py`
- Create: `paper/scripts/render_tables.py`
- Create: `paper/figures/.gitkeep`
- Create: `paper/tables/.gitkeep`

- [ ] **Step 1: Write failing output-inventory tests**

The figure test must require these non-empty PDF files:

```python
EXPECTED = {
    "architecture.pdf",
    "request-sequence.pdf",
    "decision-state-machine.pdf",
    "session-outcomes.pdf",
    "policy-heatmap.pdf",
    "evidence-chain.pdf",
    "trust-relationships.pdf",
    "threat-mitigation.pdf",
    "evolution-timeline.pdf",
    "sovereign-discovery.pdf",
    "artifact-schema.pdf",
    "claim-boundaries.pdf",
}
```

It must also require generated table fragments for dataset inventory,
constructs, controls, empirical results, threats, related work, and claim
boundaries.

- [ ] **Step 2: Run tests and confirm missing-output failure**

```powershell
python -m pytest paper/tests/test_generate_figures.py -q
```

Expected: fail listing missing figures and tables.

- [ ] **Step 3: Implement data figures**

Use matplotlib with a color-blind-safe palette, embedded fonts, readable
single-column and double-column dimensions, and deterministic metadata.
`session-outcomes.pdf` and `policy-heatmap.pdf` must consume only normalized
data files; no hard-coded counts are permitted.

- [ ] **Step 4: Implement architecture figures**

Generate architecture, sequence, state-machine, evidence-chain, trust graph,
threat matrix, timeline, sovereign-discovery, schema, and claim-boundary
figures as vector drawings. Every future-only component must use dashed lines
and labels such as `PROPOSED / NOT IMPLEMENTED`.

- [ ] **Step 5: Implement generated LaTeX tables**

Escape LaTeX special characters, derive quantitative values from JSON/CSV, and
render stable `booktabs` fragments. The claim-boundary table must contain
separate `Implemented`, `Declarative`, `Proposed`, and `Not claimed` columns.

- [ ] **Step 6: Generate and test outputs**

```powershell
python paper/scripts/generate_figures.py --data paper/data --output paper/figures
python paper/scripts/render_tables.py --data paper/data --output paper/tables
python -m pytest paper/tests/test_generate_figures.py -q
```

Expected: all 12 PDF figures and 7 tables exist and are non-empty.

- [ ] **Step 7: Commit reproducible visuals**

```powershell
git add paper/scripts/generate_figures.py paper/scripts/render_tables.py paper/tests/test_generate_figures.py paper/figures paper/tables
git commit -m "feat(paper): add reproducible figures and tables"
```

### Task 4: Build and validate the primary-source bibliography

**Files:**
- Create: `paper/references.bib`
- Create: `paper/data/reference-audit.csv`

- [ ] **Step 1: Collect primary sources**

Validate current bibliographic metadata for Project NANDA, *Architecting the
Internet of AI Agents*, W3C DID Core, W3C Verifiable Credentials, IETF DNS
standards used in the discussion, NIST AI RMF, supply-chain/evidence standards
actually cited, Rust, and relevant agent-protocol literature. Use official
project pages, standards bodies, publisher pages, DOI records, or arXiv.

- [ ] **Step 2: Encode required local intellectual contributions**

Add transparent entries for:

- `ATrust: Agentic Trust Protocol`, attributed to Edison Vazquez;
- `DCP-AI: Digital Citizenship Protocol for Autonomous AI`, attributed to
  Danilo Naranjo.

If no public publication metadata exists, encode them as `@misc` with only
verified title, author, organization/project URL, year when verified, and an
access date. Do not invent DOI, venue, volume, or pages.

- [ ] **Step 3: Audit every bibliography record**

Create `reference-audit.csv` with columns:

```text
citation_key,title,source_type,primary_url,metadata_verified,notes
```

Every cited record must have `metadata_verified=true`; uncited contextual
records may remain false but must not appear in the manuscript.

- [ ] **Step 4: Run bibliography syntax checks**

Use the available BibTeX parser or LaTeX build. Confirm unique citation keys,
balanced braces, required author/title/year-or-access-date fields, and no
placeholder values.

- [ ] **Step 5: Commit bibliography and audit**

```powershell
git add paper/references.bib paper/data/reference-audit.csv
git commit -m "docs(paper): add audited primary-source bibliography"
```

### Task 5: Write the modular academic-English manuscript

**Files:**
- Create: `paper/main.tex`
- Create: `paper/sections/abstract.tex`
- Create: `paper/sections/introduction.tex`
- Create: `paper/sections/background.tex`
- Create: `paper/sections/language-architecture.tex`
- Create: `paper/sections/sovereign-identity.tex`
- Create: `paper/sections/atrust-dcp.tex`
- Create: `paper/sections/runtime.tex`
- Create: `paper/sections/evidence.tex`
- Create: `paper/sections/methodology.tex`
- Create: `paper/sections/results.tex`
- Create: `paper/sections/security-analysis.tex`
- Create: `paper/sections/discussion.tex`
- Create: `paper/sections/limitations.tex`
- Create: `paper/sections/future-work.tex`
- Create: `paper/sections/conclusion.tex`
- Create: `paper/appendices/artifact-excerpts.tex`

- [ ] **Step 1: Create the arXiv-compatible LaTeX shell**

Use a stable article class and common TeX Live packages. Add the four approved
authors and affiliations exactly. Configure hyperlinks, `booktabs`,
`microtype`, accessible figure descriptions where supported, bibliography, and
modular `\input{}` statements.

- [ ] **Step 2: Write abstract and introduction**

State the problem, method, 33-session dataset, 27/6 completeness result,
contributions, and explicit limitation that runtime evidence is not a security
certification. Define the five research questions from `design-spec.md`.

- [ ] **Step 3: Write background and architecture sections**

Ground Project NANDA, ATrust, DCP-AI, Passport, ANS, compiler stages, runtime
profiles, governance, and evidence contracts in sources and repository
artifacts. Separate implemented, declarative, and proposed components in every
section.

- [ ] **Step 4: Write methodology and results**

Use generated values only. Report all evidence-verification outcomes, artifact
completeness, policy outcomes, event distributions, and representative
authorized prompts. Highlight contradictions rather than normalizing them
away.

- [ ] **Step 5: Write security, discussion, and limitations**

Describe threat boundaries and observed behavior without claiming universal
prompt-injection prevention, real authentication, operational DNS, credential
verification, blockchain immutability, post-quantum security, or compliance
certification.

- [ ] **Step 6: Write future work and conclusion**

Define a staged Sovereign DNS/ANS/DID research path, external verification,
larger controlled evaluations, and production isolation work. Keep future work
clearly outside current empirical results.

- [ ] **Step 7: Add bounded appendices**

Include redacted artifact excerpts, schema notes, and reproducibility commands.
Do not reproduce full user conversations, API keys, bearer tokens, or secret
values.

- [ ] **Step 8: Run manuscript consistency checks**

Search for uncited quantitative claims, undefined acronyms, missing figure
references, unsupported superlatives, `TBD`, `TODO`, `placeholder`, and claim
boundary violations.

- [ ] **Step 9: Commit manuscript**

```powershell
git add paper/main.tex paper/sections paper/appendices
git commit -m "docs(paper): write ArgorixLang arxiv preprint"
```

### Task 6: Add reproducible build automation and compile the PDF

**Files:**
- Create: `paper/Makefile`
- Create: `paper/README.md`
- Create: `paper/latexmkrc`
- Create: `paper/argorixlang-preprint.pdf`

- [ ] **Step 1: Add deterministic build targets**

The Makefile must expose:

```text
analyze
verify
figures
tables
paper
test
clean
all
```

`all` must run analysis, verification, figures, tables, tests, and PDF build in
that order. `clean` may remove only generated LaTeX auxiliary files and must
not delete runtime inputs or normalized source data.

- [ ] **Step 2: Document Windows and portable commands**

The README must list exact prerequisites, the immutable input path, the Python
commands, evidence verification command, LaTeX build command, output inventory,
and the distinction between regenerating results and compiling only the paper.

- [ ] **Step 3: Compile with a reproducible LaTeX engine**

Prefer an existing `latexmk`; otherwise use a pinned portable Tectonic release
recorded in the README. Run compilation until cross-references and bibliography
stabilize.

- [ ] **Step 4: Enforce build diagnostics**

Fail on undefined citations, undefined references, missing figures, fatal
overfull boxes, or bibliography errors. Record acceptable non-fatal warnings
in the README only when they cannot affect meaning or layout.

- [ ] **Step 5: Commit build tooling and PDF**

```powershell
git add paper/Makefile paper/README.md paper/latexmkrc paper/argorixlang-preprint.pdf
git commit -m "build(paper): add reproducible preprint build"
```

### Task 7: Render, visually inspect, and perform final scientific QA

**Files:**
- Modify: `paper/main.tex`
- Modify: `paper/sections/*.tex`
- Modify: `paper/figures/*.pdf`
- Modify: `paper/argorixlang-preprint.pdf`
- Create: `paper/data/final-qa.json`

- [ ] **Step 1: Render every PDF page to images**

Use Poppler or the bundled PDF tooling to render all pages at sufficient
resolution for layout inspection.

- [ ] **Step 2: Inspect all pages**

Check title/author layout, page count, clipped text, table overflow, figure
legibility, caption placement, widows/orphans, blank pages, bibliography
formatting, and appendix boundaries.

- [ ] **Step 3: Cross-check scientific claims**

For every numerical result in the abstract, results, discussion, and
conclusion, locate the corresponding normalized data field. For every
implemented feature claim, locate the relevant repository or runtime artifact.
For every related-work statement, locate its bibliography source.

- [ ] **Step 4: Run the complete acceptance suite**

```powershell
python -m pytest paper/tests -q
powershell -ExecutionPolicy Bypass -File paper/scripts/verify_runtime.ps1
python paper/scripts/analyze_runtime.py --input demo/argorix-chatbot-runtime/generated --summary paper/data/runtime_summary.json --sessions paper/data/sessions.csv --events paper/data/event_counts.csv
python paper/scripts/generate_figures.py --data paper/data --output paper/figures
python paper/scripts/render_tables.py --data paper/data --output paper/tables
```

Then rebuild the PDF. Expected: tests pass, 33/27/6 inventory remains stable,
all complete evidence bundles have explicit verification outcomes, no
undefined LaTeX references/citations remain, and the rendered paper is
approximately 20 pages.

- [ ] **Step 5: Write the final QA record**

`final-qa.json` must include the commit, dataset counts, test command outcomes,
evidence-verification totals, LaTeX engine/version, page count, figure count,
table count, citation count, inspection timestamp, and any retained warning.

- [ ] **Step 6: Commit final corrections and QA**

```powershell
git add paper
git commit -m "docs(paper): finalize verified ArgorixLang study"
```
