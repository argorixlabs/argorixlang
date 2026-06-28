import csv
import json
import unittest
from pathlib import Path, PurePosixPath


PAPER = Path(__file__).parents[1]
RESULTS = PAPER / "data" / "verification-results.json"
SESSIONS = PAPER / "data" / "sessions.csv"
REQUIRED_FIELDS = {
    "request_id",
    "exit_code",
    "verified",
    "evidence_path",
    "stdout",
    "stderr",
}


class VerificationResultsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.records = json.loads(RESULTS.read_text(encoding="utf-8"))
        with SESSIONS.open(encoding="utf-8", newline="") as stream:
            cls.complete_ids = sorted(
                row["request_id"]
                for row in csv.DictReader(stream)
                if row["complete"].lower() == "true"
            )

    def test_has_exactly_one_stable_record_per_complete_session(self):
        request_ids = [record["request_id"] for record in self.records]

        self.assertEqual(request_ids, self.complete_ids)
        self.assertEqual(len(request_ids), len(set(request_ids)))
        self.assertEqual(len(request_ids), 27)
        self.assertTrue(all(set(record) == REQUIRED_FIELDS for record in self.records))

    def test_records_are_sorted_and_evidence_paths_are_safe_and_relative(self):
        request_ids = [record["request_id"] for record in self.records]
        self.assertEqual(request_ids, sorted(request_ids))

        for record in self.records:
            evidence_path = PurePosixPath(record["evidence_path"])
            self.assertFalse(evidence_path.is_absolute())
            self.assertNotIn("..", evidence_path.parts)
            self.assertEqual(
                evidence_path,
                PurePosixPath(record["request_id"]) / "session.evidence.json",
            )

    def test_failures_are_preserved_as_bounded_diagnostics(self):
        for record in self.records:
            self.assertIs(type(record["exit_code"]), int)
            self.assertIs(type(record["verified"]), bool)
            self.assertEqual(record["verified"], record["exit_code"] == 0)
            self.assertIs(type(record["stdout"]), str)
            self.assertIs(type(record["stderr"]), str)
            self.assertLessEqual(len(record["stdout"]), 4096)
            self.assertLessEqual(len(record["stderr"]), 4096)
            if not record["verified"]:
                self.assertNotEqual(record["exit_code"], 0)
                self.assertTrue(record["stdout"] or record["stderr"])


if __name__ == "__main__":
    unittest.main()
