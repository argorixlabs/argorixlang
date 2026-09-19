#!/usr/bin/env python3
"""Bounded and generated checks for the MAT-004 reduced model."""

from __future__ import annotations

import argparse
import copy
import datetime as dt
import hashlib
import itertools
import json
import pathlib
import random
import sys
from typing import Callable

from model import BorrowToken, Capability, Handle, MemoryModel, ModelError


ROOT = pathlib.Path(__file__).resolve().parents[3]
HERE = pathlib.Path(__file__).resolve().parent
POLICY_PATH = HERE / "model-policy.json"
CASES_PATH = HERE / "counterexamples.json"


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def expect_error(model: MemoryModel, expected: str, operation: Callable[[], object]) -> dict[str, object]:
    before = model.snapshot()
    observed = None
    try:
        operation()
    except ModelError as error:
        observed = error.code
    after = model.snapshot()
    return {
        "expected": expected,
        "observed": observed,
        "error_matched": observed == expected,
        "failure_atomic": before == after,
        "passed": observed == expected and before == after,
    }


def scenario(case_id: str) -> dict[str, object]:
    model = MemoryModel(host_limit=128)
    arena = model.arena_create("alice", 16)
    handle = model.alloc("alice", arena, "u8", 4)
    if case_id == "use_after_free":
        model.free("alice", handle)
        return expect_error(model, "UseAfterFree", lambda: model.read(handle, 0))
    if case_id == "arena_use_after_release":
        model.arena_release("alice", arena)
        return expect_error(model, "ArenaReleased", lambda: model.read(handle, 0))
    if case_id == "out_of_bounds":
        return expect_error(model, "OutOfBounds", lambda: model.read(handle, 4))
    if case_id == "mutable_alias":
        model.borrow_read(handle, "reader")
        return expect_error(model, "BorrowConflict", lambda: model.borrow_write(handle, "writer"))
    if case_id == "oom_atomicity":
        return expect_error(model, "OutOfMemory", lambda: model.alloc("alice", arena, "i64", 2))
    if case_id == "type_mismatch":
        return expect_error(model, "TypeMismatch", lambda: model.write(handle, 0, "not-a-byte"))
    if case_id == "unauthorized_effect":
        return expect_error(
            model, "CapabilityMissing",
            lambda: model.authorize_effect({"fs.read"}, None, "alice", "fs.read", "pkg/source.argx")
        )
    if case_id == "effect_not_declared":
        capability = Capability("c1", "fs.read", "pkg/source.argx", "alice")
        return expect_error(
            model, "EffectNotDeclared",
            lambda: model.authorize_effect(set(), capability, "alice", "fs.read", "pkg/source.argx")
        )
    if case_id == "memory_error_before_effect":
        result = expect_error(model, "OutOfBounds", lambda: model.read(handle, 99))
        result["effect_events"] = len(model.effect_events)
        result["passed"] = result["passed"] and not model.effect_events
        return result
    raise KeyError(case_id)


def run_counterexamples(cases: dict[str, object]) -> list[dict[str, object]]:
    results = []
    for case in cases["cases"]:  # type: ignore[index]
        result = scenario(case["id"])
        result.update({"id": case["id"], "expected_from_fixture": case["expected_error"]})
        if result["observed"] != case["expected_error"]:
            result["passed"] = False
        results.append(result)
    return results


def bounded_exploration(max_depth: int) -> dict[str, object]:
    actions = ("read0", "read4", "write", "borrow_read", "borrow_write", "free")
    explored = 0
    invariant_failures: list[str] = []
    observed_errors: set[str] = set()
    for sequence in itertools.product(actions, repeat=max_depth):
        model = MemoryModel(host_limit=64)
        arena = model.arena_create("alice", 16)
        handle = model.alloc("alice", arena, "u8", 4)
        tokens: list[BorrowToken] = []
        for action in sequence:
            before = model.snapshot()
            try:
                if action == "read0":
                    model.read(handle, 0)
                elif action == "read4":
                    model.read(handle, 4)
                elif action == "write":
                    model.write(handle, 0, 7)
                elif action == "borrow_read":
                    tokens.append(model.borrow_read(handle, "reader"))
                elif action == "borrow_write":
                    tokens.append(model.borrow_write(handle, "writer"))
                elif action == "free":
                    model.free("alice", handle)
            except ModelError as error:
                observed_errors.add(error.code)
                if model.snapshot() != before:
                    invariant_failures.append(f"failure_atomicity:{sequence}:{action}:{error.code}")
                    break
            try:
                model.assert_invariants()
            except AssertionError as error:
                invariant_failures.append(f"{error}:{sequence}:{action}")
                break
        explored += 1
    return {
        "depth": max_depth,
        "traces_explored": explored,
        "invariant_failures": invariant_failures[:20],
        "observed_errors": sorted(observed_errors),
        "passed": not invariant_failures,
    }


def generated_sequences(seed_count: int, steps: int) -> dict[str, object]:
    failures: list[str] = []
    observed_errors: set[str] = set()
    successful_operations = 0
    for seed in range(seed_count):
        rng = random.Random(seed)
        model = MemoryModel(host_limit=256, max_arenas=4)
        arenas: list[int] = []
        handles: list[Handle] = []
        tokens: list[BorrowToken] = []
        for step in range(steps):
            operation = rng.randrange(11)
            before = model.snapshot()
            try:
                if operation == 0 or not arenas:
                    arenas.append(model.arena_create("alice", rng.choice([0, 8, 16, 32, 300])))
                elif operation == 1:
                    arena = rng.choice(arenas)
                    owner = model.arenas[arena].owner
                    handles.append(model.alloc(owner, arena, rng.choice(list(("u8", "i64", "bool"))), rng.randrange(0, 12)))
                elif operation == 2 and handles:
                    handle = rng.choice(handles)
                    handles.append(model.slice(handle, rng.randrange(0, handle.length + 2), rng.randrange(0, handle.length + 2), rng.choice(["read", "read_write"])))
                elif operation == 3 and handles:
                    model.read(rng.choice(handles), rng.randrange(0, 8))
                elif operation == 4 and handles:
                    model.write(rng.choice(handles), rng.randrange(0, 8), rng.choice([0, 1, 255, 256, "bad", True]))
                elif operation == 5 and handles:
                    tokens.append(model.borrow_read(rng.choice(handles), "reader"))
                elif operation == 6 and handles:
                    tokens.append(model.borrow_write(rng.choice(handles), "writer"))
                elif operation == 7 and tokens:
                    token = rng.choice(tokens)
                    model.end_borrow(token)
                elif operation == 8 and handles:
                    handle = rng.choice(handles)
                    arena = model.arenas.get(handle.arena_id)
                    model.free(arena.owner if arena else "alice", handle)
                elif operation == 9 and arenas:
                    arena_id = rng.choice(arenas)
                    arena = model.arenas[arena_id]
                    model.transfer(arena.owner, "bob" if arena.owner == "alice" else "alice", arena_id)
                elif operation == 10 and arenas:
                    arena_id = rng.choice(arenas)
                    arena = model.arenas[arena_id]
                    model.arena_release(arena.owner, arena_id)
                successful_operations += 1
            except ModelError as error:
                observed_errors.add(error.code)
                if model.snapshot() != before:
                    failures.append(f"seed={seed} step={step} atomicity={error.code}")
            try:
                model.assert_invariants()
            except AssertionError as error:
                failures.append(f"seed={seed} step={step} invariant={error}")
            if failures:
                break
        if failures:
            break
    return {
        "seeds": seed_count,
        "steps_per_seed": steps,
        "attempted_steps": seed_count * steps if not failures else None,
        "successful_operations": successful_operations,
        "observed_errors": sorted(observed_errors),
        "failures": failures,
        "passed": not failures,
    }


def weak_generation_probe() -> dict[str, object]:
    model = MemoryModel()
    arena = model.arena_create("alice", 8)
    handle = model.alloc("alice", arena, "u8", 1)
    model.write(handle, 0, 42)
    model.free("alice", handle)
    safe_rejected = None
    try:
        model.read(handle, 0)
    except ModelError as error:
        safe_rejected = error.code
    # Deliberately unsafe countermodel: ignores live/generation and reads tombstone bytes.
    weak_accepted = model.arenas[arena].allocations[handle.slot].values[0] == 42
    return {
        "safe_model_error": safe_rejected,
        "weak_model_accepted_stale_handle": weak_accepted,
        "rejection_gate_exercised": safe_rejected == "UseAfterFree" and weak_accepted,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    policy = json.loads(POLICY_PATH.read_text(encoding="utf-8"))
    cases = json.loads(CASES_PATH.read_text(encoding="utf-8"))
    counterexamples = run_counterexamples(cases)
    bounded = bounded_exploration(policy["bounded_exploration"]["max_depth"])
    generated = generated_sequences(
        policy["bounded_exploration"]["random_seeds"],
        policy["bounded_exploration"]["steps_per_seed"],
    )
    weak_probe = weak_generation_probe()
    mutation_detected = None
    if args.self_test:
        mutated = copy.deepcopy(cases)
        mutated["cases"][0]["expected_error"] = "OutOfBounds"
        mutation_detected = not all(item["passed"] for item in run_counterexamples(mutated))
    required = set(policy["required_counterexamples"])
    observed = {item["id"] for item in counterexamples if item["passed"]}
    overall = (
        required == observed
        and bounded["passed"]
        and generated["passed"]
        and weak_probe["rejection_gate_exercised"]
        and (not args.self_test or mutation_detected is True)
    )
    report = {
        "schema_version": 1,
        "task": "MAT-004",
        "generated_at_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "baseline_commit": policy["baseline_commit"],
        "model_policy_sha256": sha256(POLICY_PATH),
        "counterexamples_sha256": sha256(CASES_PATH),
        "mechanical_scope": policy["mechanical_scope"],
        "counterexamples": counterexamples,
        "bounded_exploration": bounded,
        "generated_sequences": generated,
        "weak_generation_probe": weak_probe,
        "mutation_self_test": {"requested": args.self_test, "detected": mutation_detected},
        "arena_decision": policy["decision"] if overall else "REJECTED_BY_MODEL_GATE",
        "overall_pass": overall,
        "not_proven": ["compiler", "backend", "concurrency", "ABI layout", "performance", "host sandboxing"]
    }
    rendered = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    if args.output:
        output = pathlib.Path(args.output)
        if not output.is_absolute():
            output = ROOT / output
        output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0 if overall else 1


if __name__ == "__main__":
    sys.exit(main())
