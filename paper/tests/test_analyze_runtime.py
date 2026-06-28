import csv
import importlib.util
import json
import os
import subprocess
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
            "module Fixture\n"
            "type UserPrompt { content: string }\n"
            "secret API_TOKEN env \"never-export-me\"\n",
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
                    "injected": {
                        "from": "User",
                        "to": "AssistantAgent",
                        "act": "tell",
                        "message_type": "UserPrompt",
                        "content": "trace prompt",
                        "secret_value": "never-export-me",
                    },
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
                        "declarative_contracts": [
                            {"name": "OpenAIProvider", "api_key": "should-not-export"}
                        ],
                        "external_contracts_total": 1,
                        "external_execution_blocked": True,
                        "blocked_attempts": 1,
                        "secret_value": "should-not-export",
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
        self.assertEqual(row["artifact_count"], 5)
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
        self.assertEqual(result["sessions"][0]["artifact_count"], 1)
        self.assertEqual(result["sessions"][1]["artifact_count"], 5)
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
        self.assertEqual(
            row["provider_boundary"],
            {
                "executable_providers": ["simulated"],
                "declarative_contract_names": ["OpenAIProvider"],
                "external_contracts_total": 1,
                "external_execution_blocked": True,
                "blocked_attempts": 1,
            },
        )
        self.assertEqual(row["prompt_text"], "trace prompt")
        self.assertNotIn("never-export-me", json.dumps(row))
        self.assertNotIn("should-not-export", json.dumps(row))
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

    def test_actual_redacted_injected_schema_does_not_invent_prompt_text(self):
        session = self.write_complete()
        trace_path = session / "session.trace.json"
        trace = json.loads(trace_path.read_text(encoding="utf-8"))
        trace["injected"] = {
            "from": "User",
            "to": "AssistantAgent",
            "act": "tell",
            "message_type": "UserPrompt",
        }
        trace_path.write_text(json.dumps(trace), encoding="utf-8")

        row = analyze_runtime.inventory_sessions(self.root)["sessions"][0]

        self.assertIsNone(row["prompt_text"])

    def test_prompt_and_policy_text_redact_secret_patterns_and_env_values(self):
        session = self.write_complete()
        trace_path = session / "session.trace.json"
        trace = json.loads(trace_path.read_text(encoding="utf-8"))
        trace["injected"]["content"] = (
            "harmless context Bearer abc.def password=hunter2 "
            "token: tok_live_123 key sk-proj-123456789 env-secret-value"
        )
        trace_path.write_text(json.dumps(trace), encoding="utf-8")
        security_path = session / "session.security.json"
        security = json.loads(security_path.read_text(encoding="utf-8"))
        security["policy"]["violations"][0] = {
            "rule": "token=rule-secret",
            "reason": "API_KEY=reason-secret",
        }
        security_path.write_text(json.dumps(security), encoding="utf-8")

        previous = os.environ.get("PAPER_TEST_API_KEY")
        os.environ["PAPER_TEST_API_KEY"] = "env-secret-value"
        try:
            row = analyze_runtime.inventory_sessions(self.root)["sessions"][0]
        finally:
            if previous is None:
                os.environ.pop("PAPER_TEST_API_KEY", None)
            else:
                os.environ["PAPER_TEST_API_KEY"] = previous

        serialized = json.dumps(row)
        self.assertIn("harmless context", row["prompt_text"])
        for secret in ("abc.def", "hunter2", "tok_live_123", "env-secret-value",
                       "sk-proj-123456789", "rule-secret", "reason-secret"):
            self.assertNotIn(secret, serialized)
        self.assertIn("[REDACTED]", serialized)

    def test_external_linked_session_and_artifact_are_not_read(self):
        external = self.root.parent / (self.root.name + "-external")
        external.mkdir()
        try:
            secret_session = external / "external-session"
            secret_session.mkdir()
            (secret_session / "session.argx").write_text(
                "external secret", encoding="utf-8"
            )
            linked_session = self.root / "linked-session"
            try:
                linked_session.symlink_to(secret_session, target_is_directory=True)
            except OSError as exc:
                self.skipTest(f"directory symlinks unavailable: {exc}")

            local = self.root / "local"
            local.mkdir()
            external_artifact = external / "session.argx"
            external_artifact.write_text("external artifact secret", encoding="utf-8")
            (local / "session.argx").symlink_to(external_artifact)

            result = analyze_runtime.inventory_sessions(self.root)

            self.assertEqual([row["request_id"] for row in result["sessions"]], ["local"])
            self.assertEqual(result["sessions"][0]["artifact_count"], 0)
            serialized = json.dumps(result)
            self.assertNotIn("external secret", serialized)
            self.assertNotIn("external artifact secret", serialized)
        finally:
            for child in external.iterdir():
                if child.is_dir():
                    for nested in child.iterdir():
                        nested.unlink()
                    child.rmdir()
                else:
                    child.unlink()
            external.rmdir()

    @unittest.skipUnless(os.name == "nt", "junction behavior is Windows-specific")
    def test_external_junction_session_is_skipped(self):
        external = self.root.parent / (self.root.name + "-junction-target")
        external.mkdir()
        (external / "session.argx").write_text("junction secret", encoding="utf-8")
        junction = self.root / "junction-session"
        result = subprocess.run(
            ["cmd", "/c", "mklink", "/J", str(junction), str(external)],
            capture_output=True,
            text=True,
        )
        if result.returncode:
            self.skipTest(f"junction creation unavailable: {result.stderr}")
        try:
            inventory = analyze_runtime.inventory_sessions(self.root)
            self.assertEqual(inventory["total_sessions"], 0)
        finally:
            os.rmdir(junction)
            (external / "session.argx").unlink()
            external.rmdir()

    def test_csv_serialization_neutralizes_formula_prefixes(self):
        session = self.write_complete(" =danger")
        security_path = session / "session.security.json"
        security = json.loads(security_path.read_text(encoding="utf-8"))
        security["execution"]["status"] = "@malicious"
        security_path.write_text(json.dumps(security), encoding="utf-8")
        rows = analyze_runtime.inventory_sessions(self.root)["sessions"]
        output = self.root.parent / (self.root.name + "-sessions.csv")
        try:
            analyze_runtime._write_sessions(output, rows)
            with output.open(encoding="utf-8", newline="") as handle:
                row = next(csv.DictReader(handle))
            self.assertEqual(row["request_id"], "' =danger")
            self.assertEqual(row["execution_status"], "'@malicious")
            self.assertEqual(rows[0]["request_id"], " =danger")
        finally:
            output.unlink(missing_ok=True)

    def test_output_paths_cannot_collide_or_enter_input_root(self):
        output = self.root.parent / (self.root.name + "-summary.json")
        with self.assertRaises(ValueError):
            analyze_runtime.validate_output_paths(
                self.root, output, output, output.with_suffix(".csv")
            )
        with self.assertRaises(ValueError):
            analyze_runtime.validate_output_paths(
                self.root,
                self.root / "summary.json",
                output.with_name("sessions.csv"),
                output.with_name("events.csv"),
            )


if __name__ == "__main__":
    unittest.main()
