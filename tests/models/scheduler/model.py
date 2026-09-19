"""Reduced deterministic scheduler model for MAT-005; not a production runtime."""

from __future__ import annotations

from collections import deque
from dataclasses import asdict, dataclass, field
import hashlib
import json
from typing import Any


class SchedulerError(Exception):
    def __init__(self, code: str):
        super().__init__(code)
        self.code = code


TERMINAL = {"ACKED", "REJECTED", "CANCELLED", "EXPIRED", "DEAD_LETTER"}
NONTERMINAL = {"ENQUEUED", "LEASED", "RETRY_QUEUED", "CANCEL_REQUESTED"}


@dataclass(frozen=True)
class Envelope:
    message_id: str
    run_id: str
    sender: str
    receiver: str
    stream: str
    sequence: int
    payload_digest: str
    payload_size: int
    authority: frozenset[str]
    causal_parent: str | None
    deadline_tick: int


@dataclass
class Message:
    envelope: Envelope
    state: str = "ENQUEUED"
    attempts: int = 0
    cancel_requested: bool = False


@dataclass
class Mailbox:
    max_messages: int
    max_bytes: int
    state: str = "OPEN"
    queue: deque[str] = field(default_factory=deque)
    used_messages: int = 0
    used_bytes: int = 0


class SchedulerModel:
    def __init__(self, run_id: str = "run-1", max_attempts: int = 2):
        self.run_id = run_id
        self.max_attempts = max_attempts
        self.tick = 0
        self.ordinal = 0
        self.mailboxes: dict[str, Mailbox] = {}
        self.messages: dict[str, Message] = {}
        self.events: list[dict[str, Any]] = []
        self.next_sequence: dict[tuple[str, str, str], int] = {}
        self.accepted_sequences: dict[tuple[str, str, str], list[int]] = {}
        self.remote_next: dict[tuple[str, str, str], int] = {}
        self.remote_buffer: dict[tuple[str, str, str], dict[int, Envelope]] = {}
        self.fingerprints: dict[str, tuple[str, frozenset[str]]] = {}
        self.effects: dict[str, str] = {}
        self.wait_for: dict[str, str] = {}

    def register(self, actor: str, max_messages: int = 4, max_bytes: int = 64) -> None:
        if actor in self.mailboxes or max_messages <= 0 or max_bytes < 0:
            raise SchedulerError("InvalidMailbox")
        self.mailboxes[actor] = Mailbox(max_messages, max_bytes)
        self._event("ACTOR_REGISTERED", actor=actor)

    def _event(self, kind: str, caused_by: str | None = None, **fields: Any) -> None:
        self.ordinal += 1
        self.events.append({
            "event_id": f"{self.run_id}:{self.tick}:{self.ordinal}",
            "tick": self.tick,
            "kind": kind,
            "caused_by": caused_by,
            **fields,
        })

    def _message_id(self, sender: str, stream: str, sequence: int) -> str:
        return f"{self.run_id}:{sender}:{stream}:{sequence}"

    def send(
        self, sender: str, receiver: str, stream: str, payload: bytes,
        authority: frozenset[str], deadline_tick: int,
        current_authority: frozenset[str] | None = None,
        causal_parent: str | None = None,
    ) -> str | None:
        mailbox = self.mailboxes.get(receiver)
        if mailbox is None:
            self._event("SEND_REJECTED", sender=sender, receiver=receiver, reason="UnknownReceiver")
            return None
        if mailbox.state != "OPEN":
            self._event("SEND_REJECTED", sender=sender, receiver=receiver, reason="MailboxClosing")
            return None
        effective = authority if current_authority is None else current_authority
        if not authority.issubset(effective):
            self._event("SEND_REJECTED", sender=sender, receiver=receiver, reason="AuthorityEscalation")
            return None
        size = len(payload)
        if mailbox.used_messages + 1 > mailbox.max_messages or mailbox.used_bytes + size > mailbox.max_bytes:
            self._event("SEND_REJECTED", sender=sender, receiver=receiver, reason="Backpressure")
            return None
        if deadline_tick <= self.tick:
            self._event("SEND_REJECTED", sender=sender, receiver=receiver, reason="Expired")
            return None
        key = (sender, receiver, stream)
        sequence = self.next_sequence.get(key, 1)
        message_id = self._message_id(sender, stream, sequence)
        digest = hashlib.sha256(payload).hexdigest()
        envelope = Envelope(message_id, self.run_id, sender, receiver, stream, sequence, digest, size,
                            authority, causal_parent, deadline_tick)
        self.next_sequence[key] = sequence + 1
        self._admit(envelope)
        return message_id

    def _admit(self, envelope: Envelope) -> None:
        mailbox = self.mailboxes[envelope.receiver]
        if mailbox.used_messages + 1 > mailbox.max_messages or mailbox.used_bytes + envelope.payload_size > mailbox.max_bytes:
            raise SchedulerError("Backpressure")
        message = Message(envelope)
        self.messages[envelope.message_id] = message
        self.fingerprints[envelope.message_id] = (envelope.payload_digest, envelope.authority)
        mailbox.queue.append(envelope.message_id)
        mailbox.used_messages += 1
        mailbox.used_bytes += envelope.payload_size
        key = (envelope.sender, envelope.receiver, envelope.stream)
        self.accepted_sequences.setdefault(key, []).append(envelope.sequence)
        self._event("MESSAGE_ENQUEUED", caused_by=envelope.causal_parent,
                    message_id=envelope.message_id, receiver=envelope.receiver, sequence=envelope.sequence)

    def remote_receive(self, envelope: Envelope) -> str:
        fingerprint = (envelope.payload_digest, envelope.authority)
        existing = self.fingerprints.get(envelope.message_id)
        if existing is not None:
            if existing != fingerprint:
                self._event("MESSAGE_REJECTED", message_id=envelope.message_id, reason="MessageIdCollision")
                return "MessageIdCollision"
            self._event("DUPLICATE_DROPPED", message_id=envelope.message_id)
            return "Duplicate"
        key = (envelope.sender, envelope.receiver, envelope.stream)
        expected = self.remote_next.get(key, 1)
        if envelope.sequence < expected:
            self._event("DUPLICATE_DROPPED", message_id=envelope.message_id)
            return "Duplicate"
        buffer = self.remote_buffer.setdefault(key, {})
        if envelope.sequence in buffer:
            buffered = buffer[envelope.sequence]
            if (buffered.payload_digest, buffered.authority) != fingerprint:
                self._event("MESSAGE_REJECTED", message_id=envelope.message_id, reason="MessageIdCollision")
                return "MessageIdCollision"
            self._event("DUPLICATE_DROPPED", message_id=envelope.message_id)
            return "Duplicate"
        buffer[envelope.sequence] = envelope
        self._event("REMOTE_BUFFERED", message_id=envelope.message_id, sequence=envelope.sequence)
        while expected in buffer:
            candidate = buffer[expected]
            mailbox = self.mailboxes.get(candidate.receiver)
            if mailbox is None or mailbox.state != "OPEN":
                break
            if mailbox.used_messages + 1 > mailbox.max_messages or mailbox.used_bytes + candidate.payload_size > mailbox.max_bytes:
                self._event("REMOTE_BACKPRESSURE", message_id=candidate.message_id)
                break
            del buffer[expected]
            self._admit(candidate)
            expected += 1
            self.remote_next[key] = expected
        return "BufferedOrAdmitted"

    def lease(self, actor: str) -> str | None:
        mailbox = self.mailboxes[actor]
        while mailbox.queue:
            message_id = mailbox.queue.popleft()
            message = self.messages[message_id]
            if self.tick >= message.envelope.deadline_tick:
                self._terminal(message, "EXPIRED", "DEADLINE_EXPIRED")
                continue
            message.state = "LEASED"
            message.attempts += 1
            self._event("MESSAGE_LEASED", message_id=message_id, actor=actor, attempt=message.attempts)
            return message_id
        self._maybe_close(actor)
        return None

    def ack(self, message_id: str) -> None:
        message = self.messages[message_id]
        if message.state not in {"LEASED", "CANCEL_REQUESTED"}:
            raise SchedulerError("InvalidTransition")
        self._terminal(message, "ACKED", "MESSAGE_ACKED")

    def fail(self, message_id: str, retryable: bool = True) -> None:
        message = self.messages[message_id]
        if message.state not in {"LEASED", "CANCEL_REQUESTED"}:
            raise SchedulerError("InvalidTransition")
        if message.cancel_requested:
            self._terminal(message, "CANCELLED", "MESSAGE_CANCELLED")
        elif retryable and message.attempts < self.max_attempts and self.tick < message.envelope.deadline_tick:
            message.state = "RETRY_QUEUED"
            self.mailboxes[message.envelope.receiver].queue.append(message_id)
            self._event("MESSAGE_RETRY_QUEUED", message_id=message_id, attempt=message.attempts)
        else:
            self._terminal(message, "DEAD_LETTER", "MESSAGE_DEAD_LETTERED")

    def crash(self, actor: str) -> None:
        for message in self.messages.values():
            if message.envelope.receiver == actor and message.state in {"LEASED", "CANCEL_REQUESTED"}:
                if self.effects.get(message.envelope.message_id) == "DISPATCHED":
                    self.effects[message.envelope.message_id] = "UNCERTAIN"
                    self._event("EFFECT_UNCERTAIN", message_id=message.envelope.message_id)
                message.state = "RETRY_QUEUED"
                self.mailboxes[actor].queue.appendleft(message.envelope.message_id)
                self._event("LEASE_REVOKED", message_id=message.envelope.message_id, reason="Crash")

    def cancel(self, message_id: str) -> str:
        message = self.messages[message_id]
        if message.state in TERMINAL:
            self._event("CANCEL_NOOP", message_id=message_id, state=message.state)
            return message.state
        effect = self.effects.get(message_id)
        if effect == "DISPATCHED":
            self.effects[message_id] = "UNCERTAIN"
            message.cancel_requested = True
            message.state = "CANCEL_REQUESTED"
            self._event("EFFECT_UNCERTAIN", message_id=message_id, reason="CancelAfterDispatch")
            return "UNCERTAIN"
        if message.state in {"ENQUEUED", "RETRY_QUEUED"}:
            mailbox = self.mailboxes[message.envelope.receiver]
            mailbox.queue.remove(message_id)
            self._terminal(message, "CANCELLED", "MESSAGE_CANCELLED")
            return "CANCELLED"
        message.cancel_requested = True
        message.state = "CANCEL_REQUESTED"
        self._event("CANCEL_REQUESTED", message_id=message_id)
        return "CANCEL_REQUESTED"

    def dispatch_effect(self, message_id: str, effect: str) -> str:
        message = self.messages[message_id]
        if message.state != "LEASED" or message.cancel_requested:
            raise SchedulerError("CancelledBeforeEffect")
        if self.tick >= message.envelope.deadline_tick:
            raise SchedulerError("ExpiredBeforeEffect")
        if effect not in message.envelope.authority:
            raise SchedulerError("CapabilityMissing")
        self.effects[message_id] = "DISPATCHED"
        self._event("EFFECT_DISPATCHED", message_id=message_id, effect=effect)
        return "DISPATCHED"

    def complete_effect(self, message_id: str, success: bool) -> None:
        if self.effects.get(message_id) != "DISPATCHED":
            raise SchedulerError("InvalidTransition")
        self.effects[message_id] = "COMPLETED" if success else "FAILED"
        self._event("EFFECT_COMPLETED" if success else "EFFECT_FAILED", message_id=message_id)

    def _terminal(self, message: Message, state: str, event: str) -> None:
        if state not in TERMINAL:
            raise SchedulerError("InvalidTransition")
        mailbox = self.mailboxes[message.envelope.receiver]
        message.state = state
        mailbox.used_messages -= 1
        mailbox.used_bytes -= message.envelope.payload_size
        self._event(event, message_id=message.envelope.message_id, state=state)
        self._maybe_close(message.envelope.receiver)

    def drain(self, actor: str) -> None:
        mailbox = self.mailboxes[actor]
        if mailbox.state == "CLOSED":
            return
        mailbox.state = "DRAINING"
        self._event("MAILBOX_DRAINING", actor=actor)
        self._maybe_close(actor)

    def _maybe_close(self, actor: str) -> None:
        mailbox = self.mailboxes[actor]
        if mailbox.state == "DRAINING" and mailbox.used_messages == 0:
            mailbox.state = "CLOSED"
            self._event("MAILBOX_CLOSED", actor=actor)

    def await_actor(self, actor: str, target: str) -> str:
        self.wait_for[actor] = target
        seen: set[str] = set()
        current = actor
        while current in self.wait_for:
            if current in seen:
                del self.wait_for[actor]
                self._event("DEADLOCK_DETECTED", actor=actor, target=target)
                return "DeadlockDetected"
            seen.add(current)
            current = self.wait_for[current]
        self._event("ACTOR_WAITING", actor=actor, target=target)
        return "Waiting"

    def advance(self, ticks: int = 1) -> None:
        if ticks < 0:
            raise SchedulerError("InvalidTick")
        self.tick += ticks
        self._event("TICK_ADVANCED", ticks=ticks)

    def semantic_digest(self) -> str:
        projection = [{key: value for key, value in event.items() if key != "event_id"} for event in self.events]
        return hashlib.sha256(json.dumps(projection, sort_keys=True).encode()).hexdigest()

    def assert_invariants(self) -> None:
        for actor, mailbox in self.mailboxes.items():
            nonterminal = [m for m in self.messages.values()
                           if m.envelope.receiver == actor and m.state in NONTERMINAL]
            if mailbox.used_messages != len(nonterminal):
                raise AssertionError("no_silent_loss")
            if mailbox.used_bytes != sum(m.envelope.payload_size for m in nonterminal):
                raise AssertionError("queue_accounting")
            if mailbox.used_messages > mailbox.max_messages or mailbox.used_bytes > mailbox.max_bytes:
                raise AssertionError("queue_bounds")
            queued = [m.envelope.message_id for m in nonterminal if m.state in {"ENQUEUED", "RETRY_QUEUED"}]
            if sorted(queued) != sorted(mailbox.queue):
                raise AssertionError("no_silent_loss")
            if mailbox.state == "CLOSED" and nonterminal:
                raise AssertionError("closed_with_messages")
        for key, sequences in self.accepted_sequences.items():
            if sequences != sorted(sequences) or len(sequences) != len(set(sequences)):
                raise AssertionError(f"fifo_per_stream:{key}")
        ack_counts: dict[str, int] = {}
        for event in self.events:
            if event["kind"] == "MESSAGE_ACKED":
                mid = event["message_id"]
                ack_counts[mid] = ack_counts.get(mid, 0) + 1
        if any(count > 1 for count in ack_counts.values()):
            raise AssertionError("dedup_single_ack")
