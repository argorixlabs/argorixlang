"""Executable reduced model for MAT-004. It is not the Argorix runtime."""

from __future__ import annotations

from dataclasses import asdict, dataclass, field
import hashlib
import json
from typing import Any


SIZES = {"bool": 1, "u8": 1, "i64": 8, "byte": 1}
MAX_COUNT = (1 << 63) - 1


class ModelError(Exception):
    def __init__(self, code: str):
        super().__init__(code)
        self.code = code


@dataclass(frozen=True)
class Handle:
    arena_id: int
    arena_epoch: int
    slot: int
    generation: int
    offset: int
    length: int
    permission: str


@dataclass(frozen=True)
class BorrowToken:
    token_id: int
    handle: Handle
    kind: str
    borrower: str


@dataclass
class Allocation:
    generation: int
    element_type: str
    values: list[Any]
    mutable: bool
    live: bool = True


@dataclass
class Arena:
    arena_id: int
    epoch: int
    owner: str
    byte_limit: int
    used_bytes: int = 0
    active: bool = True
    next_slot: int = 0
    allocations: dict[int, Allocation] = field(default_factory=dict)


@dataclass(frozen=True)
class Capability:
    capability_id: str
    kind: str
    scope: str
    subject: str
    valid: bool = True


class MemoryModel:
    def __init__(self, host_limit: int = 1 << 20, max_arenas: int = 16):
        self.host_limit = host_limit
        self.max_arenas = max_arenas
        self.next_arena_id = 1
        self.next_token_id = 1
        self.arenas: dict[int, Arena] = {}
        self.borrows: dict[int, BorrowToken] = {}
        self.effect_events: list[dict[str, str]] = []

    def snapshot(self) -> str:
        payload = {
            "next_arena_id": self.next_arena_id,
            "next_token_id": self.next_token_id,
            "arenas": {key: asdict(value) for key, value in sorted(self.arenas.items())},
            "borrows": {key: asdict(value) for key, value in sorted(self.borrows.items())},
            "effect_events": self.effect_events,
        }
        return hashlib.sha256(json.dumps(payload, sort_keys=True).encode()).hexdigest()

    def arena_create(self, owner: str, byte_limit: int) -> int:
        if not owner:
            raise ModelError("InvalidOwner")
        if byte_limit < 0 or byte_limit > self.host_limit:
            raise ModelError("InvalidLimit")
        if sum(1 for arena in self.arenas.values() if arena.active) >= self.max_arenas:
            raise ModelError("ResourceLimit")
        arena_id = self.next_arena_id
        self.next_arena_id += 1
        self.arenas[arena_id] = Arena(arena_id, 1, owner, byte_limit)
        return arena_id

    def _arena(self, arena_id: int, owner: str | None = None) -> Arena:
        arena = self.arenas.get(arena_id)
        if arena is None:
            raise ModelError("InvalidHandle")
        if not arena.active:
            raise ModelError("ArenaReleased")
        if owner is not None and arena.owner != owner:
            raise ModelError("NotOwner")
        return arena

    def alloc(self, owner: str, arena_id: int, element_type: str, length: int, mutable: bool = True) -> Handle:
        arena = self._arena(arena_id, owner)
        if element_type not in SIZES:
            raise ModelError("TypeMismatch")
        if length < 0 or length > MAX_COUNT // SIZES[element_type]:
            raise ModelError("Overflow")
        required = length * SIZES[element_type]
        if required > arena.byte_limit - arena.used_bytes:
            raise ModelError("OutOfMemory")
        slot = arena.next_slot
        arena.next_slot += 1
        zero: Any = False if element_type == "bool" else 0
        arena.allocations[slot] = Allocation(1, element_type, [zero for _ in range(length)], mutable)
        arena.used_bytes += required
        return Handle(arena_id, arena.epoch, slot, 1, 0, length, "read_write" if mutable else "read")

    def _validate(self, handle: Handle) -> tuple[Arena, Allocation]:
        arena = self.arenas.get(handle.arena_id)
        if arena is None:
            raise ModelError("InvalidHandle")
        if not arena.active or handle.arena_epoch != arena.epoch:
            raise ModelError("ArenaReleased")
        allocation = arena.allocations.get(handle.slot)
        if allocation is None:
            raise ModelError("InvalidHandle")
        if not allocation.live or handle.generation != allocation.generation:
            raise ModelError("UseAfterFree")
        if handle.offset < 0 or handle.length < 0 or handle.offset + handle.length > len(allocation.values):
            raise ModelError("InvalidHandle")
        if handle.permission not in {"read", "read_write"}:
            raise ModelError("PermissionDenied")
        if handle.permission == "read_write" and not allocation.mutable:
            raise ModelError("PermissionDenied")
        return arena, allocation

    def slice(self, handle: Handle, start: int, length: int, permission: str) -> Handle:
        self._validate(handle)
        if start < 0 or length < 0 or start + length > handle.length:
            raise ModelError("OutOfBounds")
        if permission not in {"read", "read_write"}:
            raise ModelError("PermissionDenied")
        if permission == "read_write" and handle.permission != "read_write":
            raise ModelError("PermissionDenied")
        return Handle(
            handle.arena_id, handle.arena_epoch, handle.slot, handle.generation,
            handle.offset + start, length, permission
        )

    def _allocation_borrows(self, handle: Handle) -> list[BorrowToken]:
        return [token for token in self.borrows.values()
                if token.handle.arena_id == handle.arena_id and token.handle.slot == handle.slot]

    def borrow_read(self, handle: Handle, borrower: str) -> BorrowToken:
        self._validate(handle)
        if any(token.kind == "write" for token in self._allocation_borrows(handle)):
            raise ModelError("BorrowConflict")
        return self._new_token(handle, "read", borrower)

    def borrow_write(self, handle: Handle, borrower: str) -> BorrowToken:
        self._validate(handle)
        if handle.permission != "read_write":
            raise ModelError("PermissionDenied")
        if self._allocation_borrows(handle):
            raise ModelError("BorrowConflict")
        return self._new_token(handle, "write", borrower)

    def _new_token(self, handle: Handle, kind: str, borrower: str) -> BorrowToken:
        if not borrower:
            raise ModelError("InvalidOwner")
        token = BorrowToken(self.next_token_id, handle, kind, borrower)
        self.next_token_id += 1
        self.borrows[token.token_id] = token
        return token

    def end_borrow(self, token: BorrowToken) -> None:
        if self.borrows.get(token.token_id) != token:
            raise ModelError("InvalidBorrow")
        del self.borrows[token.token_id]

    def read(self, handle: Handle, index: int, token: BorrowToken | None = None) -> Any:
        _, allocation = self._validate(handle)
        if index < 0 or index >= handle.length:
            raise ModelError("OutOfBounds")
        active = self._allocation_borrows(handle)
        if token is None and any(item.kind == "write" for item in active):
            raise ModelError("BorrowConflict")
        if token is not None and self.borrows.get(token.token_id) != token:
            raise ModelError("InvalidBorrow")
        return allocation.values[handle.offset + index]

    def write(self, handle: Handle, index: int, value: Any, token: BorrowToken | None = None) -> None:
        _, allocation = self._validate(handle)
        if handle.permission != "read_write":
            raise ModelError("PermissionDenied")
        if index < 0 or index >= handle.length:
            raise ModelError("OutOfBounds")
        if not self._value_matches(allocation.element_type, value):
            raise ModelError("TypeMismatch")
        active = self._allocation_borrows(handle)
        if token is None and active:
            raise ModelError("BorrowConflict")
        if token is not None:
            if self.borrows.get(token.token_id) != token:
                raise ModelError("InvalidBorrow")
            if token.kind != "write" or token.handle != handle:
                raise ModelError("PermissionDenied")
        allocation.values[handle.offset + index] = value

    @staticmethod
    def _value_matches(element_type: str, value: Any) -> bool:
        if element_type == "bool":
            return isinstance(value, bool)
        if not isinstance(value, int) or isinstance(value, bool):
            return False
        if element_type in {"u8", "byte"}:
            return 0 <= value <= 255
        return -(1 << 63) <= value < (1 << 63)

    def free(self, owner: str, handle: Handle) -> None:
        arena, allocation = self._validate(handle)
        if arena.owner != owner:
            raise ModelError("NotOwner")
        if handle.offset != 0 or handle.length != len(allocation.values):
            raise ModelError("NotRoot")
        if self._allocation_borrows(handle):
            raise ModelError("BorrowConflict")
        arena.used_bytes -= len(allocation.values) * SIZES[allocation.element_type]
        allocation.live = False
        allocation.generation += 1

    def transfer(self, old_owner: str, new_owner: str, arena_id: int) -> None:
        arena = self._arena(arena_id, old_owner)
        if not new_owner:
            raise ModelError("InvalidOwner")
        if any(token.handle.arena_id == arena_id for token in self.borrows.values()):
            raise ModelError("BorrowConflict")
        arena.owner = new_owner

    def arena_release(self, owner: str, arena_id: int) -> None:
        arena = self._arena(arena_id, owner)
        if any(token.handle.arena_id == arena_id for token in self.borrows.values()):
            raise ModelError("BorrowConflict")
        arena.active = False
        arena.epoch += 1
        arena.used_bytes = 0
        for allocation in arena.allocations.values():
            allocation.live = False
            allocation.generation += 1

    def authorize_effect(
        self, declared: set[str], capability: Capability | None,
        subject: str, effect: str, target: str
    ) -> str:
        before = len(self.effect_events)
        if effect not in declared:
            raise ModelError("EffectNotDeclared")
        if capability is None or not capability.valid or capability.subject != subject or capability.kind != effect:
            raise ModelError("CapabilityMissing")
        if capability.scope != target:
            raise ModelError("ScopeDenied")
        self.effect_events.append({"subject": subject, "effect": effect, "target": target, "state": "dispatched"})
        assert len(self.effect_events) == before + 1
        return "ALLOW"

    def assert_invariants(self) -> None:
        for arena in self.arenas.values():
            expected = 0
            for allocation in arena.allocations.values():
                if allocation.live:
                    expected += len(allocation.values) * SIZES[allocation.element_type]
            if arena.active and expected != arena.used_bytes:
                raise AssertionError("accounting")
            if arena.used_bytes < 0 or arena.used_bytes > arena.byte_limit:
                raise AssertionError("accounting")
            for slot in arena.allocations:
                tokens = [token for token in self.borrows.values()
                          if token.handle.arena_id == arena.arena_id and token.handle.slot == slot]
                if sum(token.kind == "write" for token in tokens) > 1:
                    raise AssertionError("borrow_exclusion")
                if any(token.kind == "write" for token in tokens) and len(tokens) != 1:
                    raise AssertionError("borrow_exclusion")
                if not arena.active and tokens:
                    raise AssertionError("released_arena_has_borrow")
