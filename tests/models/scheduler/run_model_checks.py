#!/usr/bin/env python3
"""Interleaving and replay checks for MAT-005's reduced scheduler model."""

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

from model import Envelope, SchedulerError, SchedulerModel


ROOT = pathlib.Path(__file__).resolve().parents[3]
HERE = pathlib.Path(__file__).resolve().parent
POLICY_PATH = HERE / "model-policy.json"
SCENARIOS_PATH = HERE / "scenarios.json"


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def contract_coverage(policy: dict[str, object]) -> dict[str, object]:
    concurrency = (ROOT / "spec" / "concurrency.md").read_text(encoding="utf-8")
    delivery = (ROOT / "spec" / "message-delivery.md").read_text(encoding="utf-8")
    combined = concurrency + "\n" + delivery
    missing: dict[str, list[str]] = {}
    for category in ("message_states", "mailbox_states", "effect_states"):
        values = policy[category]  # type: ignore[index]
        absent = [value for value in values if f"`{value}`" not in combined]  # type: ignore[union-attr]
        if absent:
            missing[category] = absent
    return {
        "message_states": len(policy["message_states"]),  # type: ignore[arg-type]
        "mailbox_states": len(policy["mailbox_states"]),  # type: ignore[arg-type]
        "effect_states": len(policy["effect_states"]),  # type: ignore[arg-type]
        "missing": missing,
        "passed": not missing,
    }


def remote_envelope(sequence: int, payload: bytes = b"x", authority: frozenset[str] = frozenset()) -> Envelope:
    return Envelope(
        f"run-1:remote:stream:{sequence}", "run-1", "remote", "worker", "stream", sequence,
        hashlib.sha256(payload).hexdigest(), len(payload), authority, None, 100
    )


def base(capacity: int = 4, byte_limit: int = 64) -> SchedulerModel:
    model = SchedulerModel()
    model.register("worker", capacity, byte_limit)
    return model


def run_script() -> tuple[SchedulerModel, list[str]]:
    model = base()
    ids: list[str] = []
    for payload in (b"a", b"b"):
        message_id = model.send("alice", "worker", "s", payload, frozenset(), 20)
        assert message_id
        ids.append(message_id)
    first = model.lease("worker")
    assert first == ids[0]
    model.ack(first)
    second = model.lease("worker")
    assert second == ids[1]
    model.fail(second, retryable=True)
    second_retry = model.lease("worker")
    assert second_retry == ids[1]
    model.ack(second_retry)
    model.assert_invariants()
    return model, ids


def scenario(case_id: str) -> dict[str, object]:
    if case_id == "slow_receiver_backpressure":
        model = base(1, 2)
        first = model.send("a", "worker", "s", b"a", frozenset(), 10)
        second = model.send("a", "worker", "s", b"b", frozenset(), 10)
        reason = model.events[-1].get("reason")
        passed = first is not None and second is None and reason == "Backpressure"
        model.assert_invariants()
        return {"observed": reason, "passed": passed, "accepted_messages": model.mailboxes["worker"].used_messages}
    if case_id == "duplicate_dedup":
        model = base()
        env = remote_envelope(1)
        model.remote_receive(env)
        model.remote_receive(env)
        mid = model.lease("worker")
        assert mid
        model.ack(mid)
        model.remote_receive(env)
        acks = sum(e["kind"] == "MESSAGE_ACKED" for e in model.events)
        drops = sum(e["kind"] == "DUPLICATE_DROPPED" for e in model.events)
        model.assert_invariants()
        return {"observed": "single_ack", "passed": acks == 1 and drops == 2, "acks": acks, "duplicate_drops": drops}
    if case_id == "reorder_fifo":
        model = base()
        model.remote_receive(remote_envelope(2, b"two"))
        model.remote_receive(remote_envelope(1, b"one"))
        sequences = model.accepted_sequences[("remote", "worker", "stream")]
        model.assert_invariants()
        return {"observed": str(sequences), "passed": sequences == [1, 2]}
    if case_id == "cancel_before_dispatch":
        model = base()
        mid = model.send("a", "worker", "s", b"x", frozenset({"tool.invoke"}), 10)
        assert mid
        observed = model.cancel(mid)
        model.assert_invariants()
        return {"observed": observed, "passed": observed == "CANCELLED" and mid not in model.effects}
    if case_id == "cancel_after_dispatch_uncertain":
        model = base()
        mid = model.send("a", "worker", "s", b"x", frozenset({"tool.invoke"}), 10)
        assert mid and model.lease("worker") == mid
        model.dispatch_effect(mid, "tool.invoke")
        observed = model.cancel(mid)
        model.assert_invariants()
        return {"observed": observed, "effect": model.effects[mid], "passed": observed == "UNCERTAIN" and model.effects[mid] == "UNCERTAIN"}
    if case_id == "deadline_expiry":
        model = base()
        mid = model.send("a", "worker", "s", b"x", frozenset(), 1)
        assert mid
        model.advance(1)
        leased = model.lease("worker")
        model.assert_invariants()
        return {"observed": model.messages[mid].state, "passed": leased is None and model.messages[mid].state == "EXPIRED"}
    if case_id == "authority_escalation_rejected":
        model = base()
        mid = model.send("a", "worker", "s", b"x", frozenset({"tool.invoke"}), 10, current_authority=frozenset())
        reason = model.events[-1].get("reason")
        model.assert_invariants()
        return {"observed": reason, "passed": mid is None and reason == "AuthorityEscalation"}
    if case_id == "crash_redelivery":
        model = base()
        mid = model.send("a", "worker", "s", b"x", frozenset(), 10)
        assert mid and model.lease("worker") == mid
        model.crash("worker")
        assert model.lease("worker") == mid
        model.ack(mid)
        acks = sum(e["kind"] == "MESSAGE_ACKED" for e in model.events)
        model.assert_invariants()
        return {"observed": f"attempts={model.messages[mid].attempts},single_ack", "passed": model.messages[mid].attempts == 2 and acks == 1}
    if case_id == "deadlock_cycle":
        model = base()
        first = model.await_actor("a", "b")
        second = model.await_actor("b", "a")
        return {"observed": second, "passed": first == "Waiting" and second == "DeadlockDetected"}
    if case_id == "drain_close":
        model = base()
        mid = model.send("a", "worker", "s", b"x", frozenset(), 10)
        assert mid
        model.drain("worker")
        rejected = model.send("a", "worker", "s", b"y", frozenset(), 10)
        leased = model.lease("worker")
        assert leased == mid
        model.ack(mid)
        model.assert_invariants()
        return {"observed": model.mailboxes["worker"].state, "passed": rejected is None and model.mailboxes["worker"].state == "CLOSED"}
    if case_id == "message_id_collision":
        model = base()
        first = remote_envelope(1, b"one")
        collision = remote_envelope(1, b"different")
        model.remote_receive(first)
        observed = model.remote_receive(collision)
        model.assert_invariants()
        return {"observed": observed, "passed": observed == "MessageIdCollision"}
    if case_id == "replay_same_schedule":
        first, _ = run_script()
        second, _ = run_script()
        return {"observed": "same_semantic_digest", "passed": first.semantic_digest() == second.semantic_digest(), "digest": first.semantic_digest()}
    raise KeyError(case_id)


def bounded_exploration(depth: int) -> dict[str, object]:
    actions = ("lease", "ack", "cancel", "crash")
    traces = 0
    failures: list[str] = []
    observed_errors: set[str] = set()
    for sequence in itertools.product(actions, repeat=depth):
        model = base(2, 8)
        ids = [
            model.send("a", "worker", "s", b"a", frozenset(), 20),
            model.send("a", "worker", "s", b"b", frozenset(), 20),
        ]
        active: str | None = None
        for action in sequence:
            before = copy.deepcopy(model)
            try:
                if action == "lease":
                    leased = model.lease("worker")
                    if leased:
                        active = leased
                elif action == "ack" and active:
                    model.ack(active)
                    active = None
                elif action == "cancel":
                    target = active or next((mid for mid in ids if mid and model.messages[mid].state not in {"ACKED", "CANCELLED", "EXPIRED", "DEAD_LETTER"}), None)
                    if target:
                        model.cancel(target)
                elif action == "crash":
                    model.crash("worker")
                    active = None
            except SchedulerError as error:
                observed_errors.add(error.code)
                if model.semantic_digest() != before.semantic_digest():
                    failures.append(f"error_mutated:{sequence}:{action}:{error.code}")
            try:
                model.assert_invariants()
            except AssertionError as error:
                failures.append(f"invariant:{sequence}:{action}:{error}")
            if failures:
                break
        traces += 1
        if failures:
            break
    return {"depth": depth, "traces_explored": traces, "failures": failures, "observed_errors": sorted(observed_errors), "passed": not failures}


def generated_sequences(seeds: int, steps: int) -> dict[str, object]:
    failures: list[str] = []
    totals = {"accepted": 0, "rejected": 0, "leased": 0, "terminal": 0}
    for seed in range(seeds):
        rng = random.Random(seed)
        model = base(3, 12)
        active: list[str] = []
        known: list[str] = []
        for step in range(steps):
            action = rng.randrange(8)
            try:
                if action <= 2:
                    mid = model.send("a", "worker", "s", bytes([rng.randrange(256)]) * rng.randrange(0, 7), frozenset({"tool.invoke"}), rng.randrange(model.tick + 1, model.tick + 8))
                    if mid:
                        known.append(mid); totals["accepted"] += 1
                    else:
                        totals["rejected"] += 1
                elif action == 3:
                    mid = model.lease("worker")
                    if mid:
                        active.append(mid); totals["leased"] += 1
                elif action == 4 and active:
                    mid = rng.choice(active)
                    model.ack(mid); active.remove(mid); totals["terminal"] += 1
                elif action == 5 and known:
                    mid = rng.choice(known)
                    model.cancel(mid)
                    if model.messages[mid].state in {"CANCELLED", "ACKED"}:
                        totals["terminal"] += 1
                    if mid in active and model.messages[mid].state == "CANCELLED":
                        active.remove(mid)
                elif action == 6:
                    model.crash("worker"); active.clear()
                elif action == 7:
                    model.advance(1)
            except SchedulerError:
                pass
            try:
                model.assert_invariants()
            except AssertionError as error:
                failures.append(f"seed={seed} step={step} {error}")
                break
        if failures:
            break
    return {"seeds": seeds, "steps_per_seed": steps, "attempted_steps": seeds * steps if not failures else None, "totals": totals, "failures": failures, "passed": not failures}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    policy = json.loads(POLICY_PATH.read_text(encoding="utf-8"))
    fixture = json.loads(SCENARIOS_PATH.read_text(encoding="utf-8"))
    coverage = contract_coverage(policy)
    results = []
    for item in fixture["cases"]:
        result = scenario(item["id"])
        result["id"] = item["id"]
        result["expected"] = item["expected"]
        results.append(result)
    bounded = bounded_exploration(policy["bounded_exploration"]["max_depth"])
    generated = generated_sequences(policy["bounded_exploration"]["random_seeds"], policy["bounded_exploration"]["steps_per_seed"])
    mutation_detected = None
    if args.self_test:
        mutated = copy.deepcopy(fixture)
        mutated["cases"][0]["expected"] = "SilentDrop"
        original = next(result for result in results if result["id"] == mutated["cases"][0]["id"])
        mutation_detected = original["observed"] != mutated["cases"][0]["expected"]
    required = set(policy["required_scenarios"])
    passed = {item["id"] for item in results if item["passed"]}
    overall = required == passed and coverage["passed"] and bounded["passed"] and generated["passed"] and (not args.self_test or mutation_detected is True)
    report = {
        "schema_version": 1,
        "task": "MAT-005",
        "generated_at_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "baseline_commit": policy["baseline_commit"],
        "policy_sha256": sha256(POLICY_PATH),
        "scenarios_sha256": sha256(SCENARIOS_PATH),
        "mechanical_scope": policy["mechanical_scope"],
        "contract_coverage": coverage,
        "scenario_results": results,
        "bounded_exploration": bounded,
        "generated_sequences": generated,
        "mutation_self_test": {"requested": args.self_test, "detected": mutation_detected},
        "exactly_once_external_claim": False,
        "overall_pass": overall,
        "not_proven": ["threads", "network", "persistence", "authenticated transport", "external effects", "production fairness"]
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
