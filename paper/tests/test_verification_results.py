import csv
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path, PurePosixPath


PAPER = Path(__file__).parents[1]
RESULTS = PAPER / "data" / "verification-results.json"
SESSIONS = PAPER / "data" / "sessions.csv"
SUMMARY = PAPER / "data" / "runtime_summary.json"
SCRIPT = PAPER / "scripts" / "verify_runtime.ps1"
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

    def test_cargo_resolution_is_explicit_then_path_then_legacy_fallback(self):
        source = SCRIPT.read_text(encoding="utf-8")

        self.assertIn("[string]$CargoPath", source)
        explicit = source.index("if ($CargoPath)")
        path_lookup = source.index("Get-Command cargo")
        legacy = source.index(r'C:\Users\nanos\.cargo\bin\cargo.exe')
        self.assertLess(explicit, path_lookup)
        self.assertLess(path_lookup, legacy)
        self.assertIn("Cargo executable was not found", source)

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

    def test_prompt_presence_denominator_uses_only_valid_traces(self):
        summary = json.loads(SUMMARY.read_text(encoding="utf-8"))

        self.assertEqual(summary["traces_inspected_for_prompt_content"], 27)
        self.assertEqual(summary["traces_with_prompt_content"], 0)
        complete = [row for row in summary["sessions"] if row["complete"]]
        source_only = [row for row in summary["sessions"] if not row["complete"]]
        self.assertEqual({row["prompt_content_present"] for row in complete}, {False})
        self.assertEqual({row["prompt_content_present"] for row in source_only}, {None})

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

    def test_sanitizer_redacts_credentials_without_ambient_environment(self):
        secrets = [
            "bearer-credential-123456789",
            "json-token-value-123456789",
            "json-api-key-value-123456789",
            "json-password-value-123456789",
            "json-secret-value-123456789",
            "assignment-token-value-123456789",
            "sk-proj-AbCdEfGhIjKlMnOpQrStUvWxYz123456",
            "ghp_AbCdEfGhIjKlMnOpQrStUvWxYz1234567890",
            "AKIAABCDEFGHIJKLMNOP",
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.signature123",
            "bare-password-value-123456789",
            "bare-token-value-123456789",
            "bare-secret-value-123456789",
            "bare-api-key-value-123456789",
            "quoted-password-value-123456789",
            "unquoted-token-value-123456789",
            "unquoted-secret-value-123456789",
            "quoted-api-key-value-123456789",
        ]
        diagnostic = "\n".join(
            [
                f"Authorization: Bearer {secrets[0]}",
                json.dumps({"token": secrets[1]}),
                json.dumps({"api_key": secrets[2]}),
                json.dumps({"password": secrets[3]}),
                json.dumps({"secret": secrets[4]}),
                f"ACCESS_TOKEN={secrets[5]}",
                *secrets[6:10],
                f"Password={secrets[10]}",
                f"TOKEN: '{secrets[11]}'",
                f'SeCrEt="{secrets[12]}"',
                f"aPi_KeY = {secrets[13]}",
                f"PASSWORD: '{secrets[14]}'",
                f"token={secrets[15]}",
                f"SECRET: {secrets[16]}",
                f'API_KEY="{secrets[17]}"',
                "ordinary diagnostic remains",
            ]
        )

        sanitized = sanitize_with_powershell(diagnostic)

        for secret in secrets:
            self.assertNotIn(secret, sanitized)
        self.assertIn("ordinary diagnostic remains", sanitized)
        self.assertGreaterEqual(sanitized.count("<redacted>"), len(secrets))
        self.assertEqual(
            sanitized,
            sanitize_with_powershell(diagnostic, sentinel="different-environment"),
        )

    def test_sanitizer_consumes_full_authorization_values_and_sensitive_variants(self):
        secrets = [
            "basic-credential-value-123456789",
            "bearer-credential-value-123456789",
            "access-token-value-123456789",
            "client-secret-value-123456789",
            "api-key-value-123456789",
            "refresh-token-value-123456789",
            "auth-token-value-123456789",
            "password-value-123456789",
            "secret-value-123456789",
            "credential-value-123456789",
            "private-key-value-123456789",
        ]
        diagnostic = "\n".join(
            [
                f'Authorization: "Basic {secrets[0]}"',
                f"Authorization: Bearer {secrets[1]}",
                json.dumps({"accessToken": secrets[2]}),
                f"clientSecret='{secrets[3]}'",
                f"apiKey={secrets[4]}",
                f"refresh-token: {secrets[5]}",
                f"AUTH_TOKEN={secrets[6]}",
                f"password={secrets[7]}",
                f"secret={secrets[8]}",
                f"credential={secrets[9]}",
                f'privateKey="{secrets[10]}"',
                "The password policy and private key documentation remain ordinary prose.",
            ]
        )

        sanitized = sanitize_with_powershell(diagnostic)

        for secret in secrets:
            self.assertNotIn(secret, sanitized)
        self.assertIn(
            "The password policy and private key documentation remain ordinary prose.",
            sanitized,
        )

    def test_sanitizer_scrubs_windows_root_case_and_separator_variants(self):
        diagnostic = (
            r"failed at c:/CONTROLLED-CORPUS/request/session.evidence.json"
            "\n"
            r"failed at C:\controlled-corpus\request\session.trace.json"
        )

        sanitized = sanitize_with_powershell(diagnostic)

        self.assertNotIn("controlled-corpus", sanitized.lower())
        self.assertEqual(sanitized.count("<input-root>"), 2)

    def test_output_path_rejects_junction_components_before_writing(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            input_root = root / "input"
            output_container = root / "output"
            junction = output_container / "redirect"
            input_root.mkdir()
            output_container.mkdir()
            environment = os.environ.copy()
            environment["ARGORIX_TEST_JUNCTION"] = str(junction)
            environment["ARGORIX_TEST_JUNCTION_TARGET"] = str(input_root)
            subprocess.run(
                [
                    "powershell",
                    "-NoProfile",
                    "-Command",
                    "New-Item -ItemType Junction -Path "
                    "$env:ARGORIX_TEST_JUNCTION -Target "
                    "$env:ARGORIX_TEST_JUNCTION_TARGET | Out-Null",
                ],
                check=True,
                capture_output=True,
                text=True,
                env=environment,
            )

            completed = subprocess.run(
                [
                    "powershell",
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    str(SCRIPT),
                    "-InputRoot",
                    str(input_root),
                    "-OutputPath",
                    str(junction / "verification-results.json"),
                ],
                capture_output=True,
                text=True,
            )

            self.assertNotEqual(completed.returncode, 0)
            self.assertFalse((input_root / "verification-results.json").exists())
            self.assertIn("reparse", (completed.stdout + completed.stderr).lower())

    def test_verifier_build_is_independent_of_caller_working_directory(self):
        source = SCRIPT.read_text(encoding="utf-8")
        self.assertIn("--manifest-path", source)
        self.assertIn('Join-Path $repositoryRoot "Cargo.toml"', source)
        self.assertIn('CARGO_TARGET_DIR', source)
        input_root = (
            PAPER.parent / "../../demo/argorix-chatbot-runtime/generated"
        ).resolve()
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "verification.json"
            completed = subprocess.run(
                [
                    "powershell", "-NoProfile", "-ExecutionPolicy", "Bypass",
                    "-File", str(SCRIPT), "-InputRoot", str(input_root),
                    "-OutputPath", str(output),
                    "-CargoPath", r"C:\Users\nanos\.cargo\bin\cargo.exe",
                ],
                cwd=temporary, capture_output=True, text=True,
            )
            self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)
            records = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(len(records), 27)
            self.assertTrue(all(r["verified"] and r["exit_code"] == 0 for r in records))


def sanitize_with_powershell(diagnostic, sentinel="controlled-environment"):
    command = r"""
$tokens = $null
$errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile(
    $env:ARGORIX_TEST_SCRIPT, [ref]$tokens, [ref]$errors
)
if ($errors.Count -ne 0) { throw $errors[0] }
$function = $ast.Find({
    param($node)
    $node -is [Management.Automation.Language.FunctionDefinitionAst] -and
        $node.Name -eq 'ConvertTo-SafeDiagnostic'
}, $true)
Invoke-Expression $function.Extent.Text
$diagnostic = [Console]::In.ReadToEnd()
ConvertTo-SafeDiagnostic $diagnostic 'C:\controlled-corpus'
"""
    environment = os.environ.copy()
    environment["ARGORIX_TEST_SECRET_SENTINEL"] = sentinel
    environment["ARGORIX_TEST_SCRIPT"] = str(SCRIPT)
    completed = subprocess.run(
        ["powershell", "-NoProfile", "-Command", command],
        input=diagnostic,
        text=True,
        capture_output=True,
        check=True,
        env=environment,
    )
    return completed.stdout.strip()


if __name__ == "__main__":
    unittest.main()
