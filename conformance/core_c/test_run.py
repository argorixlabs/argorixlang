"""Unit tests for conformance/core_c/run.py that need no C compiler.

Run with: python3 -m unittest discover -s conformance/core_c -p "test_*.py"
"""

from __future__ import annotations

import json
import pathlib
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import run  # noqa: E402


POLICY = json.loads(run.POLICY.read_text(encoding="utf-8"))
TOOLCHAIN = {
    "allowed_compiler_names": ["cc", "clang", "gcc"],
    "required_flags": ["-std=c11", "-Wall", "-Wextra", "-Werror"],
    "runtime_sources": ["bootstrap/c/argorix_core_runtime.c", "bootstrap/c/argorix_core_runtime.h"],
    "forbidden_link_inputs": ["*.rlib", "*.rustlib", "cargo", "rustc"],
}

READELF_DYNAMIC = """
Dynamic section at offset 0x2dc8 contains 27 entries:
  Tag        Type                         Name/Value
 0x0000000000000001 (NEEDED)             Shared library: [libc.so.6]
 0x000000000000000c (INIT)               0x1000
"""

READELF_SYMBOLS = """
Symbol table '.dynsym' contains 3 entries:
   Num:    Value          Size Type    Bind   Vis      Ndx Name
     0: 0000000000000000     0 NOTYPE  LOCAL  DEFAULT  UND
     1: 0000000000000000     0 FUNC    GLOBAL DEFAULT  UND exit@GLIBC_2.2.5 (2)
     2: 0000000000000000     0 FUNC    GLOBAL DEFAULT  UND fprintf@GLIBC_2.2.5 (2)

Symbol table '.symtab' contains 3 entries:
   Num:    Value          Size Type    Bind   Vis      Ndx Name
    40: 0000000000001189    37 FUNC    GLOBAL DEFAULT   16 argorix_trap
    41: 00000000000011ae    52 FUNC    GLOBAL DEFAULT   16 argorix_trust_check
    42: 0000000000001200    12 FUNC    GLOBAL DEFAULT   16 main
"""


def case(**overrides):
    base = {
        "id": "scalar_success",
        "file": "scalar_success.argx",
        "expected_exit": 0,
        "expected_stdout": "ARGORIX_RESULT:42",
        "expected_stderr": "",
    }
    base.update(overrides)
    return base


class CompilerResolution(unittest.TestCase):
    def test_rejects_names_outside_allow_list(self):
        for name in ("sh", "bash", "rustc", "cargo", "cc; rm -rf /", "gcc -O2", "./cc", "cc/../sh"):
            with self.subTest(name=name), self.assertRaises(run.RunnerError):
                run.resolve_compiler(name, TOOLCHAIN)

    def test_resolves_allow_listed_name_on_path(self):
        with mock.patch.object(run.shutil, "which", return_value="/usr/bin/gcc"):
            self.assertEqual(run.resolve_compiler("gcc", TOOLCHAIN), pathlib.Path("/usr/bin/gcc"))

    def test_reports_missing_allow_listed_compiler(self):
        with mock.patch.object(run.shutil, "which", return_value=None):
            with self.assertRaises(run.RunnerError):
                run.resolve_compiler("clang", TOOLCHAIN)


class ArgumentVector(unittest.TestCase):
    def test_compile_argv_is_a_list_with_required_flags(self):
        argv = run.compile_argv(pathlib.Path("/usr/bin/gcc"), TOOLCHAIN,
                                [pathlib.Path("/w/case.c")], pathlib.Path("/w/case"))
        self.assertIsInstance(argv, list)
        self.assertEqual(argv[0], str(pathlib.Path("/usr/bin/gcc")))
        for flag in TOOLCHAIN["required_flags"]:
            self.assertIn(flag, argv)
        self.assertIn("-o", argv)
        self.assertTrue(any(item.endswith("argorix_core_runtime.c") for item in argv))
        self.assertFalse(any(item.endswith(".h") for item in argv))

    def test_forbidden_link_inputs_are_rejected(self):
        for item in ("/x/libstd.rlib", "/x/foo.rustlib", "/usr/bin/cargo", "rustc"):
            with self.subTest(item=item), self.assertRaises(run.RunnerError):
                run.check_argv(["/usr/bin/gcc", item], TOOLCHAIN)

    def test_run_argv_rejects_non_list_commands(self):
        with self.assertRaises(run.RunnerError):
            run.run_argv([])
        with self.assertRaises(run.RunnerError):
            run.run_argv(["gcc", 3])  # type: ignore[list-item]


class CaseValidation(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.base = pathlib.Path(self.tmp.name)
        (self.base / "scalar_success.argx").write_text("core 0.1;\n", encoding="utf-8")

    def tearDown(self):
        self.tmp.cleanup()

    def test_accepts_well_formed_case(self):
        run.validate_case(case(), self.base)

    def test_rejects_path_traversal_and_absolute_paths(self):
        for file in ("../escape.argx", "/etc/passwd.argx", "..\\escape.argx", "sub/../../x.argx"):
            with self.subTest(file=file), self.assertRaises(run.RunnerError):
                run.validate_case(case(file=file), self.base)

    def test_rejects_ids_that_are_not_safe_file_names(self):
        for case_id in ("Scalar", "a b", "../x", "x;y", ""):
            with self.subTest(case_id=case_id), self.assertRaises(run.RunnerError):
                run.validate_case(case(id=case_id), self.base)

    def test_rejects_wrong_field_types(self):
        with self.assertRaises(run.RunnerError):
            run.validate_case(case(expected_exit="0"), self.base)
        with self.assertRaises(run.RunnerError):
            run.validate_case(case(expected_exit=True), self.base)

    def test_rejects_missing_source(self):
        with self.assertRaises(run.RunnerError):
            run.validate_case(case(file="missing.argx"), self.base)


class Comparison(unittest.TestCase):
    def test_exact_match_with_one_trailing_newline(self):
        self.assertEqual(run.compare(case(), 0, "ARGORIX_RESULT:42\n", ""), [])

    def test_trap_case(self):
        trap = case(expected_exit=70, expected_stdout="",
                    expected_stderr="ARGORIX_TRAP:INTEGER_OVERFLOW")
        self.assertEqual(run.compare(trap, 70, "", "ARGORIX_TRAP:INTEGER_OVERFLOW\n"), [])

    def test_detects_each_kind_of_mismatch(self):
        self.assertEqual(len(run.compare(case(), 1, "ARGORIX_RESULT:42\n", "")), 1)
        self.assertEqual(len(run.compare(case(), 0, "ARGORIX_RESULT:41\n", "")), 1)
        self.assertEqual(len(run.compare(case(), 0, "ARGORIX_RESULT:42\n", "noise\n")), 1)
        self.assertEqual(len(run.compare(case(), 0, "ARGORIX_RESULT:42\n\n", "")), 1)
        self.assertEqual(len(run.compare(case(), None, "", "")), 2)


class Inspection(unittest.TestCase):
    def test_parses_needed_libraries(self):
        self.assertEqual(run.parse_needed(READELF_DYNAMIC), ["libc.so.6"])

    def test_parses_symbols_and_strips_versions(self):
        symbols = run.parse_symbols(READELF_SYMBOLS)
        self.assertIn(("exit", "UND"), symbols)
        self.assertIn(("argorix_trap", "16"), symbols)

    def test_clean_binary_has_no_violations(self):
        symbols = run.parse_symbols(READELF_SYMBOLS)
        self.assertEqual(run.classify(["libc.so.6"], symbols, b"\x7fELF...", POLICY), [])

    def test_trust_named_symbols_are_not_false_positives(self):
        self.assertEqual(run.classify([], [("argorix_trust_check", "16"), ("entrust_value", "16")],
                                      b"", POLICY), [])

    def test_flags_rust_libraries(self):
        violations = run.classify(["libstd-8e1f.so", "librust_sensor.so"], [], b"", POLICY)
        self.assertTrue(any("libstd-8e1f.so" in item for item in violations))
        self.assertTrue(any("librust_sensor.so" in item for item in violations))

    def test_flags_unknown_libraries(self):
        violations = run.classify(["libm.so.6"], [], b"", POLICY)
        self.assertEqual(violations, ["NEEDED library not allow-listed: libm.so.6"])

    def test_flags_rust_symbols_in_both_manglings(self):
        for name in ("__rust_alloc", "rust_begin_unwind",
                     "_ZN4core9panicking5panic17h0123456789abcdefE", "_RNvCs1234_7mycrate3foo"):
            with self.subTest(name=name):
                self.assertTrue(run.classify([], [(name, "16")], b"", POLICY))

    def test_flags_process_spawning_imports_only_when_imported(self):
        self.assertTrue(run.classify([], [("system", "UND")], b"", POLICY))
        self.assertTrue(run.classify([], [("execve", "UND")], b"", POLICY))
        self.assertEqual(run.classify([], [("system_status", "UND")], b"", POLICY), [])

    def test_flags_toolchain_byte_markers(self):
        violations = run.classify([], [], b"...\x00/rustc/abc123/library/core/src/panicking.rs", POLICY)
        self.assertEqual(violations, ["contains byte marker: /rustc/"])


if __name__ == "__main__":
    unittest.main()


def outcome(**overrides):
    base = {"emit_exit": 0, "emit_stderr": "", "compile_exit": 0, "compile_diagnostics": "",
            "exit": 0, "stdout": "", "stderr": ""}
    base.update(overrides)
    return base


GAP_REJECTED = {"id": "g13", "kind": "emit_rejected", "spec_expected_stdout": "ARGORIX_RESULT:16",
                "signature": "checked shifts are not implemented yet"}
GAP_COMPILE = {"id": "g04", "kind": "compile_error", "spec_expected_stdout": "ARGORIX_RESULT:42",
               "signature": "redefinition of"}
GAP_WRONG = {"id": "g11", "kind": "wrong_result", "spec_expected_stdout": "ARGORIX_RESULT:3",
             "observed_stdout": "ARGORIX_RESULT:4", "observed_exit": 0}
GAP_CRASH = {"id": "g15", "kind": "crash", "spec_expected_stdout": "ARGORIX_RESULT:400000",
             "observed_exit": -11, "stack_limit_mib": 8}


class GapClassification(unittest.TestCase):
    def status(self, gap, **kwargs):
        return run.classify_gap(gap, outcome(**kwargs))[0]

    def test_recorded_emission_rejection_is_still_open(self):
        self.assertEqual(self.status(GAP_REJECTED, emit_exit=1,
                                     emit_stderr="CBackendUnsupported: checked shifts are not implemented yet"),
                         run.STILL_OPEN)

    def test_emission_rejection_with_another_message_changed(self):
        self.assertEqual(self.status(GAP_REJECTED, emit_exit=1, emit_stderr="internal error"), run.CHANGED)

    def test_emission_rejection_that_now_runs_correctly_is_fixed(self):
        self.assertEqual(self.status(GAP_REJECTED, stdout="ARGORIX_RESULT:16\n"), run.FIXED)

    def test_recorded_compile_error_is_still_open(self):
        self.assertEqual(self.status(GAP_COMPILE, compile_exit=1,
                                     compile_diagnostics="error: redefinition of 'argorix_v_t'"),
                         run.STILL_OPEN)

    def test_other_compile_error_is_changed(self):
        self.assertEqual(self.status(GAP_COMPILE, compile_exit=1,
                                     compile_diagnostics="error: unknown type name"), run.CHANGED)

    def test_compile_error_that_now_compiles_and_runs_is_fixed(self):
        self.assertEqual(self.status(GAP_COMPILE, stdout="ARGORIX_RESULT:42\n"), run.FIXED)

    def test_compiles_but_prints_something_else_is_changed(self):
        self.assertEqual(self.status(GAP_COMPILE, stdout="ARGORIX_RESULT:7\n"), run.CHANGED)

    def test_recorded_wrong_result_is_still_open(self):
        self.assertEqual(self.status(GAP_WRONG, stdout="ARGORIX_RESULT:4\n"), run.STILL_OPEN)

    def test_wrong_result_corrected_is_fixed(self):
        self.assertEqual(self.status(GAP_WRONG, stdout="ARGORIX_RESULT:3\n"), run.FIXED)

    def test_a_third_wrong_value_is_changed(self):
        self.assertEqual(self.status(GAP_WRONG, stdout="ARGORIX_RESULT:5\n"), run.CHANGED)

    def test_recorded_crash_is_still_open(self):
        self.assertEqual(self.status(GAP_CRASH, exit=-11), run.STILL_OPEN)

    def test_crash_replaced_by_typed_trap_is_fixed(self):
        self.assertEqual(self.status(GAP_CRASH, exit=70, stderr="ARGORIX_TRAP:STEP_LIMIT\n"), run.FIXED)

    def test_crash_replaced_by_the_expected_result_is_fixed(self):
        self.assertEqual(self.status(GAP_CRASH, stdout="ARGORIX_RESULT:400000\n"), run.FIXED)

    def test_emission_that_starts_failing_is_changed(self):
        self.assertEqual(self.status(GAP_WRONG, emit_exit=1, emit_stderr="boom"), run.CHANGED)


class GapManifest(unittest.TestCase):
    def test_every_gap_has_a_program_and_a_usable_record(self):
        manifest = json.loads(run.GAPS.read_text(encoding="utf-8"))
        self.assertEqual(manifest["schema_version"], 1)
        for gap in manifest["gaps"]:
            with self.subTest(gap=gap["id"]):
                self.assertTrue((run.GAPS.parent / gap["file"]).is_file())
                self.assertIn(gap["kind"], ("emit_rejected", "compile_error", "wrong_result", "crash"))
                self.assertTrue(gap["spec_expected_stdout"].startswith("ARGORIX_RESULT:"))
                self.assertTrue(gap.get("note"))
                if gap["kind"] in ("emit_rejected", "compile_error"):
                    self.assertTrue(gap.get("signature"))
                if gap["kind"] == "wrong_result":
                    self.assertNotEqual(gap["observed_stdout"], gap["spec_expected_stdout"])
