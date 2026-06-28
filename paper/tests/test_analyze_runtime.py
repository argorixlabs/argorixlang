import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts" / "analyze_runtime.py"
SPEC = importlib.util.spec_from_file_location("analyze_runtime", SCRIPT)
analyze_runtime = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(analyze_runtime)

ARTIFACTS = (
    "session.argx",
    "session.argbc.json",
    "session.trace.json",
    "session.security.json",
    "session.evidence.json",
)


class RuntimeAnalysisTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)

    def tearDown(self):
        self.temp.cleanup()

    def write_complete(self, request_id="request-b"):
        session = self.root / request_id
        session.mkdir()
        (session / "session.argx").write_text(
            'module Fixture\n// prompt: "source prompt"\nsecret API_TOKEN env "never-export-me"\n',
            encoding="utf-8",
        )
        (session / "session.argbc.json").write_text(
            json.dumps(
                {
                    "runtime_execution_profiles": [{"name": "restricted-runtime"}],
                    "sandboxed_provider_adapters": [{"name": "sandbox-provider"}],
                }
            ),
            encoding="utf-8",
        )
        (session / "session.trace.json").write_text(
            json.dumps(
                {
                    "status": "completed",
                    "prompt": "trace prompt",
                    "security_checks": "passed",
                    "events": [
                        {"event_type": "PolicyEvaluated"},
                        {"event_type": "PolicyEvaluated"},
                        {"event_type": "ProviderBlocked"},
                    ],
                }
            ),
            encoding="utf-8",
        )
        (session / "session.security.json").write_text(
            json.dumps(
                {
                    "execution": {"status": "completed"},
                    "agent_passports": {
                        "total": 2,
                        "countries": ["CL"],
                        "jurisdictions": ["CL"],
                        "data_residency": ["CL", "EU"],
                    },
                    "runtime_execution_profiles": {
                        "total": 1,
                        "names": ["restricted-runtime"],
                    },
                    "sandboxed_provider_adapters": {
                        "total": 1,
                        "names": ["sandbox-provider"],
                    },
                    "policy": {
                        "passed": False,
                        "review_required": True,
                        "violations": [
                            {"rule": "future_rule", "reason": "unknown policy rule"}
                        ],
                    },
                    "provider_boundary": {
                        "executable_providers": ["simulated"],
                        "external_execution_blocked": True,
                    },
                    "ledger": {
                        "events_total": 3,
                        "event_kinds": {
                            "PolicyEvaluated": 2,
                            "ProviderBlocked": 1,
                        },
                    },
                }
            ),
            encoding="utf-8",
        )
        (session / "session.evidence.json").write_text(
            json.dumps(
                {
                    "bytecode_digest": "sha256:" + "a" * 64,
                    "trace_digest": "invalid",
                    "report_digest": "sha256:" + "b" * 64,
                    "ledger_digest": "sha256:" + "c" * 64,
                }
            ),
            encoding="utf-8",
        )
        return session

    def test_inventory_preserves_independent_security_and_policy_facts(self):
        self.write_complete()

        result = analyze_runtime.inventory_sessions(self.root)
        row = result["sessions"][0]

        self.assertEqual(result["total_sessions"], 1)
        self.assertEqual(result["complete_sessions"], 1)
        self.assertEqual(result["incomplete_sessions"], 0)
        self.assertEqual(row["security_checks"], "passed")
        self.assertIs(row["policy_passed"], False)
        self.assertIs(row["review_required"], True)
        self.assertEqual(row["policy_violation_count"], 1)
        self.assertEqual(row["policy_violation_reasons"], ["unknown policy rule"])

    def test_inventory_is_sorted_ignores_files_and_records_incomplete_artifacts(self):
        self.write_complete("request-z")
        incomplete = self.root / "request-a"
        incomplete.mkdir()
        (incomplete / "session.argx").write_text("module Incomplete", encoding="utf-8")
        (self.root / "not-a-session.txt").write_text("ignored", encoding="utf-8")

        result = analyze_runtime.inventory_sessions(self.root)

        self.assertEqual([s["request_id"] for s in result["sessions"]], ["request-a", "request-z"])
        self.assertEqual(result["complete_sessions"], 1)
        self.assertEqual(result["incomplete_sessions"], 1)
        artifact = result["sessions"][0]["artifacts"]["session.argbc.json"]
        self.assertEqual(artifact, {"present": False, "size_bytes": 0})

    def test_normalizes_runtime_ledger_passport_boundary_digest_and_prompt_fields(self):
        self.write_complete()

        row = analyze_runtime.inventory_sessions(self.root)["sessions"][0]

        self.assertEqual(row["execution_status"], "completed")
        self.assertEqual(row["ledger_events_total"], 3)
        self.assertEqual(
            row["ledger_event_kinds"],
            {"PolicyEvaluated": 2, "ProviderBlocked": 1},
        )
        self.assertEqual(row["passport_total"], 2)
        self.assertEqual(row["passport_countries"], ["CL"])
        self.assertEqual(row["passport_jurisdictions"], ["CL"])
        self.assertEqual(row["passport_data_residency"], ["CL", "EU"])
        self.assertEqual(row["runtime_profiles"], ["restricted-runtime"])
        self.assertEqual(row["sandboxed_adapters"], ["sandbox-provider"])
        self.assertEqual(row["provider_boundary"]["executable_providers"], ["simulated"])
        self.assertEqual(row["prompt_text"], "trace prompt")
        self.assertNotIn("never-export-me", json.dumps(row))
        self.assertTrue(row["evidence_digests"]["bytecode_digest"]["valid"])
        self.assertFalse(row["evidence_digests"]["trace_digest"]["valid"])

    def test_json_parse_errors_are_recorded_without_aborting_corpus(self):
        broken = self.write_complete("request-a")
        (broken / "session.trace.json").write_text("{broken", encoding="utf-8")
        self.write_complete("request-b")

        result = analyze_runtime.inventory_sessions(self.root)

        self.assertEqual(result["total_sessions"], 2)
        self.assertIn("session.trace.json", result["sessions"][0]["json_errors"])
        self.assertEqual(result["sessions"][1]["execution_status"], "completed")


if __name__ == "__main__":
    unittest.main()
