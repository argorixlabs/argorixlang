use argorix_conformance::core_memory::{
    build_complete_tree, tree_sum, validate_abi_policy, AbiPolicy, Arena, ArenaBuffer, Capability,
    Handle, HostGate, HostOperation, HostProfile, MemoryError, Permission, HANDLE_SIZE, TYPE_BYTE,
};

const OWNER: u64 = 7;

fn arena(limit: u64) -> Arena {
    Arena::new(11, OWNER, limit, 1024, 64).unwrap()
}

#[test]
fn canonical_handle_roundtrips_and_rejects_reserved_bytes() {
    let mut arena = arena(128);
    let handle = arena.alloc(OWNER, TYPE_BYTE, 8, 1, true).unwrap();
    assert_eq!(Handle::decode(&handle.encode()).unwrap(), handle);
    let mut malformed = handle.encode();
    malformed[47] = 1;
    assert_eq!(Handle::decode(&malformed), Err(MemoryError::InvalidHandle));
    assert_eq!(HANDLE_SIZE, 48);
}

#[test]
fn freed_and_released_handles_never_revive() {
    let mut arena = arena(128);
    let old = arena.alloc(OWNER, TYPE_BYTE, 4, 1, true).unwrap();
    arena.free(OWNER, old).unwrap();
    assert_eq!(arena.read_element(old, 0), Err(MemoryError::UseAfterFree));
    assert_eq!(arena.free(OWNER, old), Err(MemoryError::DoubleFree));
    let reused = arena.alloc(OWNER, TYPE_BYTE, 4, 1, true).unwrap();
    assert_eq!(old.slot, reused.slot);
    assert_ne!(old.allocation_generation, reused.allocation_generation);
    assert_eq!(arena.read_element(old, 0), Err(MemoryError::UseAfterFree));
    arena.release(OWNER).unwrap();
    assert_eq!(
        arena.read_element(reused, 0),
        Err(MemoryError::ArenaReleased)
    );
}

#[test]
fn bounds_permissions_borrows_and_ownership_fail_atomically() {
    let mut arena = arena(128);
    let handle = arena.alloc(OWNER, TYPE_BYTE, 8, 1, true).unwrap();
    let read_only = arena.slice(handle, 2, 3, Permission::Read).unwrap();
    let before = arena.clone();
    assert_eq!(
        arena.read_element(read_only, 3),
        Err(MemoryError::OutOfBounds)
    );
    assert_eq!(arena, before);
    assert_eq!(
        arena.write_element(read_only, 0, &[9]),
        Err(MemoryError::PermissionDenied)
    );
    assert_eq!(arena, before);

    let read = arena.borrow_read(handle).unwrap();
    let borrowed = arena.clone();
    assert_eq!(arena.borrow_write(handle), Err(MemoryError::BorrowConflict));
    assert_eq!(arena, borrowed);
    assert_eq!(arena.transfer(OWNER, 9), Err(MemoryError::BorrowConflict));
    assert_eq!(arena, borrowed);
    arena.end_borrow(read).unwrap();
    arena.transfer(OWNER, 9).unwrap();
    assert_eq!(arena.owner(), 9);
    assert_eq!(arena.free(OWNER, handle), Err(MemoryError::NotOwner));
}

#[test]
fn oom_overflow_and_limits_do_not_mutate_state() {
    let mut arena = arena(8);
    let _ = arena.alloc(OWNER, TYPE_BYTE, 8, 1, true).unwrap();
    let before = arena.clone();
    assert_eq!(
        arena.alloc(OWNER, TYPE_BYTE, 1, 1, true),
        Err(MemoryError::OutOfMemory)
    );
    assert_eq!(arena, before);
    assert_eq!(
        arena.alloc(OWNER, TYPE_BYTE, u64::MAX, 2, true),
        Err(MemoryError::Overflow)
    );
    assert_eq!(arena, before);
}

#[test]
fn recursive_tree_and_growable_buffer_stay_within_measured_limits() {
    let mut tree_arena = Arena::new(21, OWNER, 16 * 1024, 128, 64).unwrap();
    let root = build_complete_tree(&mut tree_arena, OWNER, 6, 8).unwrap();
    assert_eq!(tree_sum(&tree_arena, root, 8).unwrap(), 2016);
    assert_eq!(tree_arena.used_bytes(), 63 * 120);
    let before = tree_arena.clone();
    assert_eq!(
        build_complete_tree(&mut tree_arena, OWNER, 9, 8),
        Err(MemoryError::DepthLimit)
    );
    assert_eq!(tree_arena, before);

    let mut buffer_arena = Arena::new(22, OWNER, 512, 32, 64).unwrap();
    let mut buffer = ArenaBuffer::new();
    for value in 0_u8..100 {
        buffer.push(&mut buffer_arena, OWNER, value).unwrap();
    }
    assert_eq!(buffer.len(), 100);
    assert_eq!(buffer.capacity(), 128);
    assert_eq!(
        buffer.values(&buffer_arena).unwrap(),
        (0_u8..100).collect::<Vec<_>>()
    );
    assert!(buffer_arena.peak_used_bytes() <= 192);
}

#[test]
fn buffer_growth_oom_is_transactional() {
    let mut arena = Arena::new(23, OWNER, 8, 8, 8).unwrap();
    let mut buffer = ArenaBuffer::new();
    for value in 0_u8..4 {
        buffer.push(&mut arena, OWNER, value).unwrap();
    }
    let before_arena = arena.clone();
    let before_buffer = buffer.clone();
    assert_eq!(
        buffer.push(&mut arena, OWNER, 4),
        Err(MemoryError::OutOfMemory)
    );
    assert_eq!(arena, before_arena);
    assert_eq!(buffer, before_buffer);
}

#[test]
fn memory_is_checked_before_host_authority_and_profiles_do_not_mix() {
    let mut arena = arena(64);
    let input = arena.alloc(OWNER, TYPE_BYTE, 1, 1, true).unwrap();
    let invalid = Handle { length: 2, ..input };
    let mut gate = HostGate::default();
    let mut compiler_cap = Capability {
        profile: HostProfile::Compiler,
        operation: HostOperation::PackageRead,
        budget: 1,
        expires_at: 10,
    };
    assert_eq!(
        gate.authorize(
            &arena,
            invalid,
            HostProfile::Compiler,
            HostOperation::PackageRead,
            Some(&mut compiler_cap),
            1,
        ),
        Err(MemoryError::OutOfBounds)
    );
    assert_eq!(gate.dispatched, 0);
    assert_eq!(compiler_cap.budget, 1);
    assert_eq!(
        gate.authorize(
            &arena,
            input,
            HostProfile::Agent,
            HostOperation::PackageRead,
            Some(&mut compiler_cap),
            1,
        ),
        Err(MemoryError::OperationDenied)
    );
    assert_eq!(
        gate.authorize(
            &arena,
            input,
            HostProfile::Compiler,
            HostOperation::ArbitraryFfi,
            Some(&mut compiler_cap),
            1,
        ),
        Err(MemoryError::OperationDenied)
    );
    gate.authorize(
        &arena,
        input,
        HostProfile::Compiler,
        HostOperation::PackageRead,
        Some(&mut compiler_cap),
        1,
    )
    .unwrap();
    assert_eq!(gate.dispatched, 1);
    assert_eq!(compiler_cap.budget, 0);
}

#[test]
fn abi_policy_matches_the_rust_prototype() {
    let policy: AbiPolicy =
        serde_json::from_str(include_str!("../../../spec/core/memory-abi.json")).unwrap();
    assert_eq!(policy.schema_version, 1);
    assert_eq!(policy.abi_version, 1);
    assert_eq!(policy.endianness, "little");
    assert_eq!(policy.handle_size, HANDLE_SIZE);
    assert_eq!(policy.result_size, 56);
    assert_eq!(policy.host_call_size, 120);
    assert!(validate_abi_policy(&policy).is_empty());
}

#[test]
fn abi_layout_mutation_is_detected() {
    let mut policy: AbiPolicy =
        serde_json::from_str(include_str!("../../../spec/core/memory-abi.json")).unwrap();
    policy.handle_fields[0].offset = 1;
    assert!(validate_abi_policy(&policy)
        .iter()
        .any(|failure| failure.contains("CoreHandleV1")));
}
