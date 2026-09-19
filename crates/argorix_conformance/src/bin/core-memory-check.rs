use anyhow::{bail, Context, Result};
use argorix_conformance::core_memory::{
    build_complete_tree, tree_sum, validate_abi_policy, AbiPolicy, Arena, ArenaBuffer, Capability,
    Handle, HostGate, HostOperation, HostProfile, MemoryError, TYPE_BYTE,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{env, fs, hint::black_box, path::PathBuf, time::Instant};

#[derive(Serialize)]
struct CheckResult {
    id: &'static str,
    expected: &'static str,
    observed: String,
    failure_atomic: bool,
    passed: bool,
}

#[derive(Serialize)]
struct Measurements {
    configured_arena_ceiling_bytes: u64,
    tree_depth: u32,
    tree_nodes: u64,
    tree_used_bytes: u64,
    tree_peak_bytes: u64,
    buffer_elements: u64,
    buffer_capacity: u64,
    buffer_peak_bytes: u64,
    checked_reads: u64,
    checked_read_elapsed_ns: u128,
    indicative_ns_per_checked_read: f64,
}

#[derive(Serialize)]
struct Report {
    schema_version: u32,
    task: &'static str,
    baseline_commit: String,
    scope: &'static str,
    abi_policy_sha256: String,
    checks: Vec<CheckResult>,
    handle_roundtrip: bool,
    abi_contract_passed: bool,
    abi_contract_failures: Vec<String>,
    abi_mutation_detected: bool,
    forbidden_ffi_denied: bool,
    memory_before_effect: bool,
    unsafe_blocks_in_prototype: usize,
    measurements: Measurements,
    overall_pass: bool,
    not_proven: Vec<&'static str>,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn check(
    id: &'static str,
    expected: &'static str,
    observed: MemoryError,
    atomic: bool,
) -> CheckResult {
    CheckResult {
        id,
        expected,
        observed: observed.to_string(),
        failure_atomic: atomic,
        passed: observed.to_string() == expected && atomic,
    }
}

fn main() -> Result<()> {
    let mut args = env::args_os().skip(1);
    let policy_path = PathBuf::from(
        args.next()
            .context("usage: core-memory-check ABI_JSON OUTPUT")?,
    );
    let output = PathBuf::from(
        args.next()
            .context("usage: core-memory-check ABI_JSON OUTPUT")?,
    );
    if args.next().is_some() {
        bail!("usage: core-memory-check ABI_JSON OUTPUT");
    }
    let policy_raw = fs::read(&policy_path)?;
    let policy: AbiPolicy = serde_json::from_slice(&policy_raw)?;
    let abi_contract_failures = validate_abi_policy(&policy);
    let abi_contract_passed = abi_contract_failures.is_empty();
    let mut mutated_policy = policy.clone();
    mutated_policy.handle_fields[0].offset = 1;
    let abi_mutation_detected = !validate_abi_policy(&mutated_policy).is_empty();

    let owner = 7;
    let ceiling = 1_048_576;
    let mut arena = Arena::new(100, owner, ceiling, 10_000, 256)?;
    let handle = arena.alloc(owner, TYPE_BYTE, 16, 1, true)?;
    let handle_roundtrip = Handle::decode(&handle.encode())? == handle;
    let mut checks = Vec::new();

    let before = arena.clone();
    let error = arena.read_element(handle, 16).unwrap_err();
    checks.push(check(
        "out_of_bounds",
        "OutOfBounds",
        error,
        arena == before,
    ));

    let token = arena.borrow_read(handle)?;
    let before = arena.clone();
    let error = arena.borrow_write(handle).unwrap_err();
    checks.push(check(
        "borrow_conflict",
        "BorrowConflict",
        error,
        arena == before,
    ));
    arena.end_borrow(token)?;

    let freed = arena.alloc(owner, TYPE_BYTE, 8, 1, true)?;
    arena.free(owner, freed)?;
    let before = arena.clone();
    let error = arena.read_element(freed, 0).unwrap_err();
    checks.push(check(
        "use_after_free",
        "UseAfterFree",
        error,
        arena == before,
    ));

    let mut tiny = Arena::new(101, owner, 4, 8, 8)?;
    let _ = tiny.alloc(owner, TYPE_BYTE, 4, 1, true)?;
    let before = tiny.clone();
    let error = tiny.alloc(owner, TYPE_BYTE, 1, 1, true).unwrap_err();
    checks.push(check("oom", "OutOfMemory", error, tiny == before));

    let before = arena.clone();
    let error = build_complete_tree(&mut arena, owner, 9, 8).unwrap_err();
    checks.push(check("depth_limit", "DepthLimit", error, arena == before));

    let released = arena.alloc(owner, TYPE_BYTE, 1, 1, true)?;
    arena.release(owner)?;
    let before = arena.clone();
    let error = arena.read_element(released, 0).unwrap_err();
    checks.push(check(
        "arena_released",
        "ArenaReleased",
        error,
        arena == before,
    ));

    let mut tree_arena = Arena::new(102, owner, ceiling, 10_000, 256)?;
    let root = build_complete_tree(&mut tree_arena, owner, 8, 8)?;
    if tree_sum(&tree_arena, root, 8)? != 32_640 {
        bail!("tree oracle failed");
    }
    let mut buffer_arena = Arena::new(103, owner, ceiling, 128, 256)?;
    let mut buffer = ArenaBuffer::new();
    for index in 0_u64..4096 {
        buffer.push(&mut buffer_arena, owner, (index % 251) as u8)?;
    }
    if buffer.values(&buffer_arena)?.len() != 4096 {
        bail!("buffer oracle failed");
    }

    let mut benchmark_arena = Arena::new(104, owner, 64, 8, 8)?;
    let benchmark_handle = benchmark_arena.alloc(owner, TYPE_BYTE, 1, 1, true)?;
    benchmark_arena.write_element(benchmark_handle, 0, &[42])?;
    let checked_reads = 100_000_u64;
    let started = Instant::now();
    for _ in 0..checked_reads {
        black_box(benchmark_arena.read_element(benchmark_handle, 0)?);
    }
    let elapsed = started.elapsed().as_nanos();

    let mut gate = HostGate::default();
    let mut cap = Capability {
        profile: HostProfile::Compiler,
        operation: HostOperation::PackageRead,
        budget: 1,
        expires_at: 10,
    };
    let forbidden_ffi_denied = gate.authorize(
        &benchmark_arena,
        benchmark_handle,
        HostProfile::Compiler,
        HostOperation::ArbitraryFfi,
        Some(&mut cap),
        1,
    ) == Err(MemoryError::OperationDenied)
        && gate.dispatched == 0;
    let invalid = Handle {
        length: 2,
        ..benchmark_handle
    };
    let memory_before_effect = gate.authorize(
        &benchmark_arena,
        invalid,
        HostProfile::Compiler,
        HostOperation::PackageRead,
        Some(&mut cap),
        1,
    ) == Err(MemoryError::OutOfBounds)
        && gate.dispatched == 0
        && cap.budget == 1;
    let prototype_source = include_str!("../core_memory.rs");
    let unsafe_blocks = prototype_source.matches("unsafe {").count();
    let checks_pass = checks.iter().all(|item| item.passed);
    let overall_pass = checks_pass
        && handle_roundtrip
        && abi_contract_passed
        && abi_mutation_detected
        && forbidden_ffi_denied
        && memory_before_effect
        && unsafe_blocks == 0;
    let report = Report {
        schema_version: 1,
        task: "ESP-005",
        baseline_commit: option_env!("GIT_COMMIT").unwrap_or("UNSPECIFIED").into(),
        scope: "Safe Rust sequential prototype and canonical ABI vectors; not stage0/C/native runtime or OS sandbox",
        abi_policy_sha256: digest(&policy_raw),
        checks,
        handle_roundtrip,
        abi_contract_passed,
        abi_contract_failures,
        abi_mutation_detected,
        forbidden_ffi_denied,
        memory_before_effect,
        unsafe_blocks_in_prototype: unsafe_blocks,
        measurements: Measurements {
            configured_arena_ceiling_bytes: ceiling,
            tree_depth: 8,
            tree_nodes: 255,
            tree_used_bytes: tree_arena.used_bytes(),
            tree_peak_bytes: tree_arena.peak_used_bytes(),
            buffer_elements: buffer.len(),
            buffer_capacity: buffer.capacity(),
            buffer_peak_bytes: buffer_arena.peak_used_bytes(),
            checked_reads,
            checked_read_elapsed_ns: elapsed,
            indicative_ns_per_checked_read: elapsed as f64 / checked_reads as f64,
        },
        overall_pass,
        not_proven: vec![
            "Core syntax compiles",
            "C backend equivalence",
            "native ABI layout",
            "thread safety",
            "OS sandbox enforcement",
            "production performance",
            "Rust independence",
        ],
    };
    fs::write(
        &output,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;
    println!(
        "ESP-005: checks={}/{}; tree={} bytes; buffer_peak={} bytes; {:.1} ns/read; overall_pass={}",
        report.checks.iter().filter(|item| item.passed).count(),
        report.checks.len(),
        report.measurements.tree_peak_bytes,
        report.measurements.buffer_peak_bytes,
        report.measurements.indicative_ns_per_checked_read,
        report.overall_pass
    );
    if !overall_pass {
        bail!("ESP-005 prototype validation failed");
    }
    Ok(())
}
