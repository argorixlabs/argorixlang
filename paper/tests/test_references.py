import csv
import re
from pathlib import Path


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
        if kind not in {"misc", "software"}:
            assert re.search(r"(?m)^\s*year\s*=", body), key


def test_audit_matches_bibliography():
    bib_keys = {key for _, key, _ in entries()}
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
