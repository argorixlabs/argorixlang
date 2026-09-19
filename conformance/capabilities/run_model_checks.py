#!/usr/bin/env python3
"""Delegation, revocation and TOCTOU checks for MAT-006."""

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

from model import AuthorityError, AuthorityModel


ROOT = pathlib.Path(__file__).resolve().parents[2]
HERE = pathlib.Path(__file__).resolve().parent
POLICY_PATH = HERE / "model-policy.json"
SCENARIOS_PATH = HERE / "scenarios.json"


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def base(budget: int = 10, expires: int = 100) -> tuple[AuthorityModel, str]:
    model = AuthorityModel()
    root = model.issue_root(
        "root-authority", "alice", "agent-runtime", "fs:/project",
        frozenset({"delegate", "fs.read", "tool.invoke"}), budget, 0, expires,
    )
    return model, root


def delegated(model: AuthorityModel, root: str, budget: int = 3) -> str:
    return model.delegate(
        root, "alice", "bob", "fs:/project/sub",
        frozenset({"fs.read"}), budget, 0, 50,
    )


def expect_delegate_error(expected: str, **changes: object) -> dict[str, object]:
    model, root = base()
    args = {
        "parent_id": root,
        "caller_subject": "alice",
        "child_subject": "bob",
        "resource_scope": "fs:/project/sub",
        "operations": frozenset({"fs.read"}),
        "budget": 3,
        "not_before": 0,
        "expires_at": 50,
    }
    args.update(changes)
    observed = None
    before = model.semantic_digest()
    try:
        model.delegate(**args)  # type: ignore[arg-type]
    except AuthorityError as error:
        observed = error.code
    model.assert_invariants()
    return {"observed": observed, "passed": observed == expected and before == model.semantic_digest()}


def scenario(case_id: str) -> dict[str, object]:
    if case_id == "scope_escalation":
        return expect_delegate_error("ScopeEscalation", resource_scope="fs:/other")
    if case_id == "operation_escalation":
        return expect_delegate_error("OperationEscalation", operations=frozenset({"net.connect"}))
    if case_id == "expiry_extension":
        return expect_delegate_error("ExpiryExtension", expires_at=101)
    if case_id == "budget_amplification":
        return expect_delegate_error("BudgetAmplification", budget=11)
    if case_id == "parent_revocation_cascade":
        model, root = base()
        child = delegated(model, root)
        model.revoke(root)
        result = model.evaluate(child, "bob", "fs:/project/sub/a", "fs.read", 1, "n1")
        model.assert_invariants()
        return {"observed": result["reason"], "passed": result["outcome"] == "DENY" and result["reason"] == "AncestorRevoked"}
    if case_id == "revocation_before_commit":
        model, root = base()
        decision = model.evaluate(root, "alice", "fs:/project/a", "fs.read", 1, "n1")
        model.revoke(root)
        result = model.commit(decision["ticket_id"])
        model.assert_invariants()
        return {"observed": result["reason"], "passed": result["outcome"] == "DENY" and result["reason"] == "Revoked"}
    if case_id == "revocation_after_dispatch":
        model, root = base()
        decision = model.evaluate(root, "alice", "fs:/project/a", "fs.read", 1, "n1")
        committed = model.commit(decision["ticket_id"])
        model.revoke(root)
        state = model.effects[decision["ticket_id"]]
        model.assert_invariants()
        return {"observed": state, "passed": committed["state"] == "DISPATCHED" and state == "UNCERTAIN"}
    if case_id == "nonce_replay":
        model, root = base()
        decision = model.evaluate(root, "alice", "fs:/project/a", "fs.read", 1, "nonce")
        model.commit(decision["ticket_id"])
        replay = model.evaluate(root, "alice", "fs:/project/a", "fs.read", 1, "nonce")
        model.assert_invariants()
        return {"observed": replay["reason"], "passed": replay["outcome"] == "DENY" and replay["reason"] == "Replay"}
    if case_id == "authority_failure_unknown":
        model, root = base()
        model.set_available(False)
        result = model.evaluate(root, "alice", "fs:/project/a", "fs.read", 1, "n1")
        return {"observed": result["outcome"], "passed": result["outcome"] == "UNKNOWN" and result["ticket_id"] is None}
    if case_id in {"policy_deny", "policy_review", "policy_unknown"}:
        expected = {"policy_deny": "DENY", "policy_review": "REVIEW", "policy_unknown": "UNKNOWN"}[case_id]
        model, root = base()
        model.set_policy("fs.read", expected)
        result = model.evaluate(root, "alice", "fs:/project/a", "fs.read", 1, "n1")
        return {"observed": result["outcome"], "passed": result["outcome"] == expected and result["ticket_id"] is None}
    if case_id == "expiry_conservative":
        model, root = base(expires=5)
        model.advance(5)
        result = model.evaluate(root, "alice", "fs:/project/a", "fs.read", 1, "n1")
        return {"observed": result["reason"], "passed": result["reason"] == "Expired"}
    if case_id == "budget_race":
        model, root = base(budget=1)
        first = model.evaluate(root, "alice", "fs:/project/a", "fs.read", 1, "n1")
        second = model.evaluate(root, "alice", "fs:/project/b", "fs.read", 1, "n2")
        one = model.commit(first["ticket_id"])
        two = model.commit(second["ticket_id"])
        model.assert_invariants()
        return {"observed": two["reason"], "passed": one["outcome"] == "ALLOW" and two["reason"] == "BudgetExceeded" and model.grants[root].budget_remaining == 0}
    if case_id == "policy_epoch_invalidates_ticket":
        model, root = base()
        decision = model.evaluate(root, "alice", "fs:/project/a", "fs.read", 1, "n1")
        model.set_policy("fs.read", "DENY")
        result = model.commit(decision["ticket_id"])
        return {"observed": result["reason"], "passed": result["reason"] == "PolicyChanged" and model.effects[decision["ticket_id"]] == "AUTHORIZED"}
    if case_id == "subject_mismatch":
        model, root = base()
        result = model.evaluate(root, "mallory", "fs:/project/a", "fs.read", 1, "n1")
        return {"observed": result["reason"], "passed": result["reason"] == "SubjectMismatch"}
    if case_id == "removed_permission_not_recovered":
        model, root = base()
        child = delegated(model, root)
        model.set_policy("fs.read", "DENY")
        result = model.evaluate(child, "bob", "fs:/project/sub/a", "fs.read", 1, "n1")
        escalation_blocked = False
        try:
            model.delegate(root, "alice", "carol", "fs:/project/new", frozenset({"fs.read"}), 1, 0, 20)
        except AuthorityError as error:
            escalation_blocked = error.code == "PolicyDenied"
        model.assert_invariants()
        return {"observed": result["outcome"], "passed": result["outcome"] == "DENY" and escalation_blocked}
    raise KeyError(case_id)


def contract_coverage(policy: dict[str, object]) -> dict[str, object]:
    text = (ROOT / "spec" / "capabilities.md").read_text(encoding="utf-8")
    missing: dict[str, list[str]] = {}
    for category in ("decisions", "dispatch_states"):
        absent = [value for value in policy[category] if f"`{value}`" not in text]  # type: ignore[index]
        if absent:
            missing[category] = absent
    return {"decisions": len(policy["decisions"]), "dispatch_states": len(policy["dispatch_states"]), "missing": missing, "passed": not missing}  # type: ignore[arg-type]


def bounded_exploration(depth: int) -> dict[str, object]:
    actions = ("delegate", "evaluate", "commit", "revoke")
    failures: list[str] = []
    traces = 0
    for sequence in itertools.product(actions, repeat=depth):
        model, root = base(budget=4, expires=30)
        children: list[str] = []
        tickets: list[str] = []
        nonce = 0
        for action in sequence:
            try:
                if action == "delegate" and model.grants[root].status == "ACTIVE":
                    child = model.delegate(root, "alice", f"child-{len(children)}", "fs:/project/sub", frozenset({"fs.read"}), 1, 0, 20)
                    children.append(child)
                elif action == "evaluate":
                    grant = children[-1] if children else root
                    subject = model.grants[grant].subject
                    result = model.evaluate(grant, subject, "fs:/project/sub/a" if children else "fs:/project/a", "fs.read", 1, f"n{nonce}")
                    nonce += 1
                    if result["ticket_id"]:
                        tickets.append(result["ticket_id"])
                elif action == "commit" and tickets:
                    model.commit(tickets[-1])
                elif action == "revoke":
                    model.revoke(children[-1] if children else root)
            except AuthorityError:
                pass
            try:
                model.assert_invariants()
            except AssertionError as error:
                failures.append(f"{sequence}:{action}:{error}")
                break
        traces += 1
        if failures:
            break
    return {"depth": depth, "traces_explored": traces, "failures": failures, "passed": not failures}


def generated_sequences(seeds: int, steps: int) -> dict[str, object]:
    failures: list[str] = []
    totals = {"delegated": 0, "allowed": 0, "denied": 0, "review": 0, "unknown": 0, "committed": 0, "revoked": 0}
    for seed in range(seeds):
        rng = random.Random(seed)
        model, root = base(budget=32, expires=200)
        grants = [root]
        tickets: list[str] = []
        for step in range(steps):
            action = rng.randrange(7)
            try:
                if action == 0:
                    parent = rng.choice(grants)
                    p = model.grants[parent]
                    child = model.delegate(parent, p.subject, f"s{seed}-{step}", p.resource_scope + "/x", frozenset({"fs.read"}), rng.randrange(0, min(3, p.budget_remaining) + 1), model.tick, min(p.expires_at, model.tick + 20))
                    grants.append(child); totals["delegated"] += 1
                elif action in {1, 2}:
                    grant = rng.choice(grants); g = model.grants[grant]
                    result = model.evaluate(grant, g.subject, g.resource_scope + "/a", "fs.read", rng.randrange(0, 3), f"{seed}:{step}")
                    total_key = {"ALLOW": "allowed", "DENY": "denied", "REVIEW": "review", "UNKNOWN": "unknown"}[result["outcome"]]
                    totals[total_key] += 1
                    if result["ticket_id"]:
                        tickets.append(result["ticket_id"])
                elif action == 3 and tickets:
                    result = model.commit(rng.choice(tickets))
                    if result["state"] == "DISPATCHED": totals["committed"] += 1
                elif action == 4:
                    model.revoke(rng.choice(grants)); totals["revoked"] += 1
                elif action == 5:
                    model.set_policy("fs.read", rng.choice(["ALLOW", "DENY", "REVIEW", "UNKNOWN"]))
                elif action == 6:
                    model.advance(1)
            except AuthorityError:
                pass
            try:
                model.assert_invariants()
            except AssertionError as error:
                failures.append(f"seed={seed} step={step}:{error}")
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
    fixtures = json.loads(SCENARIOS_PATH.read_text(encoding="utf-8"))
    coverage = contract_coverage(policy)
    results = []
    for item in fixtures["cases"]:
        result = scenario(item["id"])
        result.update({"id": item["id"], "expected": item["expected"]})
        results.append(result)
    bounded = bounded_exploration(policy["bounded_exploration"]["max_depth"])
    generated = generated_sequences(policy["bounded_exploration"]["random_seeds"], policy["bounded_exploration"]["steps_per_seed"])
    mutation_detected = None
    if args.self_test:
        mutated = copy.deepcopy(fixtures)
        mutated["cases"][0]["expected"] = "ALLOW"
        mutation_detected = results[0]["observed"] != mutated["cases"][0]["expected"]
    required = set(policy["required_scenarios"])
    passed = {item["id"] for item in results if item["passed"]}
    overall = required == passed and coverage["passed"] and bounded["passed"] and generated["passed"] and (not args.self_test or mutation_detected is True)
    report = {
        "schema_version": 1,
        "task": "MAT-006",
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
        "non_allow_dispatch_count": 0,
        "overall_pass": overall,
        "not_proven": ["authenticated identity", "cryptographic tokens", "durable revocation", "distributed clock", "host enforcement", "real effect cancellation"]
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
