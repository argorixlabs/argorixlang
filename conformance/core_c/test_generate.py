"""Unit tests for the differential oracle in conformance/core_c/generate.py.

The oracle decides what every generated program is expected to print, so its
own rules are tested here against the clauses of `spec/core/evaluation.md`.

Run with: python3 -m unittest discover -s conformance/core_c -p "test_*.py"
"""

from __future__ import annotations

import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import generate  # noqa: E402


def lit(value, kind="i32"):
    return ("lit", kind, value)


def run(node, env=None, functions=None):
    return generate.evaluate(node, env or {}, functions or {}, [generate.MAX_LOOP_ITERATIONS])


class Arithmetic(unittest.TestCase):
    def test_exact_width_overflow_traps(self):
        for kind, left, right in (("u8", 255, 1), ("i8", 127, 1), ("u16", 65535, 1),
                                  ("i64", 2 ** 63 - 1, 1)):
            with self.subTest(kind=kind), self.assertRaises(generate.Trap) as raised:
                generate.arithmetic("+", left, right, kind)
            self.assertEqual(raised.exception.code, "INTEGER_OVERFLOW")

    def test_unsigned_underflow_traps(self):
        with self.assertRaises(generate.Trap):
            generate.arithmetic("-", 0, 1, "u32")

    def test_division_by_zero_traps(self):
        for op in ("/", "%"):
            with self.subTest(op=op), self.assertRaises(generate.Trap) as raised:
                generate.arithmetic(op, 7, 0, "i32")
            self.assertEqual(raised.exception.code, "DIVISION_BY_ZERO")

    def test_minimum_divided_by_minus_one_traps(self):
        for op in ("/", "%"):
            with self.subTest(op=op), self.assertRaises(generate.Trap) as raised:
                generate.arithmetic(op, -(2 ** 31), -1, "i32")
            self.assertEqual(raised.exception.code, "INTEGER_OVERFLOW")

    def test_division_truncates_toward_zero(self):
        self.assertEqual(generate.arithmetic("/", -7, 2, "i32"), -3)
        self.assertEqual(generate.arithmetic("/", 7, -2, "i32"), -3)
        self.assertEqual(generate.arithmetic("%", -7, 2, "i32"), -1)
        self.assertEqual(generate.arithmetic("%", 7, -2, "i32"), 1)

    def test_bitwise_operators(self):
        self.assertEqual(generate.arithmetic("&", 12, 10, "u32"), 8)
        self.assertEqual(generate.arithmetic("|", 12, 10, "u32"), 14)
        self.assertEqual(generate.arithmetic("^", 12, 10, "u32"), 6)

    def test_results_inside_the_range_do_not_trap(self):
        self.assertEqual(generate.arithmetic("*", 16, 15, "u8"), 240)


class Evaluation(unittest.TestCase):
    def test_left_operand_traps_first(self):
        node = ("bin", "+", ("bin", "/", lit(1), lit(0), "i32"), ("bin", "+", lit(2 ** 31 - 1), lit(1)), "i32")
        with self.assertRaises(generate.Trap) as raised:
            run(node)
        self.assertEqual(raised.exception.code, "DIVISION_BY_ZERO")

    def test_and_short_circuits_before_a_trap(self):
        trapping = ("compare", "==", ("bin", "/", lit(1), lit(0), "i32"), lit(0))
        self.assertFalse(run(("and", ("compare", "==", lit(1), lit(2)), trapping)))

    def test_or_short_circuits_before_a_trap(self):
        trapping = ("compare", "==", ("bin", "/", lit(1), lit(0), "i32"), lit(0))
        self.assertTrue(run(("or", ("compare", "==", lit(1), lit(1)), trapping)))

    def test_if_evaluates_only_the_taken_branch(self):
        trapping = ("bin", "/", lit(1), lit(0), "i32")
        self.assertEqual(run(("if", ("compare", "<", lit(1), lit(2)), lit(5), trapping)), 5)

    def test_while_runs_until_the_condition_is_false(self):
        env = {"i": 0, "total": 0}
        body = [("compound", "total", "+", ("var", "i"), "i32"),
                ("compound", "i", "+", lit(1), "i32")]
        generate.execute(("while", ("compare", "<", ("var", "i"), lit(4)), body), env, {},
                         [generate.MAX_LOOP_ITERATIONS])
        self.assertEqual((env["i"], env["total"]), (4, 6))

    def test_a_loop_that_does_not_finish_is_reported(self):
        env = {"i": 0}
        with self.assertRaises(RuntimeError):
            generate.execute(("while", ("compare", "==", lit(1), lit(1)), []), env, {}, [10])

    def test_calls_pass_arguments_by_position(self):
        functions = {"f": {"name": "f", "params": [("a", "i32"), ("b", "i32")], "ret": "i32",
                           "body": [], "tail": ("bin", "-", ("var", "a"), ("var", "b"), "i32")}}
        self.assertEqual(run(("call", "f", [lit(10), lit(4)]), {}, functions), 6)


class Rendering(unittest.TestCase):
    def test_negative_literals_are_rendered_without_an_out_of_range_literal(self):
        self.assertEqual(generate.render_expression(lit(-5, "i8")), "(0i8 - 5i8)")
        self.assertEqual(generate.render_expression(lit(-128, "i8")), "(0i8 - 127i8 - 1i8)")

    def test_if_expressions_are_parenthesised(self):
        node = ("if", ("compare", "<", lit(1), lit(2)), lit(3), lit(4))
        self.assertTrue(generate.render_expression(node).startswith("(if "))


class Corpus(unittest.TestCase):
    def test_build_corpus_writes_programs_and_expectations(self):
        with tempfile.TemporaryDirectory() as directory:
            out = pathlib.Path(directory)
            manifest = generate.build_corpus(seed=3, count=12, out=out)
            self.assertEqual(len(manifest["cases"]), 12)
            self.assertEqual(manifest["schema_version"], 1)
            for case in manifest["cases"]:
                with self.subTest(case=case["id"]):
                    source = (out / case["file"]).read_text(encoding="utf-8")
                    self.assertTrue(source.startswith("core 0.1;"))
                    self.assertIn("pub fn argorix_main()", source)
                    if case["expected_exit"] == 0:
                        self.assertTrue(case["expected_stdout"].startswith("ARGORIX_RESULT:"))
                        self.assertEqual(case["expected_stderr"], "")
                    else:
                        self.assertEqual(case["expected_exit"], 70)
                        self.assertTrue(case["expected_stderr"].startswith("ARGORIX_TRAP:"))

    def test_the_same_seed_produces_the_same_corpus(self):
        with tempfile.TemporaryDirectory() as first, tempfile.TemporaryDirectory() as second:
            one = generate.build_corpus(seed=9, count=8, out=pathlib.Path(first))
            two = generate.build_corpus(seed=9, count=8, out=pathlib.Path(second))
            self.assertEqual(one["cases"], two["cases"])

    def test_generated_programs_avoid_the_shapes_of_known_gaps(self):
        with tempfile.TemporaryDirectory() as directory:
            out = pathlib.Path(directory)
            generate.build_corpus(seed=5, count=30, out=out)
            for path in out.glob("*.argx"):
                source = path.read_text(encoding="utf-8")
                with self.subTest(program=path.name):
                    self.assertNotIn("<<", source)
                    self.assertNotIn(">>", source)
                    self.assertNotIn("Array<", source)
                    self.assertNotIn("struct ", source)
                    # No `if` statement: every `if` is an expression in a tail,
                    # a value, or a condition, never a bare statement.
                    for line in source.splitlines():
                        self.assertFalse(line.strip().startswith("if ") and line.rstrip().endswith("}"),
                                         f"if statement in {path.name}: {line}")


if __name__ == "__main__":
    unittest.main()
