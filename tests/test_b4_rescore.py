"""Historical B4 correction must not rewrite the camera-ready result."""

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
EVAL = ROOT / "evaluation" / "adversarial"
HISTORICAL = EVAL / "results" / "summary.json"


class B4RescoreTest(unittest.TestCase):
    def test_e5_scope_is_corrected_without_changing_historical_result(self):
        before = hashlib.sha256(HISTORICAL.read_bytes()).hexdigest()
        original = json.loads(HISTORICAL.read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory(prefix="argorix-b4-") as temp:
            output = Path(temp)
            completed = subprocess.run(
                [
                    sys.executable,
                    str(EVAL / "harness" / "score.py"),
                    "--run-id", "e5-live-a",
                    "--raw-dir", str(EVAL / "results" / "raw" / "e5-live-a"),
                    "--results-dir", str(output),
                    "--oracle", str(EVAL / "oracle.json"),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            corrected = json.loads((output / "summary.json").read_text(encoding="utf-8"))
        after = hashlib.sha256(HISTORICAL.read_bytes()).hexdigest()
        self.assertEqual(before, after)
        self.assertEqual(corrected["rows_total"], original["rows_total"])
        self.assertEqual(corrected["E5"]["runs"], original["E5"]["runs"])
        self.assertEqual(
            corrected["primary_metrics"]["outcome_accuracy"]["text"],
            original["primary_metrics"]["outcome_accuracy"]["text"],
        )
        boundary = next(item for item in corrected["findings"]["boundaries"] if item["id"] == "B4")
        self.assertIn("E5 evaluated prompt injection", boundary["claim_effect"])
        self.assertIn("not evaluate resistance of a native Argorix agent loop", boundary["claim_effect"])


if __name__ == "__main__":
    unittest.main()
