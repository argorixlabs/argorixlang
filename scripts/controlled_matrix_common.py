from __future__ import annotations

import csv
import hashlib
import json
import tempfile
from pathlib import Path
from typing import Iterable


ROOT = Path(__file__).resolve().parents[1]
EXPERIMENT = ROOT / "experiments" / "controlled-matrix"
RESULTS = ROOT / "results"

COUNTRIES = [
    ("CL", "Chile", "home"),
    ("AR", "Argentina", "latin_america"),
    ("BR", "Brazil", "latin_america"),
    ("UY", "Uruguay", "latin_america"),
    ("PE", "Peru", "latin_america"),
    ("CO", "Colombia", "latin_america"),
    ("MX", "Mexico", "latin_america"),
    ("US", "United States", "north_america"),
    ("CA", "Canada", "north_america"),
    ("ES", "Spain", "europe"),
    ("FR", "France", "europe"),
    ("DE", "Germany", "europe"),
    ("IT", "Italy", "europe"),
    ("NL", "Netherlands", "europe"),
    ("SE", "Sweden", "europe"),
    ("NO", "Norway", "europe"),
    ("GB", "United Kingdom", "europe"),
    ("PT", "Portugal", "europe"),
    ("IE", "Ireland", "europe"),
    ("JP", "Japan", "asia_pacific"),
    ("KR", "South Korea", "asia_pacific"),
    ("SG", "Singapore", "asia_pacific"),
    ("IN", "India", "asia_pacific"),
    ("CN", "China", "asia_pacific"),
    ("AU", "Australia", "asia_pacific"),
    ("NZ", "New Zealand", "asia_pacific"),
    ("ZA", "South Africa", "africa_middle_east"),
    ("NG", "Nigeria", "africa_middle_east"),
    ("KE", "Kenya", "africa_middle_east"),
    ("AE", "United Arab Emirates", "africa_middle_east"),
    ("IL", "Israel", "africa_middle_east"),
]
RECOGNIZED = {code for code, _, _ in COUNTRIES}
EU_COUNTRIES = {"ES", "FR", "DE", "IT", "NL", "SE", "PT", "IE"}
SENSITIVE_CLASSES = {"sensitive", "personal", "regulated", "confidential"}
CSV_COLUMNS = [
    "Case", "Country", "Residency", "Data Class", "Expected", "Observed",
    "Policy", "Provider", "Evidence", "Source", "Trace Events",
    "Side Effect", "Outcome",
]


def atomic_write(path: Path, data: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", newline="", dir=path.parent, delete=False) as handle:
        handle.write(data)
        tmp = Path(handle.name)
    tmp.replace(path)


def sha256_text(value: str) -> str:
    return "sha256:" + hashlib.sha256(value.encode("utf-8")).hexdigest()


def canonical_json(value: object) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def write_json(path: Path, value: object) -> None:
    atomic_write(path, json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n")


def write_csv(path: Path, rows: Iterable[dict], columns: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", newline="", dir=path.parent, delete=False) as handle:
        writer = csv.DictWriter(handle, fieldnames=columns)
        writer.writeheader()
        for row in rows:
            writer.writerow({column: row.get(column, "") for column in columns})
        tmp = Path(handle.name)
    tmp.replace(path)


def load_fixtures() -> list[dict]:
    fixtures: list[dict] = []
    for path in sorted(EXPERIMENT.glob("*/*.json")):
        fixtures.append(json.loads(path.read_text(encoding="utf-8")))
    return fixtures


def is_alpha2(value: object) -> bool:
    return isinstance(value, str) and len(value) == 2 and value.isalpha() and value.upper() == value


def source_for(case: dict) -> str:
    return (
        f"case={case['case']}\n"
        f"country={case.get('country')!r}\n"
        f"residency={case.get('residency')!r}\n"
        f"data_class={case.get('data_class')!r}\n"
        f"profile={case.get('profile', 'default')!r}\n"
    )
