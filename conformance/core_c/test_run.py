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
