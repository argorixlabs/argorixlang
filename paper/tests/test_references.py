import csv
import re
from pathlib import Path

import bibtexparser


ROOT = Path(__file__).parents[1]
BIB = ROOT / "references.bib"
AUDIT = ROOT / "data" / "reference-audit.csv"


def entries():
    text = BIB.read_text(encoding="utf-8")
    matches = list(re.finditer(r"(?m)^@(\w+)\{([^,]+),", text))
    assert text.count("{") == text.count("}")
    return [(match.group(1).lower(), match.group(2), text[match.start():matches[index + 1].start() if index + 1 < len(matches) else len(text)])
            for index, match in enumerate(matches)]


def test_unique_keys_and_required_fields():
    parsed = entries()
    keys = [key for _, key, _ in parsed]
    assert len(keys) == len(set(keys))
    for kind, key, body in parsed:
        assert re.search(r"(?m)^\s*title\s*=", body), key
        assert re.search(r"(?m)^\s*(author|editor)\s*=", body), key
        if key == "vazquez_atrust":
            assert not re.search(r"(?m)^\s*(year|urldate|url|doi)\s*=", body)
        else:
            assert re.search(r"(?m)^\s*(year|urldate)\s*=", body), key


def normalized_title(value):
    return value.replace("{", "").replace("}", "")


def test_audit_matches_bibliography():
    parsed = bibtexparser.load(BIB.open(encoding="utf-8"))
    bib_entries = {entry["ID"]: entry for entry in parsed.entries}
    bib_keys = set(bib_entries)
    with AUDIT.open(encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle)
        assert reader.fieldnames == [
            "citation_key", "title", "source_type", "primary_url",
            "metadata_verified", "notes",
        ]
        rows = list(reader)
    audit_keys = {row["citation_key"] for row in rows}
    assert audit_keys == bib_keys
    assert len(rows) == len(audit_keys)
    assert all(row["metadata_verified"] == "true" for row in rows)
    for row in rows:
        entry = bib_entries[row["citation_key"]]
        assert row["title"] == normalized_title(entry["title"])
        if row["primary_url"]:
            doi_url = f"https://doi.org/{entry['doi']}" if "doi" in entry else None
            assert row["primary_url"] in {entry.get("url"), doi_url}
    assert bib_entries["naranjo_dcpai"]["doi"] == "10.31224/6671"


def test_required_source_truths():
    parsed = bibtexparser.load(BIB.open(encoding="utf-8"))
    records = {entry["ID"]: entry for entry in parsed.entries}
    with AUDIT.open(encoding="utf-8", newline="") as handle:
        audits = {row["citation_key"]: row for row in csv.DictReader(handle)}

    assert normalized_title(records["projectnanda2026"]["title"]) == (
        "Architecting the Internet of AI Agents"
    )
    assert records["projectnanda2026"]["urldate"] == "2026-06-28"
    assert audits["projectnanda2026"]["primary_url"] == "https://projectnanda.org/"
    for key in ("w3c_did_core_2022", "w3c_vc_data_model_2025"):
        assert records[key]["author"] == "{World Wide Web Consortium}"
        assert "editor" not in records[key]
    assert records["naranjo_dcpai"]["author"] == "Naranjo Emparanza, Danilo"

    a2a = records["a2a_spec"]
    assert normalized_title(a2a["title"]) == "Agent2Agent (A2A) Protocol Specification"
    assert a2a["version"] == "1.0.0"
    assert a2a["url"] == "https://a2a-protocol.org/v1.0.0/specification/"
    assert records["rust_project"]["title"] == "Rust Programming Language"

    atrust = records["vazquez_atrust"]
    assert atrust["note"] == (
        "Author-supplied metadata used in this study; unpublished conceptual protocol"
    )
    assert not {"year", "urldate", "url", "doi"} & atrust.keys()
    assert audits["vazquez_atrust"]["source_type"] == "author-supplied metadata"

    assert records["opa_docs"]["url"] == "https://www.openpolicyagent.org/docs"
    assert records["opa_docs"]["urldate"] == "2026-06-28"
    assert records["bordini2007jason"]["doi"] == "10.1002/9780470061848"
    assert records["bordini2007jason"]["year"] == "2007"
    assert records["torresarias2019intoto"]["booktitle"] == (
        "28th USENIX Security Symposium (USENIX Security 19)"
    )
    assert records["torresarias2019intoto"]["pages"] == "1393--1410"
    assert records["wasi_intro"]["url"] == "https://wasi.dev/"
    assert records["wasi_intro"]["urldate"] == "2026-06-28"
    assert records["wasi_intro"]["author"] == "{WASI Subgroup}"
    assert records["wasi_intro"]["title"] == "Introduction"
