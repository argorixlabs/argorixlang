"""Reduced authority model for MAT-006. Not a production authorization service."""

from __future__ import annotations

from dataclasses import asdict, dataclass, field
import hashlib
import json
from pathlib import PurePosixPath
from typing import Any


class AuthorityError(Exception):
    def __init__(self, code: str):
        super().__init__(code)
        self.code = code


@dataclass
class Grant:
    grant_id: str
    issuer: str
    subject: str
    audience: str
    resource_scope: str
    operations: frozenset[str]
    budget_total: int
    budget_remaining: int
    budget_spent: int
    not_before: int
    expires_at: int
    policy_version_issued: int
    revocation_epoch_issued: int
    delegable: bool
    depth_remaining: int
    parent_id: str | None
    status: str = "ACTIVE"
    children: list[str] = field(default_factory=list)


@dataclass
class Ticket:
    ticket_id: str
    request_digest: str
    grant_id: str
    subject: str
    resource: str
    operation: str
    cost: int
    nonce: str
    issued_tick: int
    expires_tick: int
    policy_version: int
    revocation_epoch: int
    committed: bool = False


class AuthorityModel:
    def __init__(self, ticket_ttl: int = 2):
        self.tick = 0
        self.ticket_ttl = ticket_ttl
        self.policy_version = 1
        self.revocation_epoch = 1
        self.available = True
        self.policy: dict[str, str] = {}
        self.grants: dict[str, Grant] = {}
        self.tickets: dict[str, Ticket] = {}
        self.nonce_digest: dict[str, str] = {}
        self.nonce_ticket: dict[str, str] = {}
        self.committed_nonces: set[str] = set()
        self.effects: dict[str, str] = {}
        self.events: list[dict[str, Any]] = []
        self.next_grant = 1
        self.next_ticket = 1

    def _event(self, kind: str, **fields: Any) -> None:
        self.events.append({"tick": self.tick, "kind": kind, **fields})

    @staticmethod
    def _canonical_scope(scope: str) -> tuple[str, str]:
        if ":" not in scope:
            raise AuthorityError("InvalidScope")
        kind, raw = scope.split(":", 1)
        if not kind or not raw.startswith("/") or ".." in PurePosixPath(raw).parts:
            raise AuthorityError("InvalidScope")
        normalized = "/" + "/".join(part for part in PurePosixPath(raw).parts if part != "/")
        return kind, normalized.rstrip("/") or "/"

    @classmethod
    def scope_contains(cls, parent: str, child: str) -> bool:
        parent_kind, parent_path = cls._canonical_scope(parent)
        child_kind, child_path = cls._canonical_scope(child)
        if parent_kind != child_kind:
            return False
        return child_path == parent_path or child_path.startswith(parent_path.rstrip("/") + "/")

    def issue_root(
        self, issuer: str, subject: str, audience: str, resource_scope: str,
        operations: frozenset[str], budget: int, not_before: int,
        expires_at: int, delegable: bool = True, depth: int = 3,
    ) -> str:
        self._canonical_scope(resource_scope)
        if not issuer or not subject or not audience or not operations or budget < 0 or expires_at <= not_before:
            raise AuthorityError("InvalidGrant")
        grant_id = f"grant-{self.next_grant}"
        self.next_grant += 1
        self.grants[grant_id] = Grant(
            grant_id, issuer, subject, audience, resource_scope, operations,
            budget, budget, 0, not_before, expires_at, self.policy_version,
            self.revocation_epoch, delegable, depth, None,
        )
        self._event("GRANT_ISSUED", grant_id=grant_id, subject=subject)
        return grant_id

    def delegate(
        self, parent_id: str, caller_subject: str, child_subject: str,
        resource_scope: str, operations: frozenset[str], budget: int,
        not_before: int, expires_at: int, audience: str | None = None,
        delegable: bool = False,
    ) -> str:
        parent = self._active_grant(parent_id, caller_subject)
        if not parent.delegable or parent.depth_remaining <= 0 or "delegate" not in parent.operations:
            raise AuthorityError("DelegationDenied")
        if not operations.issubset(parent.operations):
            raise AuthorityError("OperationEscalation")
        if any(self.policy.get(operation, "ALLOW") != "ALLOW" for operation in operations):
            raise AuthorityError("PolicyDenied")
        if not self.scope_contains(parent.resource_scope, resource_scope):
            raise AuthorityError("ScopeEscalation")
        target_audience = audience or parent.audience
        if target_audience != parent.audience:
            raise AuthorityError("AudienceEscalation")
        if not_before < parent.not_before:
            raise AuthorityError("NotBeforeExtension")
        if expires_at > parent.expires_at:
            raise AuthorityError("ExpiryExtension")
        if budget < 0 or budget > parent.budget_remaining:
            raise AuthorityError("BudgetAmplification")
        grant_id = f"grant-{self.next_grant}"
        child = Grant(
            grant_id, parent.subject, child_subject, target_audience, resource_scope,
            operations, budget, budget, 0, not_before, expires_at,
            self.policy_version, self.revocation_epoch,
            delegable and parent.delegable, parent.depth_remaining - 1, parent_id,
        )
        parent.budget_remaining -= budget
        parent.children.append(grant_id)
        self.next_grant += 1
        self.grants[grant_id] = child
        self._event("GRANT_DELEGATED", grant_id=grant_id, parent_id=parent_id, subject=child_subject)
        return grant_id

    def _lineage(self, grant: Grant) -> list[Grant]:
        lineage = [grant]
        current = grant
        seen = {grant.grant_id}
        while current.parent_id is not None:
            if current.parent_id in seen or current.parent_id not in self.grants:
                raise AuthorityError("InvalidLineage")
            current = self.grants[current.parent_id]
            lineage.append(current)
            seen.add(current.grant_id)
        return lineage

    def _active_grant(self, grant_id: str, subject: str | None = None) -> Grant:
        grant = self.grants.get(grant_id)
        if grant is None:
            raise AuthorityError("GrantMissing")
        if subject is not None and grant.subject != subject:
            raise AuthorityError("SubjectMismatch")
        lineage = self._lineage(grant)
        if grant.status == "REVOKED":
            raise AuthorityError("Revoked")
        if any(parent.status == "REVOKED" for parent in lineage[1:]):
            raise AuthorityError("AncestorRevoked")
        if self.tick < grant.not_before:
            raise AuthorityError("NotYetValid")
        if self.tick >= grant.expires_at:
            raise AuthorityError("Expired")
        return grant

    @staticmethod
    def _request_digest(grant_id: str, subject: str, resource: str, operation: str, cost: int, nonce: str) -> str:
        payload = [grant_id, subject, resource, operation, cost, nonce]
        return hashlib.sha256(json.dumps(payload, separators=(",", ":")).encode()).hexdigest()

    def evaluate(self, grant_id: str, subject: str, resource: str, operation: str, cost: int, nonce: str) -> dict[str, Any]:
        digest = self._request_digest(grant_id, subject, resource, operation, cost, nonce)
        previous = self.nonce_digest.get(nonce)
        if previous is not None and previous != digest:
            return self._decision("DENY", "ReplayCollision", grant_id, nonce)
        if nonce in self.committed_nonces:
            return self._decision("DENY", "Replay", grant_id, nonce)
        if previous == digest and nonce in self.nonce_ticket:
            ticket = self.tickets[self.nonce_ticket[nonce]]
            return {"outcome": "ALLOW", "reason": "CachedSameRequest", "ticket_id": ticket.ticket_id}
        self.nonce_digest[nonce] = digest
        if not self.available:
            return self._decision("UNKNOWN", "AuthorityUnavailable", grant_id, nonce)
        try:
            grant = self._active_grant(grant_id, subject)
        except AuthorityError as error:
            return self._decision("DENY", error.code, grant_id, nonce)
        if operation not in grant.operations:
            return self._decision("DENY", "OperationDenied", grant_id, nonce)
        try:
            in_scope = self.scope_contains(grant.resource_scope, resource)
        except AuthorityError:
            in_scope = False
        if not in_scope:
            return self._decision("DENY", "ScopeDenied", grant_id, nonce)
        if cost < 0 or cost > grant.budget_remaining:
            return self._decision("DENY", "BudgetExceeded", grant_id, nonce)
        policy = self.policy.get(operation, "ALLOW")
        if policy == "DENY":
            return self._decision("DENY", "PolicyDenied", grant_id, nonce)
        if policy == "REVIEW":
            return self._decision("REVIEW", "PolicyReview", grant_id, nonce)
        if policy != "ALLOW":
            return self._decision("UNKNOWN", "PolicyUnknown", grant_id, nonce)
        ticket_id = f"ticket-{self.next_ticket}"
        self.next_ticket += 1
        ticket = Ticket(
            ticket_id, digest, grant_id, subject, resource, operation, cost, nonce,
            self.tick, min(self.tick + self.ticket_ttl, grant.expires_at),
            self.policy_version, self.revocation_epoch,
        )
        self.tickets[ticket_id] = ticket
        self.nonce_ticket[nonce] = ticket_id
        self.effects[ticket_id] = "AUTHORIZED"
        self._event("AUTHORIZATION_DECIDED", outcome="ALLOW", ticket_id=ticket_id, grant_id=grant_id)
        return {"outcome": "ALLOW", "reason": "Authorized", "ticket_id": ticket_id}

    def _decision(self, outcome: str, reason: str, grant_id: str, nonce: str) -> dict[str, Any]:
        self._event("AUTHORIZATION_DECIDED", outcome=outcome, reason=reason, grant_id=grant_id, nonce=nonce)
        return {"outcome": outcome, "reason": reason, "ticket_id": None}

    def commit(self, ticket_id: str) -> dict[str, Any]:
        ticket = self.tickets.get(ticket_id)
        if ticket is None:
            return {"outcome": "DENY", "reason": "TicketMissing", "state": "DENIED"}
        if ticket.committed or ticket.nonce in self.committed_nonces:
            return {"outcome": "DENY", "reason": "Replay", "state": self.effects.get(ticket_id)}
        if not self.available:
            return {"outcome": "UNKNOWN", "reason": "AuthorityUnavailable", "state": "AUTHORIZED"}
        try:
            grant = self._active_grant(ticket.grant_id, ticket.subject)
        except AuthorityError as error:
            return {"outcome": "DENY", "reason": error.code, "state": "AUTHORIZED"}
        if self.tick >= ticket.expires_tick:
            return {"outcome": "DENY", "reason": "TicketExpired", "state": "AUTHORIZED"}
        if ticket.policy_version != self.policy_version:
            return {"outcome": "DENY", "reason": "PolicyChanged", "state": "AUTHORIZED"}
        if ticket.revocation_epoch != self.revocation_epoch:
            return {"outcome": "DENY", "reason": "RevocationChanged", "state": "AUTHORIZED"}
        if self.policy.get(ticket.operation, "ALLOW") != "ALLOW":
            return {"outcome": "DENY", "reason": "PolicyDenied", "state": "AUTHORIZED"}
        if ticket.operation not in grant.operations or not self.scope_contains(grant.resource_scope, ticket.resource):
            return {"outcome": "DENY", "reason": "AuthorityChanged", "state": "AUTHORIZED"}
        if ticket.cost > grant.budget_remaining:
            return {"outcome": "DENY", "reason": "BudgetExceeded", "state": "AUTHORIZED"}
        grant.budget_remaining -= ticket.cost
        grant.budget_spent += ticket.cost
        ticket.committed = True
        self.committed_nonces.add(ticket.nonce)
        self.effects[ticket_id] = "DISPATCHED"
        self._event("EFFECT_DISPATCHED", ticket_id=ticket_id, grant_id=grant.grant_id, cost=ticket.cost)
        return {"outcome": "ALLOW", "reason": "Committed", "state": "DISPATCHED"}

    def revoke(self, grant_id: str) -> None:
        grant = self.grants[grant_id]
        if grant.status != "REVOKED":
            grant.status = "REVOKED"
            self.revocation_epoch += 1
            self._event("GRANT_REVOKED", grant_id=grant_id, epoch=self.revocation_epoch)
        descendants = {item.grant_id for item in self.grants.values() if grant_id in {g.grant_id for g in self._lineage(item)[1:]}}
        affected = descendants | {grant_id}
        for ticket_id, ticket in self.tickets.items():
            if ticket.grant_id in affected and self.effects.get(ticket_id) == "DISPATCHED":
                self.effects[ticket_id] = "UNCERTAIN"
                self._event("EFFECT_UNCERTAIN", ticket_id=ticket_id, reason="RevokedAfterDispatch")

    def set_policy(self, operation: str, outcome: str) -> None:
        if outcome not in {"ALLOW", "DENY", "REVIEW", "UNKNOWN"}:
            raise AuthorityError("InvalidPolicyOutcome")
        self.policy[operation] = outcome
        self.policy_version += 1
        self._event("POLICY_CHANGED", operation=operation, outcome=outcome, version=self.policy_version)

    def set_available(self, available: bool) -> None:
        self.available = available
        self._event("AUTHORITY_AVAILABILITY", available=available)

    def advance(self, ticks: int = 1) -> None:
        if ticks < 0:
            raise AuthorityError("InvalidTick")
        self.tick += ticks
        self._event("TICK_ADVANCED", ticks=ticks)

    def assert_invariants(self) -> None:
        for grant in self.grants.values():
            if grant.parent_id:
                parent = self.grants[grant.parent_id]
                if not grant.operations.issubset(parent.operations):
                    raise AssertionError("delegation_attenuates_operations")
                if not self.scope_contains(parent.resource_scope, grant.resource_scope):
                    raise AssertionError("delegation_attenuates_scope")
                if grant.expires_at > parent.expires_at or grant.not_before < parent.not_before:
                    raise AssertionError("delegation_attenuates_time")
                if grant.depth_remaining >= parent.depth_remaining:
                    raise AssertionError("delegation_depth")
            children_total = sum(self.grants[child].budget_total for child in grant.children)
            if grant.budget_remaining + grant.budget_spent + children_total != grant.budget_total:
                raise AssertionError("budget_conservation")
            if grant.budget_remaining < 0 or grant.budget_spent < 0:
                raise AssertionError("budget_nonnegative")
        for ticket_id, state in self.effects.items():
            if state == "DISPATCHED" and not self.tickets[ticket_id].committed:
                raise AssertionError("dispatch_without_commit")
        if len(self.committed_nonces) != len(set(self.committed_nonces)):
            raise AssertionError("single_use_commit")

    def semantic_digest(self) -> str:
        payload = {
            "tick": self.tick,
            "policy_version": self.policy_version,
            "revocation_epoch": self.revocation_epoch,
            "grants": {key: asdict(value) for key, value in sorted(self.grants.items())},
            "tickets": {key: asdict(value) for key, value in sorted(self.tickets.items())},
            "effects": self.effects,
            "events": self.events,
        }
        return hashlib.sha256(json.dumps(payload, sort_keys=True, default=list).encode()).hexdigest()
