#!/usr/bin/env python3
"""Generate random Core 0.1 programs with their spec-derived expected results.

ESP-008.R differential testing. The programs are built from an AST that is
evaluated here, by rules taken from `spec/core/evaluation.md` and
`spec/core/types.md`: exact-width arithmetic that traps on overflow, trapping
division by zero and `MIN / -1`, truncating `/` and `%`, strict left-to-right
evaluation, and short-circuiting `&&` and `||`. Nothing in this file consults
`argorixc`, the C backend, or the C runtime, so the expected result is an
independent oracle, as the master plan requires.

The output is an ordinary case corpus, so the existing pipeline runs it:

    python3 conformance/core_c/generate.py --seed 7 --count 50 --out target/core-c/fuzz
    python3 conformance/core_c/run.py all --argorixc target/debug/argorixc --cc gcc \\
        --cases target/core-c/fuzz/cases.json --bundle target/core-c/fuzz-bundle

A failure is a real divergence between the backend and the spec, not a known
gap: the generator stays away from the shapes recorded in
`conformance/core_c/gaps/` (no shadowing, no block operands, no shifts, no
signed negation, no `if` statements, no unused parameters or functions, no
recursion, and no `if` expression anywhere inside a comparison operand).
"""

from __future__ import annotations

import argparse
import json
import pathlib
import random
import sys
from typing import Any


INT_TYPES = {
    "u8": (0, 2 ** 8 - 1),
    "u16": (0, 2 ** 16 - 1),
    "u32": (0, 2 ** 32 - 1),
    "u64": (0, 2 ** 64 - 1),
    "i8": (-(2 ** 7), 2 ** 7 - 1),
    "i16": (-(2 ** 15), 2 ** 15 - 1),
    "i32": (-(2 ** 31), 2 ** 31 - 1),
    "i64": (-(2 ** 63), 2 ** 63 - 1),
}
ARITH = ["+", "-", "*", "/", "%", "&", "|", "^"]
COMPARE = ["==", "!=", "<", "<=", ">", ">="]
MAX_LOOP_ITERATIONS = 10_000


class Trap(Exception):
    """A typed Core trap, as the runtime would report it."""

    def __init__(self, code: str) -> None:
        super().__init__(code)
        self.code = code


# --------------------------------------------------------------------------
# Oracle: evaluate the AST by the specification


def check_range(value: int, kind: str) -> int:
    low, high = INT_TYPES[kind]
    if value < low or value > high:
        raise Trap("INTEGER_OVERFLOW")
    return value


def truncating_div(left: int, right: int) -> int:
    quotient = abs(left) // abs(right)
    return -quotient if (left < 0) != (right < 0) else quotient


def arithmetic(op: str, left: int, right: int, kind: str) -> int:
    if op in ("/", "%"):
        if right == 0:
            raise Trap("DIVISION_BY_ZERO")
        low, _ = INT_TYPES[kind]
        if left == low and right == -1:
            raise Trap("INTEGER_OVERFLOW")
        quotient = truncating_div(left, right)
        return check_range(quotient if op == "/" else left - quotient * right, kind)
    if op == "+":
        return check_range(left + right, kind)
    if op == "-":
        return check_range(left - right, kind)
    if op == "*":
        return check_range(left * right, kind)
    if op == "&":
        return left & right
    if op == "|":
        return left | right
    if op == "^":
        return left ^ right
    raise ValueError(f"unsupported operator {op}")


def evaluate(node: tuple, env: dict[str, int], functions: dict[str, dict[str, Any]], fuel: list[int]) -> Any:
    """Evaluate one expression. Operands run strictly left to right."""
    kind = node[0]
    if kind == "lit":
        return node[2]
    if kind == "var":
        return env[node[1]]
    if kind == "not":
        return not evaluate(node[1], env, functions, fuel)
    if kind == "and":
        return bool(evaluate(node[1], env, functions, fuel)) and bool(evaluate(node[2], env, functions, fuel))
    if kind == "or":
        return bool(evaluate(node[1], env, functions, fuel)) or bool(evaluate(node[2], env, functions, fuel))
    if kind == "compare":
        left = evaluate(node[2], env, functions, fuel)
        right = evaluate(node[3], env, functions, fuel)
        return {"==": left == right, "!=": left != right, "<": left < right,
                "<=": left <= right, ">": left > right, ">=": left >= right}[node[1]]
    if kind == "bin":
        left = evaluate(node[2], env, functions, fuel)
        right = evaluate(node[3], env, functions, fuel)
        return arithmetic(node[1], left, right, node[4])
    if kind == "if":
        branch = node[2] if evaluate(node[1], env, functions, fuel) else node[3]
        return evaluate(branch, env, functions, fuel)
    if kind == "call":
        function = functions[node[1]]
        arguments = [evaluate(argument, env, functions, fuel) for argument in node[2]]
        local = dict(zip([name for name, _ in function["params"]], arguments))
        return run_body(function["body"], function["tail"], local, functions, fuel)
    raise ValueError(f"unsupported node {kind}")


def run_body(body: list[tuple], tail: tuple, env: dict[str, int],
             functions: dict[str, dict[str, Any]], fuel: list[int]) -> Any:
    for statement in body:
        execute(statement, env, functions, fuel)
    return evaluate(tail, env, functions, fuel)


def execute(statement: tuple, env: dict[str, int], functions: dict[str, dict[str, Any]], fuel: list[int]) -> None:
    kind = statement[0]
    if kind == "let":
        env[statement[1]] = evaluate(statement[3], env, functions, fuel)
    elif kind == "assign":
        env[statement[1]] = evaluate(statement[2], env, functions, fuel)
    elif kind == "compound":
        env[statement[1]] = arithmetic(statement[2], env[statement[1]],
                                       evaluate(statement[3], env, functions, fuel), statement[4])
    elif kind == "while":
        while evaluate(statement[1], env, functions, fuel):
            fuel[0] -= 1
            if fuel[0] <= 0:
                raise RuntimeError("program does not terminate quickly enough")
            for inner in statement[2]:
                execute(inner, env, functions, fuel)
    else:
        raise ValueError(f"unsupported statement {kind}")


# --------------------------------------------------------------------------
# Rendering: the same AST as Core source


def render_expression(node: tuple) -> str:
    kind = node[0]
    if kind == "lit":
        if node[1] == "bool":
            return "true" if node[2] else "false"
        kind, value = node[1], node[2]
        if value >= 0:
            return f"{value}{kind}"
        high = INT_TYPES[kind][1]
        if abs(value) > high:  # the minimum has no positive literal of its own
            return f"(0{kind} - {high}{kind} - 1{kind})"
        return f"(0{kind} - {abs(value)}{kind})"
    if kind == "var":
        return node[1]
    if kind == "not":
        return f"!{render_expression(node[1])}"
    if kind in ("and", "or"):
        operator = "&&" if kind == "and" else "||"
        return f"({render_expression(node[1])} {operator} {render_expression(node[2])})"
    if kind in ("compare", "bin"):
        return f"({render_expression(node[2])} {node[1]} {render_expression(node[3])})"
    if kind == "if":
        return (f"(if {render_expression(node[1])} {{ {render_expression(node[2])} }} "
                f"else {{ {render_expression(node[3])} }})")
    if kind == "call":
        return f"{node[1]}({', '.join(render_expression(argument) for argument in node[2])})"
    raise ValueError(f"unsupported node {kind}")


def render_statements(body: list[tuple], indent: str) -> list[str]:
    lines = []
    for statement in body:
        kind = statement[0]
        if kind == "let":
            mutable = "mut " if statement[4] else ""
            lines.append(f"{indent}let {mutable}{statement[1]}: {statement[2]} = {render_expression(statement[3])};")
        elif kind == "assign":
            lines.append(f"{indent}{statement[1]} = {render_expression(statement[2])};")
        elif kind == "compound":
            lines.append(f"{indent}{statement[1]} {statement[2]}= {render_expression(statement[3])};")
        elif kind == "while":
            lines.append(f"{indent}while {render_expression(statement[1])} {{")
            lines += render_statements(statement[2], indent + "    ")
            lines.append(f"{indent}}}")
    return lines


def render_program(program: dict[str, Any]) -> str:
    lines = [f"core 0.1;", f"module fuzz.{program['id']};", ""]
    for function in program["functions"]:
        parameters = ", ".join(f"{name}: {kind}" for name, kind in function["params"])
        lines.append(f"fn {function['name']}({parameters}) -> {function['ret']} {{")
        lines += render_statements(function["body"], "    ")
        lines.append(f"    {render_expression(function['tail'])}")
        lines.append("}")
        lines.append("")
    main = program["main"]
    lines.append(f"pub fn argorix_main() -> {main['ret']} {{")
    lines += render_statements(main["body"], "    ")
    lines.append(f"    {render_expression(main['tail'])}")
    lines.append("}")
    return "\n".join(lines) + "\n"


# --------------------------------------------------------------------------
# Generation


class Generator:
    def __init__(self, rng: random.Random, kind: str) -> None:
        self.rng = rng
        self.kind = kind
        self.counter = 0

    def fresh(self, prefix: str) -> str:
        self.counter += 1
        return f"{prefix}{self.counter}"

    def literal(self) -> tuple:
        low, high = INT_TYPES[self.kind]
        # Small values keep most programs away from an overflow trap, while the
        # occasional extreme value exercises the checked paths.
        if self.rng.random() < 0.1:
            value = self.rng.choice([low, high, high // 2, 0, 1])
        else:
            value = self.rng.randint(max(low, -20), min(high, 20))
        return ("lit", self.kind, value)

    def value(self, names: list[str], depth: int, functions: list[dict[str, Any]],
              allow_if: bool = True) -> tuple:
        if depth <= 0 or self.rng.random() < 0.3:
            if names and self.rng.random() < 0.6:
                return ("var", self.rng.choice(names))
            return self.literal()
        choice = self.rng.random()
        if choice < 0.55:
            op = self.rng.choice(ARITH)
            left = self.value(names, depth - 1, functions, allow_if)
            right = self.value(names, depth - 1, functions, allow_if)
            if op in ("/", "%") and right[0] == "lit" and right[2] == 0:
                right = ("lit", self.kind, self.rng.randint(1, 20))
            if op == "^" and left[0] == "lit" and right[0] == "lit":
                # GCC reads `10 ^ 6` in the generated C as a mistyped power
                # (-Wxor-used-as-pow), so one side is always a computed value.
                left = ("bin", "+", left, ("lit", self.kind, 0), self.kind)
            return ("bin", op, left, right, self.kind)
        if choice < 0.75 and allow_if:
            return ("if", self.condition(names, depth - 1, functions),
                    self.value(names, depth - 1, functions), self.value(names, depth - 1, functions))
        if choice < 0.9 and functions:
            function = self.rng.choice(functions)
            return ("call", function["name"],
                    [self.value(names, depth - 1, functions, allow_if) for _ in function["params"]])
        return self.literal()

    def condition(self, names: list[str], depth: int, functions: list[dict[str, Any]]) -> tuple:
        choice = self.rng.random()
        if choice < 0.6 or depth <= 0:
            left = self.value(names, depth, functions, allow_if=False)
            right = self.value(names, depth, functions, allow_if=False)
            if render_expression(left) == render_expression(right):
                right = self.literal()
                if render_expression(left) == render_expression(right):
                    right = ("bin", "+", right, ("lit", self.kind, 1), self.kind)
            op = self.rng.choice(COMPARE)
            if not self.kind.startswith("i") and op in ("<", "<=", ">", ">="):
                # `unsigned < 0` and `0 > unsigned` are always false, which the
                # declared -Wtype-limits profile rejects in the generated C.
                if left[0] == "lit" and left[2] == 0:
                    left = ("lit", self.kind, self.rng.randint(1, 20))
                if right[0] == "lit" and right[2] == 0:
                    right = ("lit", self.kind, self.rng.randint(1, 20))
            low, high = INT_TYPES[self.kind]
            for side in ("left", "right"):
                node = left if side == "left" else right
                # A literal at the edge of the type makes the comparison
                # constant, which -Wtype-limits rejects.
                if op in ("<", "<=", ">", ">=") and node[0] == "lit" and node[2] in (low, high):
                    replacement = ("lit", self.kind, self.rng.randint(1, 20))
                    left, right = (replacement, right) if side == "left" else (left, replacement)
            return ("compare", op, left, right)
        if choice < 0.8:
            return ("and" if self.rng.random() < 0.5 else "or",
                    self.condition(names, depth - 1, functions), self.condition(names, depth - 1, functions))
        return ("not", self.condition(names, depth - 1, functions))

    def statements(self, names: list[str], functions: list[dict[str, Any]], allow_loop: bool,
                   mutable_names: list[str] | None = None) -> list[tuple]:
        """Build a statement list. `names` is every readable name in scope;
        `mutable_names` is the subset that may be assigned (`let mut` locals).
        Parameters are by value and immutable, so they never appear there."""
        mutable_names = [] if mutable_names is None else mutable_names
        body: list[tuple] = []
        for _ in range(self.rng.randint(1, 4)):
            choice = self.rng.random()
            if choice < 0.5 or not mutable_names:
                name = self.fresh("v")
                mutable = self.rng.random() < 0.5
                body.append(("let", name, self.kind, self.value(names, 2, functions), mutable))
                names.append(name)
                if mutable:
                    mutable_names.append(name)
            elif choice < 0.7:
                body.append(("assign", self.rng.choice(mutable_names), self.value(names, 2, functions)))
            elif choice < 0.85 or not allow_loop:
                body.append(("compound", self.rng.choice(mutable_names), self.rng.choice(["+", "-", "*"]),
                             self.literal(), self.kind))
            else:
                body.append(self.loop(names, mutable_names, functions))
        return body

    def loop(self, names: list[str], mutable_names: list[str], functions: list[dict[str, Any]]) -> tuple:
        counter = self.fresh("i")
        limit = self.rng.randint(1, 6)
        inner: list[tuple] = []
        if mutable_names:
            inner.append(("assign", self.rng.choice(mutable_names), self.value(names + [counter], 1, functions)))
        inner.append(("compound", counter, "+", ("lit", self.kind, 1), self.kind))
        names.append(counter)
        mutable_names.append(counter)
        return ("seq_loop", counter, limit, inner)

    def function(self, index: int, functions: list[dict[str, Any]]) -> dict[str, Any]:
        name = f"helper{index}"
        params = [(f"p{index}_{position}", self.kind) for position in range(self.rng.randint(1, 2))]
        names = [name for name, _ in params]
        body = self.statements(list(names), functions, allow_loop=False, mutable_names=[])
        # Every parameter is used, so the generated C survives -Wunused-parameter.
        tail: tuple = ("var", names[0])
        for extra in names[1:]:
            tail = ("bin", "^", tail, ("var", extra), self.kind)
        tail = ("bin", "^", tail, self.value(self.declared(body) + names, 2, functions), self.kind)
        tail = use_every_local(body, tail, self.kind)
        return {"name": name, "params": params, "ret": self.kind, "body": body, "tail": tail}

    @staticmethod
    def declared(body: list[tuple]) -> list[str]:
        names = []
        for statement in body:
            if statement[0] == "let":
                names.append(statement[1])
            elif statement[0] == "while":
                names += Generator.declared(statement[2])
        return names


def reads(node: Any, seen: set[str]) -> None:
    """Collect every name read by an expression or statement tree."""
    if not isinstance(node, tuple):
        return
    if node[0] == "var":
        seen.add(node[1])
        return
    if node[0] == "compound":  # `x += e` reads x as well
        seen.add(node[1])
    for item in node[1:]:
        if isinstance(item, tuple):
            reads(item, seen)
        elif isinstance(item, list):
            for entry in item:
                reads(entry, seen)


def declared_names(body: list[tuple]) -> list[str]:
    names = []
    for statement in body:
        if statement[0] in ("let", "seq_loop"):
            names.append(statement[1])
        if statement[0] == "seq_loop":
            names += declared_names(statement[3])
        if statement[0] == "while":
            names += declared_names(statement[2])
    return names


def use_every_local(body: list[tuple], tail: tuple, kind: str) -> tuple:
    """Fold any unread local into the tail expression.

    An unused local is valid Core but produces C that fails the declared
    -Wall -Wextra -Werror profile (issue #27), which would drown real
    divergences in noise.
    """
    seen: set[str] = set()
    for statement in body:
        reads(statement, seen)
    reads(tail, seen)
    for name in declared_names(body):
        if name not in seen:
            tail = ("bin", "^", tail, ("var", name), kind)
    return tail


def collect_calls(node: Any, seen: set[str]) -> None:
    """Collect every function called anywhere in an expression or body."""
    if isinstance(node, list):
        for item in node:
            collect_calls(item, seen)
        return
    if not isinstance(node, tuple):
        return
    if node[0] == "call":
        seen.add(node[1])
    for item in node[1:]:
        collect_calls(item, seen)


def expand_loops(body: list[tuple], kind: str) -> list[tuple]:
    """Turn the generator's counted loop into a Core `while` with a counter."""
    expanded: list[tuple] = []
    for statement in body:
        if statement[0] == "seq_loop":
            _, counter, limit, inner = statement
            expanded.append(("let", counter, kind, ("lit", kind, 0), True))
            condition = ("compare", "<", ("var", counter), ("lit", kind, limit))
            expanded.append(("while", condition, expand_loops(inner, kind)))
        else:
            expanded.append(statement)
    return expanded


def generate_program(seed: int, index: int) -> dict[str, Any] | None:
    rng = random.Random((seed << 20) + index)
    kind = rng.choice(list(INT_TYPES))
    generator = Generator(rng, kind)
    functions: list[dict[str, Any]] = []
    for position in range(rng.randint(0, 2)):
        functions.append(generator.function(position, list(functions)))
    main_body = generator.statements([], functions, allow_loop=True, mutable_names=[])
    names = Generator.declared(main_body) + [s[1] for s in main_body if s[0] == "seq_loop"]
    main_tail = generator.value(names, 3, functions)
    called: set[str] = set()
    collect_calls(main_body, called)
    collect_calls(main_tail, called)
    for function in functions:
        if function["name"] not in called:
            arguments = [generator.literal() for _ in function["params"]]
            main_tail = ("bin", "^", main_tail, ("call", function["name"], arguments), kind)
    main_tail = use_every_local(main_body, main_tail, kind)
    program = {
        "id": f"fuzz_{seed}_{index:04d}",
        "kind": kind,
        "functions": [dict(item, body=expand_loops(item["body"], kind)) for item in functions],
        "main": {"ret": kind, "body": expand_loops(main_body, kind), "tail": main_tail},
    }
    return program


def expected_outcome(program: dict[str, Any]) -> dict[str, Any] | None:
    """Run the oracle. None means the program is unusable for a case."""
    functions = {item["name"]: item for item in program["functions"]}
    fuel = [MAX_LOOP_ITERATIONS]
    try:
        value = run_body(program["main"]["body"], program["main"]["tail"], {}, functions, fuel)
    except Trap as trap:
        return {"expected_exit": 70, "expected_stdout": "", "expected_stderr": f"ARGORIX_TRAP:{trap.code}"}
    except RuntimeError:
        return None
    except RecursionError:
        return None
    if isinstance(value, bool):
        return None  # the generator only builds integer-valued mains
    return {"expected_exit": 0, "expected_stdout": f"ARGORIX_RESULT:{value}", "expected_stderr": ""}


def build_corpus(seed: int, count: int, out: pathlib.Path) -> dict[str, Any]:
    out.mkdir(parents=True, exist_ok=True)
    for stale in out.glob("fuzz_*.argx"):
        stale.unlink()
    cases = []
    index = 0
    attempts = 0
    while len(cases) < count and attempts < count * 20:
        attempts += 1
        index += 1
        program = generate_program(seed, index)
        if program is None:
            continue
        outcome = expected_outcome(program)
        if outcome is None:
            continue
        source = render_program(program)
        (out / f"{program['id']}.argx").write_text(source, encoding="utf-8", newline="\n")
        cases.append({"id": program["id"], "file": f"{program['id']}.argx", **outcome})
    manifest = {
        "schema_version": 1,
        "core_version": "0.1",
        "generator": "conformance/core_c/generate.py",
        "oracle": "spec/core/evaluation.md and spec/core/types.md, evaluated independently of argorixc",
        "seed": seed,
        "attempts": attempts,
        "cases": cases,
    }
    (out / "cases.json").write_text(json.dumps(manifest, indent=1) + "\n", encoding="utf-8", newline="\n")
    return manifest


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--count", type=int, default=40)
    parser.add_argument("--out", type=pathlib.Path, required=True)
    args = parser.parse_args(argv)
    manifest = build_corpus(args.seed, args.count, args.out.resolve())
    traps = sum(1 for case in manifest["cases"] if case["expected_exit"] == 70)
    print(f"generated {len(manifest['cases'])} cases in {args.out} "
          f"(seed {args.seed}, {traps} expected traps, {manifest['attempts']} attempts)")
    return 0 if manifest["cases"] else 1


if __name__ == "__main__":
    sys.exit(main())
